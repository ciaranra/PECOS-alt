use super::{ByteMessage, NoiseModel};
use crate::errors::QueueError;

pub struct PassThroughNoise;

impl NoiseModel for PassThroughNoise {
    fn apply_noise(&self, message: ByteMessage) -> Result<ByteMessage, QueueError> {
        // Just return the message unchanged
        Ok(message)
    }

    fn clone_box(&self) -> Box<dyn NoiseModel> {
        Box::new(PassThroughNoise)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        Ok(())
    }
}
