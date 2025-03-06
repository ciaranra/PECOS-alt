//! Message builder for creating byte-encoded messages
//!
//! This module provides utilities for constructing binary messages
//! according to the byte protocol.

use super::protocol::{
    BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageFlags, MessageHeader,
    MessageType, QuantumGateHeader, calc_padding,
};
use crate::channels::ByteMessage;
use bytemuck::bytes_of;
use pecos_core::types::{GateType, QuantumCommand};
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
    pub fn add_message(
        &mut self,
        msg_type: MessageType,
        payload: &[u8],
        flags: MessageFlags,
    ) -> &mut Self {
        // Update mode based on message type
        match msg_type {
            MessageType::BeginBatch
            | MessageType::EndBatch
            | MessageType::QuantumGate
            | MessageType::Measurement => {
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
    fn add_quantum_gate(&mut self, cmd: &QuantumCommand) -> &mut Self {
        // Handle measurement gates using the add_measurements method
        if let GateType::Measure { result_id } = cmd.gate {
            return self.add_measurements(&[cmd.qubits[0]], &[result_id]);
        }

        // Calculate total payload size
        let header_size = size_of::<QuantumGateHeader>();
        let qubits_size = cmd.qubits.len() * size_of::<u32>();
        let params_size = match &cmd.gate {
            GateType::RZ { .. } => size_of::<f64>(),
            GateType::R1XY { .. } => 2 * size_of::<f64>(),
            _ => 0,
        };
        let total_size = header_size + qubits_size + params_size;

        // Create a buffer for the payload
        let mut payload = Vec::with_capacity(total_size);

        // Determine gate type and parameters
        let (gate_type, has_params) = match &cmd.gate {
            GateType::X => (1, false),
            GateType::Y => (2, false),
            GateType::Z => (3, false),
            GateType::H => (4, false),
            GateType::CX => (5, false),
            GateType::RZ { .. } => (6, true),
            GateType::R1XY { .. } => (7, true),
            GateType::SZZ => (8, false),
            GateType::Measure { .. } => (0, false), // Handled above, dummy values
        };

        // Create and write gate header
        let gate_header = QuantumGateHeader {
            gate_type,
            num_qubits: u8::try_from(cmd.qubits.len()).unwrap_or(u8::MAX),
            has_params: u8::from(has_params),
            reserved: 0,
        };
        payload.extend_from_slice(bytes_of(&gate_header));

        // Write qubit indices
        for &qubit in &cmd.qubits {
            // Explicitly convert usize to u32
            let qubit_u32: u32 = qubit.try_into().unwrap_or(0);
            payload.extend_from_slice(&qubit_u32.to_le_bytes());
        }

        // Write parameters if needed
        if has_params {
            match &cmd.gate {
                GateType::RZ { theta } => {
                    payload.extend_from_slice(&theta.to_le_bytes());
                }
                GateType::R1XY { phi, theta } => {
                    payload.extend_from_slice(&phi.to_le_bytes());
                    payload.extend_from_slice(&theta.to_le_bytes());
                }
                _ => {}
            }
        }

        // Add as a quantum gate message
        self.add_message(MessageType::QuantumGate, &payload, MessageFlags::NONE)
    }

    /// Add multiple measurement results at once
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
                result_id: result_id as u32,
                outcome: result as u32,
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
            self.add_quantum_gate(cmd);
        }

        // End batch message
        self.add_message(MessageType::EndBatch, &[], MessageFlags::NONE);

        self
    }

    /// Add Hadamard (H) gate(s) to the specified qubit(s)
    pub fn add_h(&mut self, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            let cmd = QuantumCommand {
                gate: GateType::H,
                qubits: vec![qubit],
            };
            self.add_quantum_gate(&cmd);
        }
        self
    }

    /// Add Pauli-X gate(s) to the specified qubit(s)
    pub fn add_x(&mut self, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            let cmd = QuantumCommand {
                gate: GateType::X,
                qubits: vec![qubit],
            };
            self.add_quantum_gate(&cmd);
        }
        self
    }

    /// Add Pauli-Y gate(s) to the specified qubit(s)
    pub fn add_y(&mut self, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            let cmd = QuantumCommand {
                gate: GateType::Y,
                qubits: vec![qubit],
            };
            self.add_quantum_gate(&cmd);
        }
        self
    }

    /// Add Pauli-Z gate(s) to the specified qubit(s)
    pub fn add_z(&mut self, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            let cmd = QuantumCommand {
                gate: GateType::Z,
                qubits: vec![qubit],
            };
            self.add_quantum_gate(&cmd);
        }
        self
    }

    /// Add RZ (Z-rotation) gate(s) with the same angle to the specified qubit(s)
    pub fn add_rz(&mut self, theta: f64, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            let cmd = QuantumCommand {
                gate: GateType::RZ { theta },
                qubits: vec![qubit],
            };
            self.add_quantum_gate(&cmd);
        }
        self
    }

    /// Add R1XY (arbitrary single-qubit rotation) gate(s) to the specified qubit(s)
    pub fn add_r1xy(&mut self, phi: f64, theta: f64, qubit_ids: &[usize]) -> &mut Self {
        for &qubit in qubit_ids {
            let cmd = QuantumCommand {
                gate: GateType::R1XY { phi, theta },
                qubits: vec![qubit],
            };
            self.add_quantum_gate(&cmd);
        }
        self
    }

    /// Add CNOT (Controlled-X) gate(s) from each control qubit to each target qubit
    pub fn add_cx(&mut self, control_ids: &[usize], target_ids: &[usize]) -> &mut Self {
        for &control in control_ids {
            for &target in target_ids {
                // Skip if control and target are the same qubit
                if control != target {
                    let cmd = QuantumCommand {
                        gate: GateType::CX,
                        qubits: vec![control, target],
                    };
                    self.add_quantum_gate(&cmd);
                }
            }
        }
        self
    }

    /// Add SZZ (quadratic phase) gate(s) between pairs of qubits
    pub fn add_szz(&mut self, qubit1_ids: &[usize], qubit2_ids: &[usize]) -> &mut Self {
        for &qubit1 in qubit1_ids {
            for &qubit2 in qubit2_ids {
                // Skip if qubits are the same
                if qubit1 != qubit2 {
                    let cmd = QuantumCommand {
                        gate: GateType::SZZ,
                        qubits: vec![qubit1, qubit2],
                    };
                    self.add_quantum_gate(&cmd);
                }
            }
        }
        self
    }

    /// Add measurement operations to the specified qubits
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

    /// Build the message with type checking
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
            BuilderMode::QuantumOperations => {
                // For quantum operations, ensure we have both BeginBatch and EndBatch
                // Instead of scanning the buffer (which can cause alignment issues),
                // just add EndBatch which is safe even if one already exists
                self.add_message(MessageType::EndBatch, &[], MessageFlags::NONE);
            }
            // Other modes don't need special handling
            _ => {}
        }

        self.build_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let message = ByteMessage::builder()
            .for_quantum_operations() // This properly initializes the builder
            .add_h(&[0])
            .add_cx(&[0], &[1])
            .add_measurements(&[1], &[0])
            .build();

        // Verify the message structure
        let commands = message.parse_quantum_operations().unwrap();
        assert_eq!(commands.len(), 3);

        // Check the H gate
        assert!(matches!(commands[0].gate, GateType::H));
        assert_eq!(commands[0].qubits, vec![0]);

        // Check the CX gate
        assert!(matches!(commands[1].gate, GateType::CX));
        assert_eq!(commands[1].qubits, vec![0, 1]);

        // Check the measurement
        if let GateType::Measure { result_id } = commands[2].gate {
            assert_eq!(result_id, 0);
        } else {
            panic!("Expected Measure gate");
        }
        assert_eq!(commands[2].qubits, vec![1]);
    }

    #[test]
    fn test_builder_measurement_message() {
        let message = MessageBuilder::new()
            .add_measurement_results(&[0, 1], &[1, 2])
            .build();

        // Verify the measurements
        let measurements = message.parse_measurements().unwrap();
        assert_eq!(measurements.len(), 2);
        assert_eq!(measurements[0], (1 << 16)); // result_id=1, outcome=0
        assert_eq!(measurements[1], (2 << 16) | 1); // result_id=2, outcome=1
    }

    #[test]
    fn test_builder_gates() {
        // Test each specific gate builder method
        let message = ByteMessage::builder()
            .for_quantum_operations()
            .add_h(&[0])
            .add_x(&[1])
            .add_y(&[2])
            .add_z(&[3])
            .add_cx(&[0], &[1])
            .add_rz(0.5, &[2])
            .add_r1xy(0.1, 0.2, &[3])
            .add_szz(&[0], &[1])
            .build();

        // Verify the message structure
        let commands = message.parse_quantum_operations().unwrap();
        assert_eq!(commands.len(), 8);

        // Check a few gates
        assert!(matches!(commands[0].gate, GateType::H));
        assert!(matches!(commands[1].gate, GateType::X));

        // Check RZ gate
        if let GateType::RZ { theta } = commands[5].gate {
            assert_eq!(theta, 0.5);
        } else {
            panic!("Expected RZ gate");
        }

        // Check R1XY gate
        if let GateType::R1XY { phi, theta } = commands[6].gate {
            assert_eq!(phi, 0.1);
            assert_eq!(theta, 0.2);
        } else {
            panic!("Expected R1XY gate");
        }
    }

    #[test]
    #[should_panic(expected = "Cannot mix quantum operations and measurement results")]
    fn test_builder_type_checking() {
        // This should panic because we're mixing message types
        let _ = MessageBuilder::new()
            .add_h(&[0]) // Quantum operation
            .add_measurement_results(&[1], &[0]) // Measurement result
            .build();
    }

    #[test]
    fn test_builder_empty() {
        // Building with no operations should create a flush message
        let message = MessageBuilder::new().build();
        let msg_type = message.message_type().unwrap();
        assert_eq!(msg_type, MessageType::Flush);
    }

    #[test]
    fn test_add_measure_collections() {
        // Test with collections of qubits and results
        let message = ByteMessage::builder()
            .for_quantum_operations() // Change to quantum operations since we're using add_measurements
            .add_measurements(&[0, 1, 2], &[10, 20, 30])
            .build();

        let commands = message.parse_quantum_operations().unwrap();
        assert_eq!(commands.len(), 3);

        // Check each measurement has the right qubit and result_id
        let expected_pairs = [(0, 10), (1, 20), (2, 30)];
        for (i, cmd) in commands.iter().enumerate() {
            if let GateType::Measure { result_id } = cmd.gate {
                assert_eq!(result_id, expected_pairs[i].1);
            } else {
                panic!("Expected Measure gate");
            }
            assert_eq!(cmd.qubits, vec![expected_pairs[i].0]);
        }
    }

    #[test]
    fn test_batch_structure() {
        // Test that quantum operations are properly wrapped in BeginBatch/EndBatch
        let mut builder = MessageBuilder::new();
        builder.add_message(MessageType::BeginBatch, &[], MessageFlags::NONE);
        builder.add_h(&[0]);

        // Build should add EndBatch automatically
        let message = builder.build();

        let commands = message.parse_quantum_operations().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0].gate, GateType::H));
    }

    #[test]
    fn test_for_quantum_operations() {
        // Test the factory method for quantum operations
        let message = ByteMessage::builder()
            .for_quantum_operations()
            .add_h(&[0])
            .build();

        // Should already have BeginBatch
        let commands = message.parse_quantum_operations().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0].gate, GateType::H));
    }

    #[test]
    fn test_message_count_and_clear() {
        // Test message counting and clearing
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();
        assert_eq!(builder.message_count(), 1); // 1 for BeginBatch

        builder.add_h(&[0]).add_h(&[1]);
        assert_eq!(builder.message_count(), 3); // BeginBatch + 2 H gates (EndBatch gets added in build())

        builder.clear();
        assert_eq!(builder.message_count(), 0);
    }
}
