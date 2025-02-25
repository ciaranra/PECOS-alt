pub mod depolarizing_noise;
pub use depolarizing_noise::DepolarizingNoise;

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
