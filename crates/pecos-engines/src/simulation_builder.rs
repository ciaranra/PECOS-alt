use crate::noise::PassThroughNoiseModel;
use crate::quantum;
use crate::shot_results::ShotVec;
use crate::{ClassicalEngine, MonteCarloEngine, NoiseModel, QuantumEngine};
use pecos_core::errors::PecosError;

/// Builder for creating and running simulations with compile-time safety
pub struct SimulationBuilder {
    classical_engine: Option<Box<dyn ClassicalEngine>>,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
}

impl Default for SimulationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulationBuilder {
    /// Create a new simulation builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            classical_engine: None,
            shots: 1,
            seed: None,
            workers: None,
            noise_model: None,
            quantum_engine: None,
        }
    }

    /// Set the classical engine (required)
    #[must_use]
    pub fn classical_engine(mut self, engine: Box<dyn ClassicalEngine>) -> Self {
        self.classical_engine = Some(engine);
        self
    }

    /// Set the number of shots
    #[must_use]
    pub fn shots(mut self, shots: usize) -> Self {
        self.shots = shots;
        self
    }

    /// Set the random seed
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the number of workers
    #[must_use]
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = Some(workers);
        self
    }

    /// Set the noise model
    #[must_use]
    pub fn noise_model(mut self, model: Box<dyn NoiseModel>) -> Self {
        self.noise_model = Some(model);
        self
    }

    /// Set the quantum engine
    #[must_use]
    pub fn quantum_engine(mut self, engine: Box<dyn QuantumEngine>) -> Self {
        self.quantum_engine = Some(engine);
        self
    }

    /// Build and run the simulation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The classical engine is not set
    /// - The simulation execution fails
    pub fn run(self) -> Result<ShotVec, PecosError> {
        let classical_engine = self
            .classical_engine
            .ok_or_else(|| PecosError::Input("Classical engine is required".to_string()))?;

        let num_qubits = classical_engine.num_qubits();
        let noise_model = self
            .noise_model
            .unwrap_or_else(|| Box::new(PassThroughNoiseModel));

        let quantum_engine = self
            .quantum_engine
            .unwrap_or_else(|| Box::new(quantum::StateVecEngine::new(num_qubits)));

        // Use MonteCarloEngine instead of HybridEngine
        MonteCarloEngine::run_with_engines(
            classical_engine,
            noise_model,
            quantum_engine,
            self.shots,
            self.workers.unwrap_or(1),
            self.seed,
        )
    }
}

/// Convenience function that uses the builder internally
///
/// # Errors
///
/// Returns an error if:
/// - The classical engine is not set
/// - The simulation execution fails
pub fn run_sim_safe(
    classical_engine: Box<dyn ClassicalEngine>,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
) -> Result<ShotVec, PecosError> {
    let mut builder = SimulationBuilder::new()
        .classical_engine(classical_engine)
        .shots(shots);

    if let Some(s) = seed {
        builder = builder.seed(s);
    }
    if let Some(w) = workers {
        builder = builder.workers(w);
    }
    if let Some(nm) = noise_model {
        builder = builder.noise_model(nm);
    }
    if let Some(qe) = quantum_engine {
        builder = builder.quantum_engine(qe);
    }

    builder.run()
}
