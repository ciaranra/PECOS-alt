//! Message builder for creating byte-encoded messages
//!
//! This module provides utilities for constructing binary messages
//! according to the byte protocol.

use super::protocol::{
    BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageFlags, MessageHeader,
    MessageType, QuantumGateHeader, calc_padding,
};
use crate::channels::ByteMessage;
use crate::channels::byte::gate_type::{GateTypeId, QuantumGate};
use bytemuck::bytes_of;
use pecos_core::types::QuantumCommand;
use std::mem::size_of;

/// Enum to track what kind of message is being built
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BuilderMode {
    Empty,              // No operations added yet
    QuantumOperations,  // Contains quantum operations
    MeasurementResults, // Contains measurement results
    ControlMessage,     // Contains control messages like Flush
}

/// Helper for building binary messages
///
/// The builder maintains internal state tracking what kind of message is being created
/// and ensures that different message types are not mixed inappropriately.
pub struct MessageBuilder {
    buffer: Vec<u8>,
    msg_count: u32,
    mode: BuilderMode,
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MessageBuilder {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            msg_count: self.msg_count,
            mode: self.mode,
        }
    }
}

impl MessageBuilder {
    /// Create a new message builder
    #[must_use]
    pub fn new() -> Self {
        let mut buffer = Vec::with_capacity(512); // Pre-allocate reasonable buffer
        // Reserve space for batch header, will fill later
        buffer.resize(size_of::<BatchHeader>(), 0);

        Self {
            buffer,
            msg_count: 0,
            mode: BuilderMode::Empty,
        }
    }

    /// Create a builder pre-configured for quantum operations
    #[must_use]
    pub fn for_quantum_operations(&mut self) -> &mut Self {
        self.mode = BuilderMode::QuantumOperations;
        self.add_message(MessageType::BeginBatch, &[], MessageFlags::NONE);
        self
    }

    /// Create a builder pre-configured for measurement results
    #[must_use]
    pub fn for_measurement_results(&mut self) -> &mut Self {
        self.mode = BuilderMode::MeasurementResults;
        self.add_message(MessageType::BeginBatch, &[], MessageFlags::NONE);
        self
    }

    /// Add padding bytes to ensure alignment
    fn add_padding(&mut self, alignment: usize) {
        let padding = calc_padding(self.buffer.len(), alignment);
        if padding > 0 {
            self.buffer.resize(self.buffer.len() + padding, 0);
        }
    }

    /// Add a message with a header and payload
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - Attempting to mix quantum operations and measurement results in the same message
    /// - Attempting to mix control messages with other message types
    pub fn add_message(
        &mut self,
        msg_type: MessageType,
        payload: &[u8],
        flags: MessageFlags,
    ) -> &mut Self {
        // Update mode based on message type
        match msg_type {
            MessageType::BeginBatch | MessageType::EndBatch => {
                // These can be used with any mode
            }
            MessageType::QuantumGate | MessageType::Measurement => {
                assert!(
                    !(self.mode == BuilderMode::MeasurementResults),
                    "Cannot mix quantum operations and measurement results in the same message"
                );
                if self.mode == BuilderMode::Empty {
                    self.mode = BuilderMode::QuantumOperations;
                }
            }
            MessageType::MeasurementResult => {
                assert!(
                    !(self.mode == BuilderMode::QuantumOperations),
                    "Cannot mix quantum operations and measurement results in the same message"
                );
                self.mode = BuilderMode::MeasurementResults;
            }
            MessageType::Flush | MessageType::Reset | MessageType::Error => {
                assert!(
                    !(self.mode != BuilderMode::Empty && self.mode != BuilderMode::ControlMessage),
                    "Control messages should be sent separately from other message types"
                );
                self.mode = BuilderMode::ControlMessage;
            }
        }

        // Ensure 4-byte alignment for message header
        self.add_padding(4);

        // Create and write message header
        let header = MessageHeader::new(
            msg_type,
            u32::try_from(payload.len()).unwrap_or(u32::MAX),
            flags,
        );
        self.buffer.extend_from_slice(bytes_of(&header));

        // Write payload
        self.buffer.extend_from_slice(payload);

        self.msg_count += 1;
        self
    }

