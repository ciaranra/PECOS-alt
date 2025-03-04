use crate::engines::noise::{NoiseModel, PassThroughNoise};
use pecos_core::types::ShotResult;

use crate::channels::byte_message::ByteMessage;
use crate::engines::{
    ClassicalEngine, ControlEngine, Engine, EngineStage, EngineSystem, QuantumEngine,
};
use crate::errors::QueueError;

/// `HybridEngine` coordinates between classical and quantum components using direct byte messaging
pub struct HybridEngine {
    classical: Box<dyn ClassicalEngine>,
    engine: Box<dyn Engine<Input = ByteMessage, Output = ByteMessage>>,
    // Store the quantum engine separately for potential reconstruction
    quantum_engine: Box<dyn QuantumEngine>,
}

impl HybridEngine {
    #[must_use]
    pub fn new(classical: Box<dyn ClassicalEngine>, quantum: Box<dyn QuantumEngine>) -> Self {
        // Use a pass-through noise model by default
        Self::with_noise(classical, quantum, Box::new(PassThroughNoise))
    }

    #[must_use]
    pub fn with_noise(
        classical: Box<dyn ClassicalEngine>,
        quantum: Box<dyn QuantumEngine>,
        noise_model: Box<dyn NoiseModel>,
    ) -> Self {
        // Store a clone of the quantum engine
        let quantum_clone = quantum.clone_box();

        // Create an EngineSystem with the noise model
        let engine = Box::new(EngineSystem::new(noise_model, quantum));

        Self {
            classical,
            engine,
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
    /// - Generating commands through the classical engine fails.
    /// - Processing commands through the quantum engine fails.
    /// - Handling measurements through the classical engine fails.
    pub fn run_shot(&mut self) -> Result<ShotResult, QueueError> {
        let mut stage = self.classical.start(())?;

        while let EngineStage::NeedsProcessing(command_message) = stage {
            // Process through engine (could be QuantumEngine or EngineSystem)
            let measurement_message = self.engine.process(command_message)?;

            // Continue classical processing with measurements
            stage = self.classical.continue_processing(measurement_message)?;
        }

        match stage {
            EngineStage::Complete(results) => Ok(results),
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }
}
