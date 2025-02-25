use crate::errors::QueueError;
use pecos_core::types::CommandBatch;
use std::any::Any;

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

    fn as_any(&self) -> &dyn Any;
}

pub type Message = u32;

pub trait MessageChannel: Send + Sync {
    /// Sends a measurement through the channel.
    ///
    /// # Parameters
    /// - `measurement`: The measurement to send.
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

    // Allow downcasting to concrete implementation
    fn as_any(&self) -> &dyn Any;
}

pub mod stdio;