    /// Add a quantum gate command
    fn add_quantum_gate(&mut self, gate: &QuantumGate) -> &mut Self {
        // Handle measurement gates using the add_measurements method
        if gate.gate_type == GateTypeId::Measure {
            if let Some(result_id) = gate.result_id {
                return self.add_measurements(&gate.qubits, &[result_id]);
            }
            return self;
        }

        // Calculate total payload size
        let header_size = size_of::<QuantumGateHeader>();
        let qubits_size = gate.qubits.len() * size_of::<u32>();
        let params_size = match gate.gate_type {
            GateTypeId::RZ => size_of::<f64>(),
            GateTypeId::R1XY => 2 * size_of::<f64>(),
            _ => 0,
        };
        let total_size = header_size + qubits_size + params_size;

        // Create a buffer for the payload
        let mut payload = Vec::with_capacity(total_size);

        // Determine gate type and parameters
        let has_params = !gate.params.is_empty();

        // Create gate header
        let header = QuantumGateHeader {
            gate_type: gate.gate_type as u8,
            num_qubits: u8::try_from(gate.qubits.len()).expect("Too many qubits for gate"),
            has_params: u8::from(has_params),
            reserved: 0,
        };

        // Add header to payload
        payload.extend_from_slice(bytes_of(&header));

        // Add qubit indices to payload
        for qubit in &gate.qubits {
            let qubit_u32 = u32::try_from(*qubit).expect("Qubit index too large");
            payload.extend_from_slice(&qubit_u32.to_le_bytes());
        }

        // Add parameters to payload if needed
        if has_params {
            match gate.gate_type {
                GateTypeId::RZ => {
                    payload.extend_from_slice(&gate.params[0].to_le_bytes());
                }
                GateTypeId::R1XY => {
                    payload.extend_from_slice(&gate.params[0].to_le_bytes()); // phi
                    payload.extend_from_slice(&gate.params[1].to_le_bytes()); // theta
                }
                // Other gate types don't have parameters
                _ => {}
            }
        }

        // Add the message to the buffer
        self.add_message(MessageType::QuantumGate, &payload, MessageFlags::NONE);
        self
    }

    /// Add multiple quantum gates at once
    pub fn add_quantum_gates(&mut self, gates: &[QuantumGate]) -> &mut Self {
        for gate in gates {
            self.add_quantum_gate(gate);
        }
        self
    }

    /// Add multiple measurement results at once
    ///
    /// # Panics
    ///
    /// This function will panic if the outcomes and `result_ids` arrays do not have the same length.
    pub fn add_measurement_results(
        &mut self,
        results: &[usize],
        result_ids: &[usize],
    ) -> &mut Self {
        assert_eq!(
            results.len(),
            result_ids.len(),
            "Outcomes and result_ids arrays must have the same length"
        );

        for (i, (&result, &result_id)) in results.iter().zip(result_ids.iter()).enumerate() {
            let is_last = i == results.len() - 1;
            let flags = if is_last {
                MessageFlags::LAST_MESSAGE
            } else {
                MessageFlags::NONE
            };

            let result_header = MeasurementResultHeader {
                result_id: u32::try_from(result_id).expect("Result ID too large"),
                outcome: u32::try_from(result).expect("Result outcome too large"),
            };

            self.add_message(
                MessageType::MeasurementResult,
                bytes_of(&result_header),
                flags,
            );
        }
        self
    }

    /// Add quantum commands from a slice
    pub fn add_quantum_commands(&mut self, commands: &[QuantumCommand]) -> &mut Self {
        // Begin batch message
        self.add_message(MessageType::BeginBatch, &[], MessageFlags::NONE);

        // Add each command
        for cmd in commands {
            self.add_quantum_gate(&QuantumGate::from_quantum_command(cmd));
        }

        // End batch message
        self.add_message(MessageType::EndBatch, &[], MessageFlags::NONE);

        self
    }

