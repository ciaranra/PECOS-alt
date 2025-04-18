use crate::byte_message::ByteMessage;
use crate::engines::noise::{DepolarizingNoise, NoiseModel, PassThroughNoise};
use crate::engines::{
    ClassicalEngine, ControlEngine, Engine, EngineStage, EngineSystem, QuantumEngine,
};
use crate::errors::QueueError;
use crate::quantum_system::QuantumSystem;
use crate::shot_results::ShotResult;
use dyn_clone;
use log::debug;
use pecos_core::sims_rngs::rng_manageable::derive_seed;

/// `HybridEngine` coordinates between classical and quantum components
///
/// This engine implements the `EngineSystem` trait, using a `ClassicalEngine` as
/// the controller and a `QuantumSystem` as the controlled engine.
pub struct HybridEngine {
    pub classical_engine: Box<dyn ClassicalEngine>,
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

    /// Set a specific seed for all components of the `HybridEngine`
    ///
    /// This method sets different but deterministic seeds for each component:
    /// - Classical engine (if it implements a seed setting method)
    /// - Quantum system (which further sets seeds for both the quantum engine and noise model)
    ///
    /// # Arguments
    /// * `seed` - Base seed value for random number generators
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns a `QueueError` if setting the seed fails for any component
    pub fn set_seed(&mut self, seed: u64) -> Result<(), QueueError> {
        // Derive seeds for each component
        let classical_seed = derive_seed(seed, "classical_engine");
        let quantum_seed = derive_seed(seed, "quantum_system");

        // Set seed for quantum system (this sets seeds for both quantum engine and noise model)
        self.quantum_system.set_seed(quantum_seed)?;

        // Set seed for classical engine
        self.classical_engine.set_seed(classical_seed)?;

        Ok(())
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
        // Use the fully qualified path to disambiguate which reset to call
        ClassicalEngine::reset(&mut *self.classical_engine)?;
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
        debug!(
            "HybridEngine::run_shot() starting - Thread {:?}",
            std::thread::current().id()
        );
        let mut stage = self.classical_engine.start(())?;

        let mut iteration_count = 0;
        while let EngineStage::NeedsProcessing(command_message) = stage {
            iteration_count += 1;
            debug!(
                "HybridEngine::run_shot() iteration {} - Thread {:?}",
                iteration_count,
                std::thread::current().id()
            );

            // Process through engine (could be QuantumEngine or EngineSystem)
            let measurement_message = self.quantum_system.process(command_message)?;

            // Continue classical processing with measurements
            stage = self
                .classical_engine
                .continue_processing(measurement_message)?;
        }

        match stage {
            EngineStage::Complete(results) => {
                debug!(
                    "HybridEngine::run_shot() completed after {} iterations with result: {:?} - Thread {:?}",
                    iteration_count,
                    results.combined_result,
                    std::thread::current().id()
                );
                Ok(results)
            }
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }
}

impl Engine for HybridEngine {
    type Input = ();
    type Output = ShotResult;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        // Delegate to process_as_system for standard implementation
        self.process_as_system(input)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // Reset both controller and engine components by using fully qualified path
        ClassicalEngine::reset(&mut *self.classical_engine)?;
        self.quantum_system.reset()
    }
}

impl EngineSystem for HybridEngine {
    type Controller = Box<dyn ClassicalEngine>;
    type ControlledEngine = QuantumSystem;
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
