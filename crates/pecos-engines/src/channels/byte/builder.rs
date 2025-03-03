//! Message builder for creating byte-encoded messages
//!
//! This module provides utilities for constructing binary messages
//! according to the byte protocol.

use super::protocol::{
    BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageFlags, MessageHeader,
    MessageType, QuantumGateHeader, calc_padding,
};
use bytemuck::bytes_of;
use pecos_core::types::{CommandBatch, GateType, QuantumCommand};
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
        let header = MessageHeader::new(msg_type, payload.len() as u32, flags);
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
            num_qubits: cmd.qubits.len() as u8,
            has_params: if has_params { 1 } else { 0 },
            reserved: 0,
        };
        payload.extend_from_slice(bytes_of(&gate_header));

        // Write qubit indices
        for qubit in &cmd.qubits {
            payload.extend_from_slice(&qubit.to_le_bytes());
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

    /// Add a command batch
    pub fn add_command_batch(&mut self, batch: &CommandBatch) -> &mut Self {
        // Begin batch message
        self.add_message(MessageType::BeginBatch, &[], MessageFlags::NONE);

        // Add each command
        for cmd in batch.commands() {
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
        let header = BatchHeader::new(self.msg_count, total_size as u32);

        // Write header to the start of the buffer
        self.buffer[0..size_of::<BatchHeader>()].copy_from_slice(bytes_of(&header));

        // Return a clone of the buffer
        self.buffer.clone()
    }
}
