use crate::channels::byte::builder::MessageBuilder;
use crate::channels::byte::protocol::{
    BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageHeader, MessageType,
    QuantumGateHeader, calc_padding,
};
use crate::errors::QueueError;
use bytemuck::from_bytes;
use log::trace;
use pecos_core::types::{GateType, QuantumCommand};
use std::mem::size_of;

/// A message encoded using the PECOS byte protocol
#[derive(Clone)]
pub struct ByteMessage {
    bytes: Vec<u8>,
}

impl ByteMessage {
    /// Create a new `ByteMessage` from raw bytes
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Get a reference to the raw bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the message and return the raw bytes
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Create a new message builder
    #[must_use]
    pub fn builder() -> MessageBuilder {
        MessageBuilder::new()
    }

    /// Determine the message type by parsing the header
    pub fn message_type(&self) -> Result<MessageType, QueueError> {
        if self.bytes.len() < size_of::<BatchHeader>() {
            return Err(QueueError::OperationError(
                "Message too small for batch header".into(),
            ));
        }

        // Parse batch header
        let batch_header = *from_bytes::<BatchHeader>(&self.bytes[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(QueueError::OperationError("Invalid batch header".into()));
        }

        // Need at least one message to determine type
        if batch_header.msg_count == 0 {
            return Err(QueueError::OperationError(
                "Batch contains no messages".into(),
            ));
        }

        // Skip to first message header (after batch header)
        let msg_offset = size_of::<BatchHeader>();
        if self.bytes.len() < msg_offset + size_of::<MessageHeader>() {
            return Err(QueueError::OperationError(
                "Message too small for message header".into(),
            ));
        }

        // Parse message header
        let msg_header = *from_bytes::<MessageHeader>(
            &self.bytes[msg_offset..msg_offset + size_of::<MessageHeader>()],
        );
        msg_header
            .get_type()
            .map_err(|e| QueueError::OperationError(e.to_string()))
    }

    /// Check if this message is empty (contains no operations)
    pub fn is_empty(&self) -> Result<bool, QueueError> {
        match self.message_type()? {
            MessageType::Flush => Ok(true),
            MessageType::BeginBatch => {
                // Check if this is a batch with no operations
                let commands = self.parse_quantum_operations()?;
                Ok(commands.is_empty())
            }
            _ => Ok(false),
        }
    }

    /// Parse quantum operations from this message
    pub fn parse_quantum_operations(&self) -> Result<Vec<QuantumCommand>, QueueError> {
        if self.bytes.len() < size_of::<BatchHeader>() {
            return Err(QueueError::OperationError(
                "Message too small for batch header".into(),
            ));
        }

        // Parse batch header
        let batch_header = *from_bytes::<BatchHeader>(&self.bytes[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(QueueError::OperationError("Invalid batch header".into()));
        }

        let mut commands = Vec::new();
        let mut offset = size_of::<BatchHeader>();
        let mut in_command_batch = false;

        // Process each message
        for _ in 0..batch_header.msg_count {
            if offset + size_of::<MessageHeader>() > self.bytes.len() {
                break;
            }

            // Parse message header
            let msg_header = *from_bytes::<MessageHeader>(
                &self.bytes[offset..offset + size_of::<MessageHeader>()],
            );
            offset += size_of::<MessageHeader>();

            let msg_type = msg_header
                .get_type()
                .map_err(|e| QueueError::OperationError(e.to_string()))?;

            let payload_size = msg_header.payload_size as usize;
            let payload_end = offset + payload_size;

            if payload_end > self.bytes.len() {
                return Err(QueueError::OperationError(format!(
                    "Message payload extends beyond message bounds: offset={}, size={}, total_len={}",
                    offset,
                    payload_size,
                    self.bytes.len()
                )));
            }

            match msg_type {
                MessageType::BeginBatch => {
                    in_command_batch = true;
                }
                MessageType::EndBatch => {
                    // End of batch reached
                    return Ok(commands);
                }
                MessageType::QuantumGate if in_command_batch => {
                    // Process quantum gate
                    let payload = &self.bytes[offset..payload_end];
                    let cmd = Self::parse_quantum_gate(payload)?;
                    commands.push(cmd);
                }
                MessageType::Measurement if in_command_batch => {
                    // Process measurement
                    let payload = &self.bytes[offset..payload_end];
                    let cmd = Self::parse_measurement(payload)?;
                    commands.push(cmd);
                }
                _ => {
                    // Skip other message types
                    trace!("Skipping message type: {:?}", msg_type);
                }
            }

            // Move offset to next message, accounting for padding
            offset = payload_end;
            let padding = calc_padding(payload_size, 4);
            if padding > 0 {
                offset += padding;
            }
        }

        Ok(commands)
    }

    /// Parse measurements from this message
    pub fn parse_measurements(&self) -> Result<Vec<u32>, QueueError> {
        if self.bytes.len() < size_of::<BatchHeader>() {
            return Err(QueueError::OperationError(
                "Message too small for batch header".into(),
            ));
        }

        // Parse batch header
        let batch_header = *from_bytes::<BatchHeader>(&self.bytes[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(QueueError::OperationError("Invalid batch header".into()));
        }

        let mut measurements = Vec::new();
        let mut offset = size_of::<BatchHeader>();

        // Process each message
        for _ in 0..batch_header.msg_count {
            if offset + size_of::<MessageHeader>() > self.bytes.len() {
                break;
            }

            // Parse message header
            let msg_header = *from_bytes::<MessageHeader>(
                &self.bytes[offset..offset + size_of::<MessageHeader>()],
            );
            offset += size_of::<MessageHeader>();

            let msg_type = msg_header
                .get_type()
                .map_err(|e| QueueError::OperationError(e.to_string()))?;

            let payload_size = msg_header.payload_size as usize;
            let payload_end = offset + payload_size;

            if payload_end > self.bytes.len() {
                return Err(QueueError::OperationError(format!(
                    "Message payload extends beyond message bounds: offset={}, size={}, total_len={}",
                    offset,
                    payload_size,
                    self.bytes.len()
                )));
            }

            if msg_type == MessageType::MeasurementResult {
                // Process measurement result
                let payload = &self.bytes[offset..payload_end];
                if payload.len() >= size_of::<MeasurementResultHeader>() {
                    let result_header = *from_bytes::<MeasurementResultHeader>(
                        &payload[0..size_of::<MeasurementResultHeader>()],
                    );

                    // Encode as u32
                    let message = ((result_header.result_id & 0xFFFF) << 16)
                        | (result_header.outcome & 0xFFFF);
                    measurements.push(message);
                }
            }

            // Move offset to next message, accounting for padding
            offset = payload_end;
            let padding = calc_padding(payload_size, 4);
            if padding > 0 {
                offset += padding;
            }
        }

        Ok(measurements)
    }

    /// Parse a quantum gate message payload
    fn parse_quantum_gate(payload: &[u8]) -> Result<QuantumCommand, QueueError> {
        if payload.len() < size_of::<QuantumGateHeader>() {
            return Err(QueueError::OperationError(
                "Quantum gate message payload too small".into(),
            ));
        }

        let header = *from_bytes::<QuantumGateHeader>(&payload[0..size_of::<QuantumGateHeader>()]);
        let num_qubits = header.num_qubits as usize;
        let has_params = header.has_params != 0;

        // Calculate and validate sizes
        let qubits_size = num_qubits * size_of::<u32>();
        let minimum_size = size_of::<QuantumGateHeader>() + qubits_size;

        if payload.len() < minimum_size {
            return Err(QueueError::OperationError(
                "Quantum gate message payload too small for qubit indices".into(),
            ));
        }

        // Read qubit indices
        let mut qubits = Vec::with_capacity(num_qubits);
        let qubits_offset = size_of::<QuantumGateHeader>();

        for i in 0..num_qubits {
            let offset = qubits_offset + i * size_of::<u32>();
            let qubit = u32::from_le_bytes([
                payload[offset],
                payload[offset + 1],
                payload[offset + 2],
                payload[offset + 3],
            ]);
            qubits.push(usize::try_from(qubit).unwrap());
        }

        // Determine gate type and parameters
        let gate = match header.gate_type {
            1 => GateType::X,
            2 => GateType::Y,
            3 => GateType::Z,
            4 => GateType::H,
            5 => GateType::CX,
            6 => {
                // RZ gate
                if !has_params || payload.len() < minimum_size + size_of::<f64>() {
                    return Err(QueueError::OperationError(
                        "RZ gate requires parameter theta".into(),
                    ));
                }

                let params_offset = qubits_offset + qubits_size;
                let theta = f64::from_le_bytes([
                    payload[params_offset],
                    payload[params_offset + 1],
                    payload[params_offset + 2],
                    payload[params_offset + 3],
                    payload[params_offset + 4],
                    payload[params_offset + 5],
                    payload[params_offset + 6],
                    payload[params_offset + 7],
                ]);

                GateType::RZ { theta }
            }
            7 => {
                // R1XY gate
                if !has_params || payload.len() < minimum_size + 2 * size_of::<f64>() {
                    return Err(QueueError::OperationError(
                        "R1XY gate requires parameters phi and theta".into(),
                    ));
                }

                let params_offset = qubits_offset + qubits_size;
                let phi = f64::from_le_bytes([
                    payload[params_offset],
                    payload[params_offset + 1],
                    payload[params_offset + 2],
                    payload[params_offset + 3],
                    payload[params_offset + 4],
                    payload[params_offset + 5],
                    payload[params_offset + 6],
                    payload[params_offset + 7],
                ]);

                let theta = f64::from_le_bytes([
                    payload[params_offset + 8],
                    payload[params_offset + 9],
                    payload[params_offset + 10],
                    payload[params_offset + 11],
                    payload[params_offset + 12],
                    payload[params_offset + 13],
                    payload[params_offset + 14],
                    payload[params_offset + 15],
                ]);

                GateType::R1XY { phi, theta }
            }
            8 => GateType::SZZ,
            _ => {
                return Err(QueueError::OperationError(format!(
                    "Unknown gate type: {}",
                    header.gate_type
                )));
            }
        };

        Ok(QuantumCommand { gate, qubits })
    }

    /// Parse a measurement message payload
    fn parse_measurement(payload: &[u8]) -> Result<QuantumCommand, QueueError> {
        if payload.len() < size_of::<MeasurementHeader>() {
            return Err(QueueError::OperationError(
                "Measurement message payload too small".into(),
            ));
        }

        let header = *from_bytes::<MeasurementHeader>(&payload[0..size_of::<MeasurementHeader>()]);

        Ok(QuantumCommand {
            gate: GateType::Measure {
                result_id: usize::try_from(header.result_id).unwrap(),
            },
            qubits: vec![usize::try_from(header.qubit).unwrap()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::byte::protocol::MessageFlags;

    #[test]
    fn test_bytemap_builder() {
        // Create commands
        let commands = vec![
            QuantumCommand {
                gate: GateType::H,
                qubits: vec![0],
            },
            QuantumCommand {
                gate: GateType::CX,
                qubits: vec![0, 1],
            },
        ];

        // Use the builder
        let message = ByteMessage::builder()
            .add_quantum_commands(&commands)
            .build();

        // Parse and verify
        let parsed_commands = message.parse_quantum_operations().unwrap();
        assert_eq!(parsed_commands.len(), 2);
        assert!(matches!(parsed_commands[0].gate, GateType::H));
        assert!(matches!(parsed_commands[1].gate, GateType::CX));
    }

    #[test]
    fn test_message_type() {
        // Create a message with a single command
        let commands = vec![QuantumCommand {
            gate: GateType::H,
            qubits: vec![0],
        }];

        // Create using the builder
        let message = ByteMessage::builder()
            .add_quantum_commands(&commands)
            .build();

        // Get the message type
        let msg_type = message.message_type().unwrap();

        // Verify it's a BeginBatch type (since that's how quantum operations are wrapped)
        assert_eq!(msg_type, MessageType::BeginBatch);

        // Create a measurement message
        let message = ByteMessage::builder()
            .add_measurement_results(&[1], &[0])
            .build();

        // Get the message type
        let msg_type = message.message_type().unwrap();

        // Verify it's a MeasurementResult type
        assert_eq!(msg_type, MessageType::MeasurementResult);
    }

    #[test]
    fn test_parse_measurements() {
        // Create measurement results
        let message = ByteMessage::builder()
            .add_measurement_results(&[0, 1], &[1, 2])
            .build();

        // Parse and verify
        let measurements = message.parse_measurements().unwrap();
        assert_eq!(measurements.len(), 2);
        assert_eq!(measurements[0], (1 << 16)); // result_id=1, outcome=0
        assert_eq!(measurements[1], (2 << 16) | 1); // result_id=2, outcome=1
    }

    #[test]
    fn test_is_empty() {
        // Create an empty flush message
        let message = ByteMessage::builder()
            .add_message(MessageType::Flush, &[], MessageFlags::NONE)
            .build();

        // Verify it's empty
        assert!(message.is_empty().unwrap());

        // Create a non-empty message
        let message = ByteMessage::builder()
            .add_quantum_commands(&[QuantumCommand {
                gate: GateType::H,
                qubits: vec![0],
            }])
            .build();

        // Verify it's not empty
        assert!(!message.is_empty().unwrap());
    }
}
