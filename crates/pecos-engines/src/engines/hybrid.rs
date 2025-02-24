use log::debug;
use pecos_core::types::ShotResult;
use pecos_noise::NoiseModel;

use crate::channels::{CommandChannel, MessageChannel};
use crate::engines::{ClassicalEngine, ControlEngine, EngineStage, QuantumEngine};
use crate::errors::QueueError;

/// `HybridEngine` coordinates between classical and quantum components via message passing
pub struct HybridEngine<C, M>
where
    C: CommandChannel + Send + Sync + 'static,
    M: MessageChannel + Send + Sync + 'static,
{
    classical: Box<dyn ClassicalEngine>,
    quantum: Box<dyn QuantumEngine>,
    cmd_channel: C,
    meas_channel: M,
    noise_model: Option<Box<dyn NoiseModel>>,
}

impl<C, M> HybridEngine<C, M>
where
    C: CommandChannel + Send + Sync + 'static + Clone,
    M: MessageChannel + Send + Sync + 'static + Clone,
{
    pub fn new(
        classical: Box<dyn ClassicalEngine>,
        quantum: Box<dyn QuantumEngine>,
        cmd_channel: C,
        meas_channel: M,
    ) -> Self {
        Self {
            classical,
            quantum,
            cmd_channel,
            meas_channel,
            noise_model: None,
        }
    }

    pub fn set_noise_model(&mut self, noise_model: Option<Box<dyn NoiseModel>>) {
        self.noise_model = noise_model;
    }

    /// Executes a single quantum circuit shot and returns the result.
    pub fn run_shot(&mut self) -> Result<ShotResult, QueueError> {
        debug!(
            "Starting new shot - thread {:?}",
            std::thread::current().id()
        );
        self.quantum.reset()?;
        self.classical.reset()?;

        let mut stage = self.classical.start(())?;

        while let EngineStage::NeedsProcessing(batch) = stage {
            // Apply noise if configured
            let batch = if let Some(noise_model) = &self.noise_model {
                noise_model.apply_noise(batch)
            } else {
                batch
            };

            // Send batch through command channel
            self.cmd_channel.send_batch(&batch)?;

            // Process through quantum engine
            let measurements = self.quantum.process(batch)?;

            // Send measurements through measurement channel
            for measurement in &measurements {
                self.meas_channel.send_measurement(*measurement)?;
            }

            // Continue classical processing with measurements
            stage = self.classical.continue_processing(measurements)?;
        }

        match stage {
            EngineStage::Complete(results) => Ok(results),
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }
}
