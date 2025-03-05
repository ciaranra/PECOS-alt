//! Byte channel implementation for binary messaging
//!
//! This module provides the `ByteChannel` implementation which handles
//! serializing and deserializing messages according to the byte protocol.

use super::builder::MessageBuilder;
use super::protocol::{
    BatchHeader, MessageHeader, MessageType, calc_padding,
};
use crate::channels::{ByteMessage, Message, MessageChannel};
use crate::errors::QueueError;
use log::debug;
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
                let header = *bytemuck::from_bytes::<BatchHeader>(&header_bytes);
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
                let header = *bytemuck::from_bytes::<MessageHeader>(&header_bytes);
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

    pub fn receive_message_data(&mut self) -> Result<Option<ByteMessage>, QueueError> {
        // Read batch header
        let Some(batch_header) = self.read_batch_header()? else {
            debug!("No batch header available");
            return Ok(None);
        };

        // Read the full message data
        let mut message_data = Vec::new();
        message_data.extend_from_slice(bytemuck::bytes_of(&batch_header));

        // Process each message
        for _ in 0..batch_header.msg_count {
            if let Some(msg_header) = self.read_message_header()? {
                message_data.extend_from_slice(bytemuck::bytes_of(&msg_header));

                let payload_size = msg_header.payload_size as usize;
                if payload_size > 0 {
                    let payload = self.read_payload(payload_size)?;
                    message_data.extend_from_slice(&payload);
                }

                // Account for padding
                let padding = calc_padding(payload_size, 4);
                if padding > 0 {
                    message_data.extend_from_slice(&vec![0u8; padding]);
                }
            } else {
                break;
            }
        }

        if message_data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ByteMessage::new(message_data)))
        }
    }

    pub fn send_message_data(&mut self, message: &ByteMessage) -> Result<(), QueueError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        writer
            .write_all(message.as_bytes())
            .map_err(|e| QueueError::OperationError(format!("Failed to write message: {e}")))?;

        writer
            .flush()
            .map_err(|e| QueueError::OperationError(format!("Failed to flush writer: {e}")))?;

        Ok(())
    }

    // Helper method to access the reader
    #[allow(dead_code)]
    pub fn as_reader(&self) -> std::sync::MutexGuard<Box<dyn Read + Send + Sync>> {
        self.reader.lock().unwrap()
    }
}

impl MessageChannel for ByteChannel {
    fn send_measurement(&mut self, measurement: Message) -> Result<(), QueueError> {
        debug!(
            "Measurement channel sending measurement: {}",
            measurement
        );

        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        // Extract result_id and outcome from the measurement
        let result_id = measurement >> 16;
        let outcome = measurement & 0xFFFF;

        // Build a ByteMessage using MessageBuilder helper
        let message = MessageBuilder::create_measurement_message(result_id, outcome, false);

        // Send the message
        writer
            .write_all(message.as_bytes())
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

                // Create a ByteMessage containing just this measurement result
                let mut builder = MessageBuilder::new();
                builder.add_message(MessageType::MeasurementResult, &payload, msg_header.get_flags());
                let message = builder.build_message();

                // Parse the measurement using ByteMessage facilities
                if let Some(&measurement) = message.parse_measurements()?.first() {
                    debug!(
                        "Received measurement: {}",
                        measurement
                    );
                    return Ok(Some(measurement));
                }
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

        // Create a flush message with LAST_MESSAGE flag
        let message = MessageBuilder::create_flush_message(true);

        writer.write_all(message.as_bytes()).map_err(|e| {
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