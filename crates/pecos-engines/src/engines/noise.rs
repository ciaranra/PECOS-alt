pub mod depolarizing_noise;
pub use depolarizing_noise::DepolarizingNoise;

use crate::Message;
use crate::engines::{ControlEngine, EngineStage};
use crate::errors::QueueError;
use pecos_core::types::CommandBatch;

/// Trait defining interface for quantum noise models
pub trait NoiseModel: Send + Sync {
    /// Apply noise to a batch of quantum commands
    fn apply_noise(&self, commands: CommandBatch) -> CommandBatch;
    fn clone_box(&self) -> Box<dyn NoiseModel>;
    /// Resets the noise model to its initial state.
    ///
    /// # Errors
    ///
    /// Returns a [`QueueError`] if the reset operation fails.
    fn reset(&mut self) -> Result<(), QueueError>;
}

impl ControlEngine for Box<dyn NoiseModel> {
    type Input = CommandBatch;
    type Output = Vec<Message>;
    type EngineInput = CommandBatch;
    type EngineOutput = Vec<Message>;

    fn start(
        &mut self,
        input: CommandBatch,
    ) -> Result<EngineStage<CommandBatch, Vec<Message>>, QueueError> {
        // Apply noise transformation to the commands
        let noisy_commands = self.apply_noise(input);

        // Request processing of the noisy commands
        Ok(EngineStage::NeedsProcessing(noisy_commands))
    }

    fn continue_processing(
        &mut self,
        results: Vec<Message>,
    ) -> Result<EngineStage<CommandBatch, Vec<Message>>, QueueError> {
        // Just pass through results from the quantum engine
        Ok(EngineStage::Complete(results))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        NoiseModel::reset(self.as_mut())
    }
}

impl ControlEngine for &mut dyn NoiseModel {
    type Input = CommandBatch;
    type Output = Vec<Message>;
    type EngineInput = CommandBatch;
    type EngineOutput = Vec<Message>;

    fn start(
        &mut self,
        input: CommandBatch,
    ) -> Result<EngineStage<CommandBatch, Vec<Message>>, QueueError> {
        // Apply noise transformation to the commands
        let noisy_commands = self.apply_noise(input);

        // Request processing of the noisy commands
        Ok(EngineStage::NeedsProcessing(noisy_commands))
    }

    fn continue_processing(
        &mut self,
        results: Vec<Message>,
    ) -> Result<EngineStage<CommandBatch, Vec<Message>>, QueueError> {
        // Just pass through results from the quantum engine
        Ok(EngineStage::Complete(results))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        (*self).reset()
    }
}

pub struct PassThroughNoise;

impl NoiseModel for PassThroughNoise {
    fn apply_noise(&self, commands: CommandBatch) -> CommandBatch {
        // Just return the commands unchanged
        commands
    }

    fn clone_box(&self) -> Box<dyn NoiseModel> {
        Box::new(PassThroughNoise)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        Ok(())
    }
}
