use crate::engines::noise::NoiseModel;
use crate::errors::QueueError;
use pecos_core::types::CommandBatch;

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
