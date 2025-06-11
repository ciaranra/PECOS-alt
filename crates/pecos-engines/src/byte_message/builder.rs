//! Message builder for creating byte-encoded messages
//!
//! This module provides utilities for constructing binary messages
//! according to the byte protocol.

use crate::byte_message::message::ByteMessage;
use crate::byte_message::protocol::{
    BatchHeader, GateCommandHeader, MeasurementHeader, MessageFlags, MessageHeader, MessageType,
    OutcomeHeader, calc_padding,
};
use bytemuck::bytes_of;
use pecos_core::QubitId;
use pecos_core::gate_type::GateType;
use pecos_core::gates::Gate;
use std::mem::size_of;

// ByteMessage guarantees 4-byte alignment by storing data in Vec<u32>

// TODO: Make add_gates() add multiple qubits at a single time...

/// Enum to track what kind of message is being built
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BuilderMode {
    Empty,               // No operations added yet
    QuantumOperations,   // Contains quantum operations
    MeasurementOutcomes, // Contains measurement outcomes
    ControlMessage,      // Contains control messages like Flush
}

/// Helper for building binary messages
///
/// The builder maintains internal state tracking what kind of message is being created
/// and ensures that different message types are not mixed inappropriately.
#[derive(Debug)]
pub struct ByteMessageBuilder {
    buffer: Vec<u8>,
    msg_count: u32,
    mode: BuilderMode,
}

impl Default for ByteMessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ByteMessageBuilder {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            msg_count: self.msg_count,
            mode: self.mode,
        }
    }
}

impl ByteMessageBuilder {
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

