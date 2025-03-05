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

/// Helper for building binary messages
pub struct MessageBuilder {
    buffer: Vec<u8>,
    msg_count: u32,
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
        }
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
    pub fn add_quantum_gate(&mut self, cmd: &QuantumCommand) -> &mut Self {
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
            GateType::Measure { .. } => {
                // Use a separate message type for measurements
                return self.add_measurement(cmd);
            }
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

    /// Add a measurement command
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Called with a non-measurement gate type
    /// - The qubit index or `result_id` cannot be converted to u32
    pub fn add_measurement(&mut self, cmd: &QuantumCommand) -> &mut Self {
        if let GateType::Measure { result_id } = cmd.gate {
            let meas_header = MeasurementHeader {
                qubit: u32::try_from(cmd.qubits[0]).unwrap(),
                result_id: u32::try_from(result_id).unwrap(),
            };
            self.add_message(
                MessageType::Measurement,
                bytes_of(&meas_header),
                MessageFlags::NONE,
            )
        } else {
            panic!("add_measurement called with non-measurement gate");
        }
    }

    /// Add a measurement result
    pub fn add_measurement_result(
        &mut self,
        result_id: u32,
        outcome: u32,
        is_last: bool,
    ) -> &mut Self {
        let result_header = MeasurementResultHeader { result_id, outcome };

        let flags = if is_last {
            MessageFlags::LAST_MESSAGE
        } else {
            MessageFlags::NONE
        };

        self.add_message(
            MessageType::MeasurementResult,
            bytes_of(&result_header),
            flags,
        )
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

    /// Build the final message batch
    pub fn build(&mut self) -> Vec<u8> {
        // Calculate total size and update batch header
        let total_size = self.buffer.len();
        let header = BatchHeader::new(
            self.msg_count,
            u32::try_from(total_size).unwrap_or(u32::MAX),
        );
        // Write header to the start of the buffer
        self.buffer[0..size_of::<BatchHeader>()].copy_from_slice(bytes_of(&header));

        // Return a clone of the buffer
        self.buffer.clone()
    }

    /// Build a ByteMessage from the constructed buffer
    pub fn build_message(&mut self) -> ByteMessage {
        ByteMessage::new(self.build())
    }

    /// Convert the builder directly into a ByteMessage
    pub fn into_message(mut self) -> ByteMessage {
        self.build_message()
    }

    /// Create a ByteMessage containing a batch of quantum commands
    pub fn create_quantum_message(commands: &[QuantumCommand]) -> ByteMessage {
        let mut builder = Self::new();
        builder.add_quantum_commands(commands);
        builder.build_message()
    }

    /// Create a ByteMessage containing a measurement result
    pub fn create_measurement_message(result_id: u32, outcome: u32, is_last: bool) -> ByteMessage {
        let mut builder = Self::new();
        builder.add_measurement_result(result_id, outcome, is_last);
        builder.build_message()
    }

    /// Create a ByteMessage with a flush command
    pub fn create_flush_message(is_last: bool) -> ByteMessage {
        let mut builder = Self::new();
        let flags = if is_last {
            MessageFlags::LAST_MESSAGE
        } else {
            MessageFlags::NONE
        };
        builder.add_message(MessageType::Flush, &[], flags);
        builder.build_message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_quantum_message() {
        // Create a test command
        let cmd = QuantumCommand {
            gate: GateType::H,
            qubits: vec![0],
        };

        // Create a ByteMessage using the new helper method
        let message = MessageBuilder::create_quantum_message(&[cmd]);

        // Parse the message back to commands
        let parsed_commands = message.parse_quantum_operations().unwrap();

        // Verify we got back the same command
        assert_eq!(parsed_commands.len(), 1);
        assert!(matches!(parsed_commands[0].gate, GateType::H));
        assert_eq!(parsed_commands[0].qubits, vec![0]);
    }

    #[test]
    fn test_create_measurement_message() {
        // Create a measurement message
        let message = MessageBuilder::create_measurement_message(42, 1, true);

        // Parse the message
        let measurements = message.parse_measurements().unwrap();

        // Verify we got the right measurement
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0], ((42 & 0xFFFF) << 16) | (1 & 0xFFFF));
    }

    #[test]
    fn test_builder_methods() {
        // Test adding a quantum gate
        let mut builder = MessageBuilder::new();
        let cmd = QuantumCommand {
            gate: GateType::CX,
            qubits: vec![0, 1],
        };

        builder.add_quantum_gate(&cmd);
        let message = builder.build_message();

        // Parse the message
        let commands = message.parse_quantum_operations().unwrap();

        // Should be empty since we didn't add begin/end batch messages
        assert!(commands.is_empty());

        // Now test with proper batch structure
        let mut builder = MessageBuilder::new();
        builder.add_message(MessageType::BeginBatch, &[], MessageFlags::NONE);
        builder.add_quantum_gate(&cmd);
        builder.add_message(MessageType::EndBatch, &[], MessageFlags::NONE);

        let message = builder.build_message();
        let commands = message.parse_quantum_operations().unwrap();

        // Now we should have our command
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0].gate, GateType::CX));
        assert_eq!(commands[0].qubits, vec![0, 1]);
    }
}