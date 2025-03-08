use super::{ByteMessage, NoiseModel};
use crate::errors::QueueError;

/// A noise model that passes messages through without modification
#[derive(Clone)]
pub struct PassThroughNoise;

impl NoiseModel for PassThroughNoise {
    fn apply_noise(&self, message: ByteMessage) -> Result<ByteMessage, QueueError> {
        // Just return the message unchanged
        Ok(message)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        Ok(())
    }
}
