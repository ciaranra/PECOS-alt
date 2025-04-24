use super::NoiseModel;
use crate::byte_message::ByteMessage;
use crate::engines::{ControlEngine, EngineStage};
use crate::errors::QueueError;
use std::any::Any;

/// A noise model that passes through messages unchanged
///
/// This is useful as a default for systems that don't need noise.
#[derive(Clone, Debug)]
pub struct PassThroughNoise;

impl NoiseModel for PassThroughNoise {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ControlEngine for PassThroughNoise {
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        input: Self::Input,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // Simply pass through the input message unchanged
        Ok(EngineStage::NeedsProcessing(input))
    }

    fn continue_processing(
        &mut self,
        result: Self::EngineOutput,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // Simply pass through the result message unchanged
        Ok(EngineStage::Complete(result))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // No state to reset
        Ok(())
    }
}
