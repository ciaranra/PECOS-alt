use crate::channels::byte_message::ByteMessage;
use crate::engines::noise::{DepolarizingNoise, NoiseModel, PassThroughNoise};
use crate::engines::{
    ClassicalEngine, ControlEngine, Engine, EngineStage, EngineSystem, QuantumEngine,
};
use crate::errors::QueueError;
use crate::quantum_system::QuantumSystem;
use crate::shot_results::ShotResult;
use dyn_clone;
use log::debug;

/// `HybridEngine` coordinates between classical and quantum components
///
/// This engine implements the `EngineSystem` trait, using a `ClassicalEngine` as
/// the controller and a `QuantumSystem` as the controlled engine.
pub struct HybridEngine {
    classical_engine: Box<dyn ClassicalEngine>,
    quantum_system: QuantumSystem,
}

impl HybridEngine {
    /// Create a new `HybridEngine` with the given classical and quantum engines
    ///
    /// This uses a pass-through noise model by default.
    #[must_use]
    pub fn new(
        classical_engine: Box<dyn ClassicalEngine>,
        quantum_engine: Box<dyn QuantumEngine>,
    ) -> Self {
        // Use a pass-through noise model by default
        Self::with_noise(classical_engine, quantum_engine, Box::new(PassThroughNoise))
    }

    /// Create a new `HybridEngine` with the given classical engine, quantum engine, and noise model
    #[must_use]
    pub fn with_noise(
        classical_engine: Box<dyn ClassicalEngine>,
        quantum_engine: Box<dyn QuantumEngine>,
        noise_model: Box<dyn NoiseModel>,
    ) -> Self {
        // Create a QuantumSystem with the provided components
        let quantum_system = QuantumSystem::new(noise_model, quantum_engine);

        Self {
            classical_engine,
            quantum_system,
        }
    }

    /// Creates a new `HybridEngine` with the specified classical engine and quantum system
    #[must_use]
    pub fn new_with_quantum_system(
        classical_engine: Box<dyn ClassicalEngine>,
        quantum_system: QuantumSystem,
    ) -> Self {
        Self {
            classical_engine,
            quantum_system,
        }
    }

    /// Create a new `HybridEngine` with the given classical engine and a quantum system with depolarizing noise
    #[must_use]
    pub fn with_depolarizing_noise(
        classical_engine: Box<dyn ClassicalEngine>,
        quantum_engine: Box<dyn QuantumEngine>,
        probability: f64,
    ) -> Self {
        let quantum_system = // Create a QuantumSystem with depolarizing noise
        QuantumSystem::new(
            Box::new(DepolarizingNoise::new_with_options(probability)),
            quantum_engine,
        );

        Self {
            classical_engine,
            quantum_system,
        }
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
        debug!("HybridEngine::reset() being called!");
        // Use as_mut() to get a mutable reference to the inner ClassicalEngine
        (self.classical_engine.as_mut() as &mut dyn ClassicalEngine).reset()?;
        self.quantum_system.reset()
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
        let mut stage = self.classical_engine.start(())?;

        while let EngineStage::NeedsProcessing(command_message) = stage {
            // Process through engine (could be QuantumEngine or EngineSystem)
            let measurement_message = self.quantum_system.process(command_message)?;

            // Continue classical processing with measurements
            stage = self
                .classical_engine
                .continue_processing(measurement_message)?;
        }

        match stage {
            EngineStage::Complete(results) => Ok(results),
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }
}

impl EngineSystem for HybridEngine {
    type Controller = Box<dyn ClassicalEngine>;
    type ControlledEngine = QuantumSystem;
    type Input = (); // Or whatever appropriate input type
    type Output = ShotResult; // Or whatever appropriate output type
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn controller(&self) -> &Self::Controller {
        &self.classical_engine
    }

    fn controller_mut(&mut self) -> &mut Self::Controller {
        &mut self.classical_engine
    }

    fn engine(&self) -> &Self::ControlledEngine {
        &self.quantum_system
    }

    fn engine_mut(&mut self) -> &mut Self::ControlledEngine {
        &mut self.quantum_system
    }
}

impl Clone for HybridEngine {
    fn clone(&self) -> Self {
        HybridEngine {
            classical_engine: dyn_clone::clone_box(&*self.classical_engine),
            quantum_system: self.quantum_system.clone(),
        }
    }
}
