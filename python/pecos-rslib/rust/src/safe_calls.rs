/// Macro to ensure correct parameter ordering for `run_sim` calls
#[macro_export]
macro_rules! run_sim_validated {
    (
        engine: $engine:expr,
        shots: $shots:expr,
        seed: $seed:expr,
        workers: $workers:expr,
        noise: $noise:expr,
        quantum: $quantum:expr
    ) => {{
        // This macro ensures parameters are in the correct order
        // and makes it impossible to mix them up
        pecos_engines::run_sim($engine, $shots, $seed, $workers, $noise, $quantum)
    }};
}

/// Type-safe wrapper for `run_sim` parameters
pub struct SimParams {
    pub classical_engine: Box<dyn pecos_engines::ClassicalEngine>,
    pub shots: usize,
    pub seed: Option<u64>,
    pub workers: Option<usize>,
    pub noise_model: Option<Box<dyn pecos_engines::NoiseModel>>,
    pub quantum_engine: Option<Box<dyn pecos_engines::QuantumEngine>>,
}

impl SimParams {
    /// Create a new `SimParams` with required fields
    pub fn new(classical_engine: Box<dyn pecos_engines::ClassicalEngine>, shots: usize) -> Self {
        Self {
            classical_engine,
            shots,
            seed: None,
            workers: None,
            noise_model: None,
            quantum_engine: None,
        }
    }

    /// Set the seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the workers
    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = Some(workers);
        self
    }

    /// Set the noise model
    pub fn with_noise_model(mut self, model: Box<dyn pecos_engines::NoiseModel>) -> Self {
        self.noise_model = Some(model);
        self
    }

    /// Run the simulation with these parameters
    pub fn run(
        self,
    ) -> Result<pecos_engines::shot_results::ShotVec, pecos_core::errors::PecosError> {
        pecos_engines::run_sim(
            self.classical_engine,
            self.shots,
            self.seed,
            self.workers,
            self.noise_model,
            self.quantum_engine,
        )
    }
}
