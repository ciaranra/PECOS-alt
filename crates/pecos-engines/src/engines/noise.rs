pub mod depolarizing;
pub mod pass_through;

pub use depolarizing::DepolarizingNoise;
pub use pass_through::PassThroughNoise;

use crate::channels::byte_message::ByteMessage;
use crate::engines::{ControlEngine, EngineStage};
use crate::errors::QueueError;

/// Trait defining interface for quantum noise models
pub trait NoiseModel: Send + Sync {
    /// Apply noise to a `ByteMessage` containing quantum commands
    ///
    /// # Parameters
    /// - `message`: A `ByteMessage` containing the quantum commands to modify
    ///
    /// # Returns
    /// - `Result<ByteMessage, QueueError>`: A new message with noise applied
    ///
    /// # Errors
    /// - Returns a `QueueError` if noise application fails
    fn apply_noise(&self, message: ByteMessage) -> Result<ByteMessage, QueueError>;

    /// Create a cloned instance of this noise model
    fn clone_box(&self) -> Box<dyn NoiseModel>;

    /// Resets the noise model to its initial state
    ///
    /// # Errors
    /// Returns a [`QueueError`] if the reset operation fails
    fn reset(&mut self) -> Result<(), QueueError>;
}

impl ControlEngine for Box<dyn NoiseModel> {
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        input: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, ByteMessage>, QueueError> {
        // Apply noise transformation to the message
        let noisy_message = self.apply_noise(input)?;

        // Request processing of the noisy commands
        Ok(EngineStage::NeedsProcessing(noisy_message))
    }

    fn continue_processing(
        &mut self,
        results: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, ByteMessage>, QueueError> {
        // Just pass through results from the quantum engine
        Ok(EngineStage::Complete(results))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        NoiseModel::reset(self.as_mut())
    }
}
