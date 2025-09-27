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
            Self::PassThrough => Box::new(PassThroughNoiseModel::new()),
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
// Convenience noise configuration structs for ergonomic API

/// No noise configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassThroughNoise;

/// Standard depolarizing noise configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepolarizingNoise {
    /// Uniform error probability for all operations
    pub p: f64,
}

/// Custom depolarizing noise configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepolarizingCustomNoise {
    /// State preparation error probability
    pub p_prep: f64,
    /// Measurement error probability
    pub p_meas: f64,
    /// Single-qubit gate error probability
    pub p1: f64,
    /// Two-qubit gate error probability
    pub p2: f64,
}

/// Biased depolarizing noise configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiasedDepolarizingNoise {
    /// Uniform probability for all operations
    pub p: f64,
}

// Implement From traits for converting noise configs to NoiseModelConfig

impl From<PassThroughNoise> for NoiseModelConfig {
    fn from(_: PassThroughNoise) -> Self {
        NoiseModelConfig::PassThrough
    }
}

impl From<DepolarizingNoise> for NoiseModelConfig {
    fn from(noise: DepolarizingNoise) -> Self {
        NoiseModelConfig::Depolarizing(noise.p)
    }
}

impl From<DepolarizingCustomNoise> for NoiseModelConfig {
    fn from(noise: DepolarizingCustomNoise) -> Self {
        NoiseModelConfig::DepolarizingCustom {
            p_prep: noise.p_prep,
            p_meas: noise.p_meas,
            p1: noise.p1,
            p2: noise.p2,
        }
    }
}

impl From<BiasedDepolarizingNoise> for NoiseModelConfig {
    fn from(noise: BiasedDepolarizingNoise) -> Self {
        NoiseModelConfig::BiasedDepolarizing(noise.p)
    }
}

impl From<GeneralNoiseModelBuilder> for NoiseModelConfig {
    fn from(builder: GeneralNoiseModelBuilder) -> Self {
        NoiseModelConfig::General(builder)
    }
}

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
    /// Number of qubits for the quantum engine and maximum allowed for allocation
    pub num_qubits: Option<usize>,
    /// Enable verbose output
    pub verbose: bool,
}

impl Default for LlvmSimConfig {
    fn default() -> Self {
        Self {
            seed: None,
            workers: 1,
            noise_model: NoiseModelConfig::PassThrough,
            quantum_engine: QuantumEngineType::StateVector,
            num_qubits: None,
            verbose: false,
        }
    }
}
