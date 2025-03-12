pub mod depolarizing;
pub mod pass_through;

pub use depolarizing::DepolarizingNoise;
pub use pass_through::PassThroughNoise;

use crate::channels::byte_message::ByteMessage;
use crate::engines::{ControlEngine, Engine, EngineStage};
use crate::errors::QueueError;
use dyn_clone::DynClone;
use std::any::Any;

/// Trait defining interface for quantum noise models
pub trait NoiseModel: DynClone + Send + Sync + Any {
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

    /// Resets the noise model to its initial state
    ///
    /// # Errors
    /// Returns a [`QueueError`] if the reset operation fails
    fn reset(&mut self) -> Result<(), QueueError>;

    /// Returns a reference to self as Any
    ///
    /// This allows for type-checking and downcasting without requiring
    /// experimental trait upcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns a mutable reference to self as Any
    ///
    /// This allows for type-checking and downcasting without requiring
    /// experimental trait upcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Register the NoiseModel trait with dyn_clone
dyn_clone::clone_trait_object!(NoiseModel);

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
        // For noise models, we typically just pass through the results
        Ok(EngineStage::Complete(results))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.as_mut().reset()
    }
}

impl Engine for Box<dyn NoiseModel> {
    type Input = ByteMessage;
    type Output = ByteMessage;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        self.apply_noise(input)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.as_mut().reset()
    }
}