    /// Create a builder pre-configured for measurement outcomes
    #[must_use]
    pub fn for_outcomes(&mut self) -> &mut Self {
        self.mode = BuilderMode::MeasurementOutcomes;
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
    /// - Attempting to mix quantum operations and measurement outcomes in the same message
    /// - Attempting to mix control messages with other message types
    pub fn add_message(
        &mut self,
        msg_type: MessageType,
        payload: &[u8],
        flags: MessageFlags,
    ) -> &mut Self {
        // Update mode based on message type
        match msg_type {
            MessageType::RecordData
            | MessageType::InfoMessage
            | MessageType::WarningMessage
            | MessageType::ErrorMessage
            | MessageType::DebugMessage
            | MessageType::BeginBatch
            | MessageType::EndBatch => {
                // These can be used with any mode
                // In the future, we might want to add dedicated modes for these
            }
            MessageType::GateCommand | MessageType::Measurement => {
                assert!(
                    !(self.mode == BuilderMode::MeasurementOutcomes),
                    "Cannot mix quantum operations and measurement outcomes in the same message"
                );
                if self.mode == BuilderMode::Empty {
                    self.mode = BuilderMode::QuantumOperations;
                }
            }
            MessageType::Outcome => {
                assert!(
                    !(self.mode == BuilderMode::QuantumOperations),
                    "Cannot mix quantum operations and measurement outcomes in the same message"
                );
                self.mode = BuilderMode::MeasurementOutcomes;
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
    ///
    /// This method adds a quantum gate to the message builder.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate to add
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    ///
    /// # Panics
    ///
    /// This function will panic if the number of qubits in the gate exceeds 255,
    /// as the protocol uses a u8 to represent the qubit count.
    pub fn add_gate_command(&mut self, gate: &Gate) -> &mut Self {
        // Handle measurement gates using the add_measurements method
        if gate.gate_type == GateType::Measure {
            let qubits_usize: Vec<usize> = gate.qubits.iter().map(|q| usize::from(*q)).collect();
            return self.add_measurements(&qubits_usize);
        }

        // Calculate total payload size
        let header_size = size_of::<GateCommandHeader>();
        let qubits_size = gate.qubits.len() * size_of::<u32>();
        let params_size = match gate.gate_type {
            GateType::R1XY => 2 * size_of::<f64>(),
            GateType::U => 3 * size_of::<f64>(),
            GateType::Idle | GateType::RZ | GateType::RZZ => size_of::<f64>(),
            _ => 0,
        };
        let total_size = header_size + qubits_size + params_size;

        // Create a buffer for the payload
        let mut payload = Vec::with_capacity(total_size);

        // Determine gate type and parameters
        let has_params = !gate.params.is_empty();

        // Create gate header
        let header = GateCommandHeader {
            gate_type: gate.gate_type as u8,
            num_qubits: u8::try_from(gate.qubits.len()).expect("Too many qubits for gate"),
            has_params: u8::from(has_params),
            reserved: 0,
        };

        // Add header to payload
        payload.extend_from_slice(bytes_of(&header));

        // Add qubit indices to payload (convert QubitId to usize to u32)
        for qubit in &gate.qubits {
            let qubit_u32 = u32::try_from(usize::from(*qubit)).expect("Qubit index too large");
            payload.extend_from_slice(&qubit_u32.to_le_bytes());
        }

        // Add parameters to payload if any
        for param in &gate.params {
            payload.extend_from_slice(&param.to_le_bytes());
        }

        // Add the message to the buffer
        self.add_message(MessageType::GateCommand, &payload, MessageFlags::NONE);
        self
    }

    /// Add multiple gate commands at once
    pub fn add_gate_commands(&mut self, gates: &[Gate]) -> &mut Self {
        for gate in gates {
            self.add_gate_command(gate);
        }
        self
    }

    /// Add multiple measurement outcomes at once
    ///
    /// # Panics
    ///
    /// Panics if any result outcome is too large to fit in a u32.
    pub fn add_outcomes(&mut self, outcomes: &[usize]) -> &mut Self {
        for (i, &result) in outcomes.iter().enumerate() {
            let is_last = i == outcomes.len() - 1;
            let flags = if is_last {
                MessageFlags::LAST_MESSAGE
            } else {
                MessageFlags::NONE
            };

            let result_header = OutcomeHeader {
                outcome: u32::try_from(result).expect("Result outcome too large"),
            };

            self.add_message(MessageType::Outcome, bytes_of(&result_header), flags);
        }
        self
    }

    /// Add idle operations for specified qubits for a given duration
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration of the idle period in seconds
    /// * `qubits` - The qubits that are idling
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining
    pub fn add_idle(&mut self, duration: f64, qubits: &[usize]) -> &mut Self {
        // Ensure we have qubits to work with
        if qubits.is_empty() {
            return self;
        }

        let mut idle_qubits = Vec::with_capacity(qubits.len());
        for &q in qubits {
            idle_qubits.push(q);
        }

        // Create and add the idle gate
        let idle_qubits_id: Vec<QubitId> = idle_qubits.into_iter().map(QubitId).collect();
        let gate = Gate::idle(duration, idle_qubits_id);
        self.add_gate_command(&gate)
    }

    /// Add an X gate
    pub fn add_x(&mut self, qubits: &[usize]) -> &mut Self {
        let gate = Gate::x(qubits);
        self.add_gate_command(&gate);
        self
    }

    /// Add a Y gate
    pub fn add_y(&mut self, qubits: &[usize]) -> &mut Self {
        let gate = Gate::y(qubits);
        self.add_gate_command(&gate);
        self
    }

    /// Add a Z gate
    pub fn add_z(&mut self, qubits: &[usize]) -> &mut Self {
        let gate = Gate::z(qubits);
        self.add_gate_command(&gate);
        self
    }

    /// Add an H gate
    pub fn add_h(&mut self, qubits: &[usize]) -> &mut Self {
        let gate = Gate::h(qubits);
        self.add_gate_command(&gate);
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
        let pairs: Vec<(usize, usize)> = controls
            .iter()
            .zip(targets.iter())
            .map(|(&c, &t)| (c, t))
            .collect();
        let gate = Gate::cx(&pairs);
        self.add_gate_command(&gate);
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
        let pairs: Vec<(usize, usize)> = qubits1
            .iter()
            .zip(qubits2.iter())
            .map(|(&q1, &q2)| (q1, q2))
            .collect();
        let gate = Gate::rzz(theta, &pairs);
        self.add_gate_command(&gate);
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
        let pairs: Vec<(usize, usize)> = qubits1
            .iter()
            .zip(qubits2.iter())
            .map(|(&q1, &q2)| (q1, q2))
            .collect();
        let gate = Gate::szz(&pairs);
        self.add_gate_command(&gate);
        self
    }

    /// Add an `SZZdg` gate
    ///
    /// # Arguments
    ///
    /// * `qubits1` - First set of qubits
    /// * `qubits2` - Second set of qubits
    ///
    /// # Returns
    ///
    /// * `&mut Self` - Returns self for method chaining
    ///
    /// # Panics
    ///
    /// This function will panic if the qubits1 and qubits2 arrays do not have the same length.
    pub fn add_szzdg(&mut self, qubits1: &[usize], qubits2: &[usize]) -> &mut Self {
        assert_eq!(
            qubits1.len(),
            qubits2.len(),
            "Qubit1 and qubit2 arrays must have the same length"
        );
        let pairs: Vec<(usize, usize)> = qubits1
            .iter()
            .zip(qubits2.iter())
            .map(|(&q1, &q2)| (q1, q2))
            .collect();
        let gate = Gate::szzdg(&pairs);
        self.add_gate_command(&gate);
        self
    }

    /// Add an RZ gate
    pub fn add_rz(&mut self, theta: f64, qubits: &[usize]) -> &mut Self {
        let gate = Gate::rz(theta, qubits);
        self.add_gate_command(&gate);
        self
    }

    /// Add an R1XY gate
    pub fn add_r1xy(&mut self, theta: f64, phi: f64, qubits: &[usize]) -> &mut Self {
        let gate = Gate::r1xy(theta, phi, qubits);
        self.add_gate_command(&gate);
        self
    }

    /// Add a U gate
    pub fn add_u(&mut self, theta: f64, phi: f64, lambda: f64, qubits: &[usize]) -> &mut Self {
        let gate = Gate::u(theta, phi, lambda, qubits);
        self.add_gate_command(&gate);
        self
    }

    /// Add measurement operations for multiple qubits
    ///
    /// # Panics
    ///
    /// Panics if any qubit ID is too large to fit in a u32.
    pub fn add_measurements(&mut self, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            // Create measurement header directly
            let meas_header = MeasurementHeader {
                qubit: u32::try_from(qubit).unwrap(),
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
        let gate = Gate::prep(qubits);
        self.add_gate_command(&gate);
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

    /// Add a record data message with key-value pair
    pub fn add_record_data(&mut self, key: &str, value: f64) -> &mut Self {
        let payload = format!("{key} {value}").into_bytes();
        self.add_message(MessageType::RecordData, &payload, MessageFlags::NONE)
    }

    /// Add a result record message
    pub fn add_result_record(&mut self, result_id: usize, label: Option<&str>) -> &mut Self {
        let payload = if let Some(label_str) = label {
            format!("{result_id} {label_str}").into_bytes()
        } else {
            format!("{result_id}").into_bytes()
        };
        self.add_message(MessageType::RecordData, &payload, MessageFlags::NONE)
    }

    /// Add a debug message
    pub fn add_debug_message(&mut self, msg: &str) -> &mut Self {
        self.add_message(
            MessageType::DebugMessage,
            msg.as_bytes(),
            MessageFlags::NONE,
        )
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
    ///
    /// This method completely replaces the builder with a new instance,
    /// releasing any allocated memory. Use this when memory usage is a concern
    /// or when you want absolute certainty of a fresh state.
    ///
    /// For performance-critical code or when creating many messages in sequence,
    /// consider using `reset()` instead, which preserves memory allocation.
    ///
    /// After clearing, you'll need to configure the builder for the desired message type
    /// by calling `for_quantum_operations()` or `for_outcomes()`.
    pub fn clear(&mut self) -> &mut Self {
        *self = Self::new();
        self
    }

    /// Reset the builder state while preserving allocated memory
    ///
    /// Unlike `clear()`, this method preserves the allocated memory buffer
    /// for better performance when reusing the same builder multiple times.
    /// This is the recommended method for performance-critical code,
    /// especially when creating many messages in sequence.
    ///
    /// After resetting, you'll need to configure the builder for the desired message type
    /// by calling `for_quantum_operations()` or `for_outcomes()`:
    ///
    /// ```
    /// # use pecos_engines::byte_message::ByteMessageBuilder;
    /// let mut builder = ByteMessageBuilder::new();
    ///
    /// // Create first message
    /// let _ = builder.for_quantum_operations();
    /// builder.add_h(&[0]);
    /// let message1 = builder.build();
    ///
    /// // Reset and configure for next message
    /// builder.reset();
    /// let _ = builder.for_quantum_operations();
    /// builder.add_h(&[1]);
    /// let message2 = builder.build();
    /// ```
    ///
    /// If memory usage is a concern or you want to ensure a completely fresh state,
    /// consider using `clear()` instead.
    pub fn reset(&mut self) -> &mut Self {
        // Truncate the buffer to just the batch header size
        self.buffer.truncate(size_of::<BatchHeader>());

        // Zero out the batch header area more efficiently
        // Using slice fill is more efficient than a loop for small fixed-size areas
        self.buffer.fill(0);

        // Reset message count and mode
        self.msg_count = 0;
        self.mode = BuilderMode::Empty;

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
        ByteMessage::new(&self.buffer)
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
            "Builder mode not specified. Call for_quantum_operations() or for_outcomes() before adding operations."
        );

        // Add validation based on the builder's current mode
        match self.mode {
            BuilderMode::Empty => {
                // Create a minimal empty message if nothing was added
                if self.msg_count == 0 {
                    self.add_flush(true);
                }
            }
            BuilderMode::QuantumOperations | BuilderMode::MeasurementOutcomes => {
                // For quantum operations and measurement outcomes, ensure we have both BeginBatch and EndBatch
                // Check if the last message is already an EndBatch
                let has_end_batch = self.buffer.len() >= size_of::<MessageHeader>() && {
                    let header_offset = self.buffer.len() - size_of::<MessageHeader>();
                    let header_slice =
                        &self.buffer[header_offset..header_offset + size_of::<MessageHeader>()];
                    // Unaligned read needed since message might not end on 4-byte boundary
                    let header = bytemuck::pod_read_unaligned::<MessageHeader>(header_slice);
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
    use crate::byte_message::GateType;
    use crate::byte_message::protocol::{BATCH_MAGIC, PROTOCOL_VERSION};
    use pecos_core::QubitId;

    #[test]
    fn test_gate_command_interface() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add gates using new GateCommand interface
        let gate = Gate::h(&[0]);
        builder.add_gate_command(&gate);
        let gate = Gate::rz(0.5, &[1]);
        builder.add_gate_command(&gate);

        // Test multiple gates at once
        let gates = vec![Gate::x(&[2]), Gate::cx(&[(0, 1)])];
        builder.add_gate_commands(&gates);

        // Build and verify basic structure
        let message = builder.build();
        assert!(!message.is_empty().unwrap());
    }

    #[test]
    fn test_builder_basic() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add some gates
        builder.add_h(&[0]);
        builder.add_cx(&[0], &[1]);
        builder.add_measurements(&[2]);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].gate_type, GateType::H);
        assert_eq!(commands[0].qubits, vec![QubitId(0)]);
        assert_eq!(commands[1].gate_type, GateType::CX);
        assert_eq!(commands[1].qubits, vec![QubitId(0), QubitId(1)]);
        assert_eq!(commands[2].gate_type, GateType::Measure);
        assert_eq!(commands[2].qubits, vec![QubitId(2)]);
    }

    #[test]
    fn test_builder_measurement_message() {
        // Create a builder for measurement outcomes
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_outcomes();

        // Add some measurement outcomes
        builder.add_outcomes(&[0]);

        // Build the message
        let message = builder.build();

        // Verify the message type
        assert_eq!(message.message_type().unwrap(), MessageType::BeginBatch);
    }

    #[test]
    fn test_builder_gates() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add various gates
        builder.add_h(&[0]);
        builder.add_x(&[1]);
        builder.add_y(&[2]);
        builder.add_z(&[3]);
        builder.add_rz(0.5, &[4]);
        builder.add_r1xy(0.1, 0.2, &[5]);
        builder.add_measurements(&[6]);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 7);
        assert_eq!(commands[0].gate_type, GateType::H);
        assert_eq!(commands[1].gate_type, GateType::X);
        assert_eq!(commands[2].gate_type, GateType::Y);
        assert_eq!(commands[3].gate_type, GateType::Z);
        assert_eq!(commands[4].gate_type, GateType::RZ);
        assert_eq!(commands[4].params, vec![0.5]);
        assert_eq!(commands[5].gate_type, GateType::R1XY);
        assert_eq!(commands[5].params, vec![0.1, 0.2]);
        assert_eq!(commands[6].gate_type, GateType::Measure);
    }

    #[test]
    #[should_panic(
        expected = "Cannot mix quantum operations and measurement outcomes in the same message"
    )]
    fn test_builder_type_checking() {
        // Create a builder for measurement outcomes
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_outcomes();

        // Try to add a gate (should panic)
        builder.add_h(&[0]);
    }

    #[test]
    fn test_builder_empty() {
        // Create an empty builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Build the message
        let message = builder.build();

        // Verify the message is empty
        assert!(message.is_empty().unwrap());
    }

    #[test]
    fn test_add_measure_collections() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add measurements for multiple qubits
        let qubits = vec![0, 1, 2];
        builder.add_measurements(&qubits);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 3);
        for i in 0..3 {
            assert_eq!(commands[i].gate_type, GateType::Measure);
            assert_eq!(commands[i].qubits, vec![QubitId(qubits[i])]);
        }
    }

