use pecos_engines::{
    noise::{
        BiasedDepolarizingNoiseModel, DepolarizingNoiseModel, GeneralNoiseModelBuilder, NoiseModel,
        PassThroughNoiseModel,
    },
    quantum::{QuantumEngine, SparseStabEngine, StateVecEngine},
};

/// Noise model configuration types
#[derive(Debug, Clone)]
pub enum NoiseModelConfig {
    /// No noise (ideal simulation)
    PassThrough,
    /// Standard depolarizing noise with uniform probability
    Depolarizing(f64),
    /// Custom depolarizing noise with different probabilities
    DepolarizingCustom {
        p_prep: f64,
        p_meas: f64,
        p1: f64,
        p2: f64,
    },
    /// Biased depolarizing noise
    BiasedDepolarizing(f64),
    /// General noise model
    General(GeneralNoiseModelBuilder),
}

impl NoiseModelConfig {
    /// Create a boxed noise model instance
    #[must_use]
    pub fn create_noise_model(self) -> Box<dyn NoiseModel> {
        match self {
            Self::PassThrough => Box::new(PassThroughNoiseModel),
            Self::Depolarizing(p) => Box::new(DepolarizingNoiseModel::new_uniform(p)),
            Self::DepolarizingCustom {
                p_prep,
                p_meas,
                p1,
                p2,
            } => Box::new(DepolarizingNoiseModel::new(p_prep, p_meas, p1, p2)),
            Self::BiasedDepolarizing(p) => Box::new(BiasedDepolarizingNoiseModel::new_uniform(p)),
            Self::General(builder) => Box::new(builder.build()),
        }
    }
}

/// Available quantum simulation engines
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantumEngineType {
    /// State vector simulator (full quantum state)
    StateVector,
    /// Sparse stabilizer simulator (efficient for Clifford circuits)
    SparseStabilizer,
}

impl QuantumEngineType {
    /// Create a boxed quantum engine instance
    #[must_use]
    pub fn create_quantum_engine(self, num_qubits: usize) -> Box<dyn QuantumEngine> {
        match self {
            Self::StateVector => Box::new(StateVecEngine::new(num_qubits)),
            Self::SparseStabilizer => Box::new(SparseStabEngine::new(num_qubits)),
        }
    }

    /// Create a boxed quantum engine instance with a specific seed
    #[must_use]
    pub fn create_quantum_engine_with_seed(
        self,
        num_qubits: usize,
        seed: u64,
    ) -> Box<dyn QuantumEngine> {
        match self {
            Self::StateVector => Box::new(StateVecEngine::with_seed(num_qubits, seed)),
            Self::SparseStabilizer => Box::new(SparseStabEngine::with_seed(num_qubits, seed)),
        }
    }
}

/// Configuration for LLVM simulation
#[derive(Debug, Clone)]
pub struct LlvmSimConfig {
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Number of worker threads (default: 1)
    pub workers: usize,
    /// Noise model configuration
    pub noise_model: NoiseModelConfig,
    /// Quantum engine type
    pub quantum_engine: QuantumEngineType,
    /// Maximum number of qubits allowed for allocation
    pub max_qubits: Option<usize>,
}

impl Default for LlvmSimConfig {
    fn default() -> Self {
        Self {
            seed: None,
            workers: 1,
            noise_model: NoiseModelConfig::PassThrough,
            quantum_engine: QuantumEngineType::StateVector,
            max_qubits: None,
        }
    }
}
