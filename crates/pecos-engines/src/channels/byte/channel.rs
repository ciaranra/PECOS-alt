//! Byte channel implementation for binary messaging
//!
//! This module provides the `ByteChannel` implementation which handles
//! serializing and deserializing messages according to the byte protocol.

use super::builder::MessageBuilder;
use super::protocol::{
    BatchHeader, MeasurementHeader, MeasurementResultHeader, MessageFlags, MessageHeader,
    MessageType, QuantumGateHeader, calc_padding,
};
use crate::channels::{CommandBatch, CommandChannel, Message, MessageChannel};
use crate::errors::QueueError;
use bytemuck::from_bytes;
use log::{debug, trace};
use pecos_core::types::{GateType, QuantumCommand};
use std::any::Any;
use std::io::{self, Read, Write};
use std::mem::size_of;
use std::sync::{Arc, Mutex};

/// A channel that communicates using binary messages over any Read/Write streams
#[derive(Clone)]
pub struct ByteChannel {
    reader: Arc<Mutex<Box<dyn Read + Send + Sync>>>,
    writer: Arc<Mutex<Box<dyn Write + Send + Sync>>>,
}

impl ByteChannel {
    /// Create a new `ByteChannel` with the given reader and writer
    #[must_use]
    pub fn new(reader: Box<dyn Read + Send + Sync>, writer: Box<dyn Write + Send + Sync>) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    /// Create a `ByteChannel` using standard input and output
    pub fn from_stdio() -> io::Result<Self> {
        Ok(Self {
            reader: Arc::new(Mutex::new(Box::new(io::stdin()))),
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        })
    }

    /// Create a `ByteChannel` with an anonymous pipe for testing
    pub fn create_for_shot() -> io::Result<Self> {
        use os_pipe::pipe;
        let (reader, writer) = pipe()?;

        Ok(Self {
            reader: Arc::new(Mutex::new(Box::new(reader))),
            writer: Arc::new(Mutex::new(Box::new(writer))),
        })
    }

    /// Read a batch header from the stream
    fn read_batch_header(&mut self) -> Result<Option<BatchHeader>, QueueError> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock reader: {e}")))?;

        let mut header_bytes = [0u8; size_of::<BatchHeader>()];
        match reader.read_exact(&mut header_bytes) {
            Ok(()) => {
                let header = *from_bytes::<BatchHeader>(&header_bytes);
                if !header.is_valid() {
                    return Err(QueueError::OperationError(
                        "Invalid batch header magic or version".into(),
                    ));
                }
                Ok(Some(header))
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // End of stream
                Ok(None)
            }
            Err(e) => Err(QueueError::OperationError(format!(
                "Error reading batch header: {e}"
            ))),
        }
    }

    /// Read a message header from the stream
    fn read_message_header(&mut self) -> Result<Option<MessageHeader>, QueueError> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock reader: {e}")))?;

        let mut header_bytes = [0u8; size_of::<MessageHeader>()];
        match reader.read_exact(&mut header_bytes) {
            Ok(()) => {
                let header = *from_bytes::<MessageHeader>(&header_bytes);
                Ok(Some(header))
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // End of stream
                Ok(None)
            }
            Err(e) => Err(QueueError::OperationError(format!(
                "Error reading message header: {e}"
            ))),
        }
    }

    /// Read payload bytes from the stream
    fn read_payload(&mut self, size: usize) -> Result<Vec<u8>, QueueError> {
        if size == 0 {
            return Ok(Vec::new());
        }

        let mut reader = self
            .reader
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock reader: {e}")))?;

        let mut payload = vec![0u8; size];
        reader.read_exact(&mut payload).map_err(|e| {
            QueueError::OperationError(format!("Error reading message payload: {e}"))
        })?;

        Ok(payload)
    }

    /// Skip padding bytes to maintain alignment
    fn skip_padding(&mut self, size: usize, alignment: usize) -> Result<(), QueueError> {
        let padding = calc_padding(size, alignment);
        if padding > 0 {
            let mut reader = self
                .reader
                .lock()
                .map_err(|e| QueueError::LockError(format!("Failed to lock reader: {e}")))?;

            let mut padding_bytes = vec![0u8; padding];
            reader
                .read_exact(&mut padding_bytes)
                .map_err(|e| QueueError::OperationError(format!("Error reading padding: {e}")))?;
        }

        Ok(())
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

    /// Parse a measurement result message payload
    fn parse_measurement_result(payload: &[u8]) -> Result<(u32, u32), QueueError> {
        if payload.len() < size_of::<MeasurementResultHeader>() {
            return Err(QueueError::OperationError(
                "Measurement result message payload too small".into(),
            ));
        }

        let header = *from_bytes::<MeasurementResultHeader>(
            &payload[0..size_of::<MeasurementResultHeader>()],
        );

        Ok((header.result_id, header.outcome))
    }
}

impl CommandChannel for ByteChannel {
    fn send_batch(&mut self, batch: &CommandBatch) -> Result<(), QueueError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        debug!("Command channel sending batch of {} commands", batch.len());

        // Build binary message
        let mut builder = MessageBuilder::new();
        let message_data = builder.add_command_batch(batch).build();

        // Send the message
        writer
            .write_all(&message_data)
            .map_err(|e| QueueError::OperationError(format!("Failed to write batch: {e}")))?;

        writer
            .flush()
            .map_err(|e| QueueError::OperationError(format!("Failed to flush writer: {e}")))?;