    /// Add an X gate
    pub fn add_x(&mut self, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::x(qubit));
        }
        self
    }

    /// Add a Y gate
    pub fn add_y(&mut self, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::y(qubit));
        }
        self
    }

    /// Add a Z gate
    pub fn add_z(&mut self, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::z(qubit));
        }
        self
    }

    /// Add an H gate
    pub fn add_h(&mut self, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::h(qubit));
        }
        self
    }

    /// Add CX (controlled-X) gates between pairs of qubits
    ///
    /// # Panics
    ///
    /// This function will panic if the controls and targets arrays do not have the same length.
    pub fn add_cx(&mut self, controls: &[usize], targets: &[usize]) -> &mut Self {
        assert_eq!(
            controls.len(),
            targets.len(),
            "Controls and targets arrays must have the same length"
        );
        for (&control, &target) in controls.iter().zip(targets.iter()) {
            self.add_quantum_gate(&QuantumGate::cx(control, target));
        }
        self
    }

    /// Add RZZ gates between pairs of qubits
    ///
    /// # Panics
    ///
    /// This function will panic if the qubits1 and qubits2 arrays do not have the same length.
    pub fn add_rzz(&mut self, theta: f64, qubits1: &[usize], qubits2: &[usize]) -> &mut Self {
        assert_eq!(
            qubits1.len(),
            qubits2.len(),
            "Qubit1 and qubit2 arrays must have the same length"
        );
        for (&qubit1, &qubit2) in qubits1.iter().zip(qubits2.iter()) {
            self.add_quantum_gate(&QuantumGate::rzz(theta, qubit1, qubit2));
        }
        self
    }

    /// Add SZZ gates between pairs of qubits
    ///
    /// # Panics
    ///
    /// This function will panic if the qubits1 and qubits2 arrays do not have the same length.
    pub fn add_szz(&mut self, qubits1: &[usize], qubits2: &[usize]) -> &mut Self {
        assert_eq!(
            qubits1.len(),
            qubits2.len(),
            "Qubit1 and qubit2 arrays must have the same length"
        );
        for (&qubit1, &qubit2) in qubits1.iter().zip(qubits2.iter()) {
            self.add_quantum_gate(&QuantumGate::szz(qubit1, qubit2));
        }
        self
    }

    /// Add an RZ gate
    pub fn add_rz(&mut self, theta: f64, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::rz(theta, qubit));
        }
        self
    }

    /// Add an R1XY gate
    pub fn add_r1xy(&mut self, phi: f64, theta: f64, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::r1xy(phi, theta, qubit));
        }
        self
    }

    /// Add measurement operations for multiple qubits
    ///
    /// # Panics
    ///
    /// This function will panic if there are not enough result IDs provided for all qubits.
    pub fn add_measurements(&mut self, qubit_ids: &[usize], result_ids: &[usize]) -> &mut Self {
        assert!(
            qubit_ids.len() <= result_ids.len(),
            "Not enough result IDs provided for all qubits"
        );

        for (i, &qubit) in qubit_ids.iter().enumerate() {
            let result_id = result_ids[i];

            // Create measurement header directly
            let meas_header = MeasurementHeader {
                qubit: u32::try_from(qubit).unwrap(),
                result_id: u32::try_from(result_id).unwrap(),
            };

            // Add measurement message
            self.add_message(
                MessageType::Measurement,
                bytes_of(&meas_header),
                MessageFlags::NONE,
            );
        }
        self
    }

    /// Add a Prep gate
    pub fn add_prep(&mut self, qubits: &[usize]) -> &mut Self {
        for &qubit in qubits {
            self.add_quantum_gate(&QuantumGate::prep(qubit));
        }
        self
    }

    /// Add a flush command
    pub fn add_flush(&mut self, is_last: bool) -> &mut Self {
        let flags = if is_last {
            MessageFlags::LAST_MESSAGE
        } else {
            MessageFlags::NONE
        };
        self.add_message(MessageType::Flush, &[], flags)
    }

    /// Check how many messages have been added
    #[must_use]
    pub fn message_count(&self) -> u32 {
        self.msg_count
    }

    /// Check what mode the builder is in
    #[must_use]
    pub fn mode(&self) -> BuilderMode {
        self.mode
    }

    /// Clear the builder and start fresh
    pub fn clear(&mut self) -> &mut Self {
        *self = Self::new();
        self
    }

    /// Build the final message batch without type checking
    pub fn build_unchecked(&mut self) -> ByteMessage {
        // Calculate total size and update batch header
        let total_size = self.buffer.len();
        let header = BatchHeader::new(
            self.msg_count,
            u32::try_from(total_size).unwrap_or(u32::MAX),
        );
        // Write header to the start of the buffer
        self.buffer[0..size_of::<BatchHeader>()].copy_from_slice(bytes_of(&header));

        // Return a ByteMessage with the buffer
        ByteMessage::new(self.buffer.clone())
    }

    /// Build the message and return it
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The builder mode is not specified (still Empty) but messages have been added
    /// - The builder mode is `QuantumOperations` but no quantum operations were added
    pub fn build(&mut self) -> ByteMessage {
        // Validate that a mode was explicitly set if operations were added
        assert!(
            !(self.msg_count > 0 && self.mode == BuilderMode::Empty),
            "Builder mode not specified. Call for_quantum_operations() or for_measurement_results() before adding operations."
        );

        // Add validation based on the builder's current mode
        match self.mode {
            BuilderMode::Empty => {
                // Create a minimal empty message if nothing was added
                if self.msg_count == 0 {
                    self.add_flush(true);
                }
            }
            BuilderMode::QuantumOperations | BuilderMode::MeasurementResults => {
                // For quantum operations and measurement results, ensure we have both BeginBatch and EndBatch
                // Check if the last message is already an EndBatch
                let has_end_batch = self.buffer.len() >= size_of::<MessageHeader>() && {
                    let header_offset = self.buffer.len() - size_of::<MessageHeader>();
                    let header_slice =
                        &self.buffer[header_offset..header_offset + size_of::<MessageHeader>()];
                    let header = bytemuck::from_bytes::<MessageHeader>(header_slice);
                    header.msg_type == MessageType::EndBatch as u8
                };

                if !has_end_batch {
                    self.add_message(MessageType::EndBatch, &[], MessageFlags::NONE);
                }
            }
            // Other modes don't need special handling
            BuilderMode::ControlMessage => {}
        }

        self.build_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::byte::gate_type::GateTypeId;
    use crate::channels::byte::protocol::{BATCH_MAGIC, PROTOCOL_VERSION};

    #[test]
    fn test_builder_basic() {
        // Create a builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add some gates
        builder.add_h(&[0]);
        builder.add_cx(&[0], &[1]);
        builder.add_measurements(&[2], &[0]);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].gate_type, GateTypeId::H);
        assert_eq!(commands[0].qubits, vec![0]);
        assert_eq!(commands[1].gate_type, GateTypeId::CX);
        assert_eq!(commands[1].qubits, vec![0, 1]);
        assert_eq!(commands[2].gate_type, GateTypeId::Measure);
        assert_eq!(commands[2].qubits, vec![2]);
        assert_eq!(commands[2].result_id, Some(0));
    }

    #[test]
    fn test_builder_measurement_message() {
        // Create a builder for measurement results
        let mut builder = MessageBuilder::new();
        let _ = builder.for_measurement_results();

        // Add some measurement results
        builder.add_measurement_results(&[0, 1], &[0, 1]);

        // Build the message
        let message = builder.build();

        // Verify the message type
        assert_eq!(message.message_type().unwrap(), MessageType::BeginBatch);
    }

    #[test]
    fn test_builder_gates() {
        // Create a builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add various gates
        builder.add_h(&[0]);
        builder.add_x(&[1]);
        builder.add_y(&[2]);
        builder.add_z(&[3]);
        builder.add_rz(0.5, &[4]);
        builder.add_r1xy(0.1, 0.2, &[5]);
        builder.add_measurements(&[6], &[0]);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 7);
        assert_eq!(commands[0].gate_type, GateTypeId::H);
        assert_eq!(commands[1].gate_type, GateTypeId::X);
        assert_eq!(commands[2].gate_type, GateTypeId::Y);
        assert_eq!(commands[3].gate_type, GateTypeId::Z);
        assert_eq!(commands[4].gate_type, GateTypeId::RZ);
        assert_eq!(commands[4].params, vec![0.5]);
        assert_eq!(commands[5].gate_type, GateTypeId::R1XY);
        assert_eq!(commands[5].params, vec![0.1, 0.2]);
        assert_eq!(commands[6].gate_type, GateTypeId::Measure);
        assert_eq!(commands[6].result_id, Some(0));
    }

    #[test]
    #[should_panic(
        expected = "Cannot mix quantum operations and measurement results in the same message"
    )]
    fn test_builder_type_checking() {
        // Create a builder for measurement results
        let mut builder = MessageBuilder::new();
        let _ = builder.for_measurement_results();

        // Try to add a gate (should panic)
        builder.add_h(&[0]);
    }

    #[test]
    fn test_builder_empty() {
        // Create an empty builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Build the message
        let message = builder.build();

        // Verify the message is empty
        assert!(message.is_empty().unwrap());
    }

    #[test]
    fn test_add_measure_collections() {
        // Create a builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add measurements for multiple qubits
        let qubits = vec![0, 1, 2];
        let result_ids = vec![10, 11, 12];
        builder.add_measurements(&qubits, &result_ids);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 3);
        for i in 0..3 {
            assert_eq!(commands[i].gate_type, GateTypeId::Measure);
            assert_eq!(commands[i].qubits, vec![qubits[i]]);
            assert_eq!(commands[i].result_id, Some(result_ids[i]));
        }
    }

    #[test]
    fn test_batch_structure() {
        // Create a builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add a gate
        builder.add_h(&[0]);

        // Build the message
        let message = builder.build();

        // Verify the batch structure
        let bytes = message.as_bytes();
        assert!(bytes.len() >= size_of::<BatchHeader>());

        // Parse the batch header
        let batch_header = bytemuck::from_bytes::<BatchHeader>(&bytes[0..size_of::<BatchHeader>()]);
        assert_eq!(batch_header.magic, BATCH_MAGIC);
        assert_eq!(batch_header.version, PROTOCOL_VERSION);
        assert_eq!(batch_header.msg_count, 3);
    }

    #[test]
    fn test_for_quantum_operations() {
        // Create a builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add a gate
        builder.add_h(&[0]);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].gate_type, GateTypeId::H);
    }

    #[test]
    fn test_message_count_and_clear() {
        // Create a builder
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add some gates
        builder.add_h(&[0]);
        builder.add_cx(&[0], &[1]);

        // Check the message count
        assert_eq!(builder.message_count(), 3);

        // Clear the builder
        builder.clear();

        // Check the message count after clearing
        assert_eq!(builder.message_count(), 0);

        // Add a new gate
        builder.add_h(&[0]);

        // Check the message count again
        assert_eq!(builder.message_count(), 1);
    }
}
