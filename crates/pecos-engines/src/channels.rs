// src/channels.rs
//! Channel interfaces for communication between engine components
//!
//! This module provides the core communication interfaces and implementations
//! for the PECOS simulator.

use crate::errors::QueueError;
use pecos_core::types::CommandBatch;
use std::any::Any;

pub mod byte;

// Re-export the byte channel for easy access
pub use byte::ByteChannel;

/// A representation of a measurement result
///
/// Packed as a 32-bit integer where:
/// - The high 16 bits represent the `result_id`
/// - The low 16 bits represent the outcome (0 or 1)
pub type Message = u32;

/// Trait defining interface for command channels that send quantum commands from
/// classical to quantum components
pub trait CommandChannel: Send + Sync {
    /// Sends a batch of quantum commands through the channel.
    ///
    /// # Errors
    /// This function returns a `QueueError` if the send operation fails for any reason.
    fn send_batch(&mut self, batch: &CommandBatch) -> Result<(), QueueError>;

    /// Receives a batch of commands from the channel.
    ///
    /// # Errors
    /// This function returns a `QueueError` if the received operation fails.
    fn receive_batch(&mut self) -> Result<Option<CommandBatch>, QueueError>;

    /// Flushes the channel and signals the end of commands.
    ///
    /// # Errors
    /// Returns a `QueueError` if the flush operation fails.
    fn flush(&mut self) -> Result<(), QueueError>;

    /// Returns this channel as a trait object for dynamic casting
    fn as_any(&self) -> &dyn Any;
}

/// Trait defining interface for measurement channels that send quantum measurement
/// results from quantum to classical components
pub trait MessageChannel: Send + Sync {
    /// Sends a measurement through the channel.
    ///
    /// # Parameters
    /// - `measurement`: The measurement to send, encoded as a u32 where:
    ///   - The high 16 bits represent the result ID
    ///   - The low 16 bits represent the measurement outcome
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error locking the queue.
    /// - The operation fails for any reason.
    fn send_measurement(&mut self, measurement: Message) -> Result<(), QueueError>;

    /// Receives a measurement from the channel.
    ///
    /// # Returns
    /// - `Ok(Some(Message))`: A received measurement.
    /// - `Ok(None)`: No more measurements.
    ///
    /// # Errors
    /// - Returns `QueueError` if receiving the measurement fails.
    fn receive_message(&mut self) -> Result<Option<Message>, QueueError>;

    /// Flushes any remaining data in the channel and signals end of measurements.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error locking the queue.
    /// - The flush operation fails for any reason.
    fn flush(&mut self) -> Result<(), QueueError>;

    /// Returns this channel as a trait object for dynamic casting
    fn as_any(&self) -> &dyn Any;
}

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_core::types::{GateType, QuantumCommand};
    use std::io::Cursor;

    #[test]
    fn test_byte_channel() {
        // Create a simple batch
        let mut batch = CommandBatch::new();
        batch.add_command(QuantumCommand {
            gate: GateType::H,
            qubits: vec![0],
        });

        // Create binary data for a measurement result
        let mut builder = byte::builder::MessageBuilder::new();
        let msg_data = builder.add_measurement_result(42, 1, false).build();

        // Test receiving measurement
        let mut msg_channel =
            byte::ByteChannel::new(Box::new(Cursor::new(msg_data)), Box::new(Vec::new()));

        let msg = msg_channel.receive_message().unwrap();
        assert_eq!(msg, Some(((42 & 0xFFFF) << 16) | (1 & 0xFFFF)));
    }
}