        Ok(())
    }

    fn receive_batch(&mut self) -> Result<Option<CommandBatch>, QueueError> {
        debug!("Command channel receiving batch");

        // Read batch header
        let Some(batch_header) = self.read_batch_header()? else {
            debug!("No batch header available");
            return Ok(None);
        };

        debug!(
            "Received batch header with {} messages",
            batch_header.msg_count
        );

        let mut batch = CommandBatch::new();
        let mut in_command_batch = false;

        // Process messages until end of batch
        while let Some(msg_header) = self.read_message_header()? {
            let msg_type = msg_header
                .get_type()
                .map_err(|e| QueueError::OperationError(e.to_string()))?;

            let payload_size = msg_header.payload_size as usize;

            match msg_type {
                MessageType::BeginBatch => {
                    debug!("Begin command batch");
                    in_command_batch = true;

                    // Skip any payload (should be empty)
                    if payload_size > 0 {
                        self.read_payload(payload_size)?;
                    }
                }
                MessageType::EndBatch => {
                    debug!("End command batch with {} commands", batch.len());

                    // Skip any payload (should be empty)
                    if payload_size > 0 {
                        self.read_payload(payload_size)?;
                    }

                    // Return the batch
                    return Ok(Some(batch));
                }
                MessageType::QuantumGate if in_command_batch => {
                    // Read payload
                    let payload = self.read_payload(payload_size)?;

                    // Parse quantum gate
                    let cmd = Self::parse_quantum_gate(&payload)?;
                    trace!("Received quantum gate: {:?}", cmd);

                    batch.add_command(cmd);
                }
                MessageType::Measurement if in_command_batch => {
                    // Read payload
                    let payload = self.read_payload(payload_size)?;

                    // Parse measurement
                    let cmd = Self::parse_measurement(&payload)?;
                    trace!("Received measurement: {:?}", cmd);

                    batch.add_command(cmd);
                }
                _ => {
                    debug!("Skipping message type: {:?}", msg_type);

                    // Skip payload
                    if payload_size > 0 {
                        self.read_payload(payload_size)?;
                    }
                }
            }

            // Ensure alignment for next message
            self.skip_padding(payload_size, 4)?;
        }

        // If we got here without an EndBatch message, the batch is incomplete
        if in_command_batch {
            debug!(
                "Warning: incomplete command batch with {} commands",
                batch.len()
            );
            if !batch.is_empty() {
                return Ok(Some(batch));
            }
        }

        Ok(None)
    }

    fn flush(&mut self) -> Result<(), QueueError> {
        debug!("Command channel flushing");

        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        // Send a Flush message
        let mut builder = MessageBuilder::new();
        let message_data = builder
            .add_message(MessageType::Flush, &[], MessageFlags::NONE)
            .build();

        writer.write_all(&message_data).map_err(|e| {
            QueueError::OperationError(format!("Failed to write flush message: {e}"))
        })?;

        writer
            .flush()
            .map_err(|e| QueueError::OperationError(format!("Failed to flush writer: {e}")))?;

        debug!("Command channel flushed");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MessageChannel for ByteChannel {
    fn send_measurement(&mut self, measurement: Message) -> Result<(), QueueError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        // Extract result_id and outcome from the measurement
        let result_id = measurement >> 16;
        let outcome = measurement & 0xFFFF;

        debug!(
            "Measurement channel sending result: id={}, outcome={}",
            result_id, outcome
        );

        // Build binary message
        let mut builder = MessageBuilder::new();
        let message_data = builder
            .add_measurement_result(result_id, outcome, false)
            .build();

        // Send the message
        writer
            .write_all(&message_data)
            .map_err(|e| QueueError::OperationError(format!("Failed to write measurement: {e}")))?;

        writer
            .flush()
            .map_err(|e| QueueError::OperationError(format!("Failed to flush writer: {e}")))?;

        Ok(())
    }

    fn receive_message(&mut self) -> Result<Option<Message>, QueueError> {
        debug!("Measurement channel receiving message");

        // Read batch header
        let Some(batch_header) = self.read_batch_header()? else {
            debug!("No batch header available");
            return Ok(None);
        };

        debug!(
            "Received batch header with {} messages",
            batch_header.msg_count
        );

        // Process messages until we find a measurement result
        while let Some(msg_header) = self.read_message_header()? {
            let msg_type = msg_header
                .get_type()
                .map_err(|e| QueueError::OperationError(e.to_string()))?;

            let payload_size = msg_header.payload_size as usize;

            if msg_type == MessageType::MeasurementResult {
                // Read payload
                let payload = self.read_payload(payload_size)?;

                // Parse measurement result
                let (result_id, outcome) = Self::parse_measurement_result(&payload)?;

                // Encode as a Message (u32)
                let message = ((result_id & 0xFFFF) << 16) | (outcome & 0xFFFF);
                debug!(
                    "Received measurement result: id={}, outcome={}, encoded={}",
                    result_id, outcome, message
                );

                return Ok(Some(message));
            }

            debug!("Skipping message type: {:?}", msg_type);

            // Skip payload
            if payload_size > 0 {
                self.read_payload(payload_size)?;
            }

            // Ensure alignment for next message
            self.skip_padding(payload_size, 4)?;
        }

        // No measurement results found
        debug!("No measurement results in batch");
        Ok(None)
    }

    fn flush(&mut self) -> Result<(), QueueError> {
        debug!("Measurement channel flushing");

        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        // Send a Flush message with LAST_MESSAGE flag
        let mut builder = MessageBuilder::new();
        let message_data = builder
            .add_message(MessageType::Flush, &[], MessageFlags::LAST_MESSAGE)
            .build();

        writer.write_all(&message_data).map_err(|e| {
            QueueError::OperationError(format!("Failed to write flush message: {e}"))
        })?;

        writer
            .flush()
            .map_err(|e| QueueError::OperationError(format!("Failed to flush writer: {e}")))?;

        debug!("Measurement channel flushed");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
