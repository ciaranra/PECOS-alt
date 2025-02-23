use crate::errors::QueueError;
use pecos_core::types::QuantumCommand;
use std::any::Any;

pub trait CommandChannel: Send + Sync {
    /// Sends a single quantum command through the channel.
    ///
    /// # Parameters
    /// - `cmd`: The quantum command to send.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error locking the queue.
    /// - The operation fails for any reason.
    fn send_command(&mut self, cmd: &QuantumCommand) -> Result<(), QueueError>;

    /// Receives a single command from the channel.
    ///
    /// # Returns
    /// - `Ok(Some(QuantumCommand))`: The command received.
    /// - `Ok(None)`: End of commands.
    /// - `Err(QueueError)`: If there is an error receiving a command.
    fn receive_command(&mut self) -> Result<Option<QuantumCommand>, QueueError>;

    /// Flushes any remaining data in the channel and signals end of commands.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error locking the queue.
    /// - The flush operation fails for any reason.
    fn flush(&mut self) -> Result<(), QueueError>;

    // Allow downcasting to concrete implementation
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
    /// - `Ok(Some(Message))`: The measurement received.
    /// - `Ok(None)`: End of measurements.
    /// - `Err(QueueError)`: If there is an error receiving the measurement.
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