    #[test]
    fn test_batch_structure() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add a gate
        builder.add_h(&[0]);

        // Build the message
        let message = builder.build();

        // Verify the batch structure
        let bytes = message.as_bytes();
        assert!(bytes.len() >= size_of::<BatchHeader>());

        // Parse the batch header - guaranteed aligned at offset 0
        let batch_header =
            *bytemuck::from_bytes::<BatchHeader>(&bytes[0..size_of::<BatchHeader>()]);
        assert_eq!(batch_header.magic, BATCH_MAGIC);
        assert_eq!(batch_header.version, PROTOCOL_VERSION);
        assert_eq!(batch_header.msg_count, 3);
    }

    #[test]
    fn test_for_quantum_operations() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add a gate
        builder.add_h(&[0]);

        // Build the message
        let message = builder.build();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Verify the commands
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].gate_type, GateType::H);
    }

    #[test]
    fn test_message_count_and_clear() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
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

    #[test]
    fn test_reset() {
        // Create a builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add some gates
        builder.add_h(&[0]);
        builder.add_cx(&[0], &[1]);

        // Check the message count
        assert_eq!(builder.message_count(), 3);

        // Get the buffer capacity before reset
        let capacity_before = builder.buffer.capacity();

        // Reset the builder
        builder.reset();

        // Check the message count after reset
        assert_eq!(builder.message_count(), 0);
        assert_eq!(builder.mode(), BuilderMode::Empty);

        // Verify the buffer capacity is preserved
        assert_eq!(builder.buffer.capacity(), capacity_before);

        // Configure for quantum operations again
        let _ = builder.for_quantum_operations();

        // Add a new gate
        builder.add_h(&[0]);

        // Check the message count again
        assert_eq!(builder.message_count(), 2);

        // Build the message and verify it's valid
        let message = builder.build();
        let commands = message.parse_quantum_operations().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].gate_type, GateType::H);
    }

    #[test]
    fn compare_clear_vs_reset_performance() {
        const ITERATIONS: usize = 5000;
        const TRIALS: usize = 5;

        let mut clear_durations = Vec::with_capacity(TRIALS);
        let mut reset_durations = Vec::with_capacity(TRIALS);

        for _ in 0..TRIALS {
            // Test with clear()
            let start_clear = std::time::Instant::now();
            {
                let mut builder = ByteMessageBuilder::new();

                for i in 0..ITERATIONS {
                    if i > 0 {
                        builder.clear();
                    }

                    // Configure for quantum operations
                    let _ = builder.for_quantum_operations();

                    // Add a gate
                    builder.add_h(&[0]);

                    // Build the message
                    let _message = builder.build();
                }
            }
            clear_durations.push(start_clear.elapsed());

            // Test with reset()
            let start_reset = std::time::Instant::now();
            {
                let mut builder = ByteMessageBuilder::new();

                for i in 0..ITERATIONS {
                    if i > 0 {
                        builder.reset();
                    }

                    // Configure for quantum operations
                    let _ = builder.for_quantum_operations();

                    // Add a gate
                    builder.add_h(&[0]);

                    // Build the message
                    let _message = builder.build();
                }
            }
            reset_durations.push(start_reset.elapsed());
        }

        // Calculate averages
        #[allow(clippy::cast_precision_loss)]
        let avg_clear = clear_durations
            .iter()
            .map(std::time::Duration::as_secs_f64)
            .sum::<f64>()
            / (TRIALS as f64);
        #[allow(clippy::cast_precision_loss)]
        let avg_reset = reset_durations
            .iter()
            .map(std::time::Duration::as_secs_f64)
            .sum::<f64>()
            / (TRIALS as f64);

        // Print results
        println!("Performance comparison ({TRIALS} trials of {ITERATIONS} iterations each):");
        println!("  clear() + for_quantum_operations(): {avg_clear:.6}s (average)");
        println!("  reset() + for_quantum_operations(): {avg_reset:.6}s (average)");
        println!("  reset() approach is {:.2}x faster", avg_clear / avg_reset);

        // We don't assert anything here as performance can vary by environment,
        // but reset() should generally be faster
    }
}
