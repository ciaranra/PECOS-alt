use crate::engines::noise::{NoiseModel, PassThroughNoise};
use pecos_core::types::{CommandBatch, ShotResult};

use crate::Message;
use crate::channels::{CommandChannel, MessageChannel};
use crate::engines::{
    ClassicalEngine, ControlEngine, Engine, EngineStage, EngineSystem, QuantumEngine,
};
use crate::errors::QueueError;

/// `HybridEngine` coordinates between classical and quantum components via message passing
pub struct HybridEngine<C, M>
where
    C: CommandChannel + Send + Sync + 'static,
    M: MessageChannel + Send + Sync + 'static,
{
    classical: Box<dyn ClassicalEngine>,
    engine: Box<dyn Engine<Input = CommandBatch, Output = Vec<Message>>>,
    cmd_channel: C,
    meas_channel: M,
    // Store the quantum engine separately for potential reconstruction
    quantum_engine: Box<dyn QuantumEngine>,
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
        // Use a pass-through noise model by default
        Self::with_noise(
            classical,
            quantum,
            Box::new(PassThroughNoise),
            cmd_channel,
            meas_channel,
        )
    }

    pub fn with_noise(
        classical: Box<dyn ClassicalEngine>,
        quantum: Box<dyn QuantumEngine>,
        noise_model: Box<dyn NoiseModel>,
        cmd_channel: C,
        meas_channel: M,
    ) -> Self {
        // Store a clone of the quantum engine
        let quantum_clone = quantum.clone_box();

        // Create an EngineSystem with the noise model
        let engine = Box::new(EngineSystem::new(noise_model, quantum));

        Self {
            classical,
            engine,
            cmd_channel,
            meas_channel,
            quantum_engine: quantum_clone,
        }
    }

    pub fn set_noise_model(&mut self, noise_model: Option<Box<dyn NoiseModel>>) {
        // Create actual noise model or use pass-through
        let actual_noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoise));

        // Create a new engine system using the stored quantum engine
        let engine = Box::new(EngineSystem::new(
            actual_noise_model,
            self.quantum_engine.clone_box(),
        ));
        self.engine = engine;
    }

    /// Resets the state of the hybrid engine, including classical, quantum, and noise model components.
    ///
    /// This function ensures all components are returned to their initial states,
    /// allowing for reuse in subsequent operations.
    ///
    /// # Errors
    /// Returns a `QueueError` if:
    /// - Resetting the classical engine fails.
    /// - Resetting the engine fails.
    pub fn reset(&mut self) -> Result<(), QueueError> {
        // Reset the classical engine
        self.classical.reset()?;

        // Reset the engine (whether it's a QuantumEngine or an EngineSystem)
        self.engine.reset()?;

        // Return success
        Ok(())
    }

    /// Executes a single quantum circuit shot and returns the result.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - Resetting the quantum or classical engine fails.
    /// - Sending a batch of commands through the command channel fails.
    /// - Processing a batch through the quantum engine fails.
    /// - Sending measurements through the measurement channel fails.
    /// - Continuing classical processing encounters an issue.
    pub fn run_shot(&mut self) -> Result<ShotResult, QueueError> {
        let mut stage = self.classical.start(())?;

        while let EngineStage::NeedsProcessing(batch) = stage {
            // Send batch through command channel
            self.cmd_channel.send_batch(&batch)?;

            // Process through engine (could be QuantumEngine or EngineSystem)
            let measurements = self.engine.process(batch)?;

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
