use crate::config::{LlvmSimConfig, NoiseModelConfig, QuantumEngineType};
use crate::simulation::LlvmSimulation;
use crate::source::LlvmSource;
use hugr_core::Hugr;
use pecos_core::errors::PecosError;
use pecos_engines::noise::GeneralNoiseModelBuilder;
use std::path::Path;

/// Builder for LLVM-based quantum simulations.
///
/// Provides a fluent API for configuring and creating simulations from various input formats.
#[derive(Debug, Clone, Default)]
pub struct LlvmSim {
    source: Option<LlvmSource>,
    config: LlvmSimConfig,
}

impl LlvmSim {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source to LLVM IR string.
    ///
    /// Note: The LLVM IR must be properly formatted without indentation,
    /// as LLVM's parser is strict about formatting.
    pub fn llvm(mut self, ir: impl Into<String>) -> Self {
        self.source = Some(LlvmSource::LlvmIr(ir.into()));
        self
    }

    /// Set the source to LLVM IR file.
    pub fn llvm_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set the source to HUGR.
    pub fn hugr(mut self, hugr: Hugr) -> Self {
        self.source = Some(LlvmSource::Hugr(Box::new(hugr)));
        self
    }

    /// Set the source to HUGR bytes.
    #[must_use]
    pub fn hugr_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.source = Some(LlvmSource::HugrBytes(bytes));
        self
    }

    /// Set the source to HUGR file.
    pub fn hugr_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::HugrFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set the random seed for reproducibility.
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self
    }

    /// Set the number of worker threads for parallel execution.
    #[must_use]
    pub fn workers(mut self, workers: usize) -> Self {
        self.config.workers = workers;
        self
    }

    /// Automatically set workers based on available CPU cores.
    pub fn auto_workers(mut self) -> Self {
        self.config.workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        self
    }

    /// Use no noise model (ideal simulation).
    #[must_use]
    pub fn with_no_noise(mut self) -> Self {
        self.config.noise_model = NoiseModelConfig::PassThrough;
        self
    }

    /// Use depolarizing noise with uniform probability.
    #[must_use]
    pub fn with_depolarizing_noise(mut self, p: f64) -> Self {
        self.config.noise_model = NoiseModelConfig::Depolarizing(p);
        self
    }

    /// Use custom depolarizing noise with different probabilities.
    #[must_use]
    pub fn with_custom_depolarizing_noise(
        mut self,
        p_prep: f64,
        p_meas: f64,
        p1: f64,
        p2: f64,
    ) -> Self {
        self.config.noise_model = NoiseModelConfig::DepolarizingCustom {
            p_prep,
            p_meas,
            p1,
            p2,
        };
        self
    }

    /// Use biased depolarizing noise.
    #[must_use]
    pub fn with_biased_depolarizing_noise(mut self, p: f64) -> Self {
        self.config.noise_model = NoiseModelConfig::BiasedDepolarizing(p);
        self
    }

    /// Use a general noise model.
    #[must_use]
    pub fn with_general_noise(mut self, builder: GeneralNoiseModelBuilder) -> Self {
        self.config.noise_model = NoiseModelConfig::General(builder);
        self
    }

    /// Use custom noise model configuration.
    #[must_use]
    pub fn with_noise_model(mut self, noise_model: NoiseModelConfig) -> Self {
        self.config.noise_model = noise_model;
        self
    }

    /// Use state vector quantum engine (default).
    #[must_use]
    pub fn with_state_vector_engine(mut self) -> Self {
        self.config.quantum_engine = QuantumEngineType::StateVector;
        self
    }

    /// Use sparse stabilizer quantum engine.
    #[must_use]
    pub fn with_sparse_stabilizer_engine(mut self) -> Self {
        self.config.quantum_engine = QuantumEngineType::SparseStabilizer;
        self
    }

    /// Use custom quantum engine type.
    #[must_use]
    pub fn with_quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.config.quantum_engine = engine;
        self
    }

    /// Enable or disable keeping temporary files (no-op for compatibility)
    #[must_use]
    pub fn keep_temp_files(self, _keep: bool) -> Self {
        // No-op for compatibility - temp file management is automatic
        self
    }

    /// Enable verbose output (no-op for compatibility)
    #[must_use]
    pub fn verbose(self, _verbose: bool) -> Self {
        // No-op for compatibility - use log crate for verbose output
        self
    }

    /// Set maximum number of qubits allowed for allocation.
    ///
    /// This enforces a limit on dynamic qubit allocation to prevent
    /// out-of-memory issues with exponentially scaling state vectors.
    #[must_use]
    pub fn max_qubits(mut self, max_qubits: usize) -> Self {
        self.config.max_qubits = Some(max_qubits);
        self
    }

    /// Enable debug output (no-op for compatibility)
    #[must_use]
    pub fn debug(self, _debug: bool) -> Self {
        // No-op for compatibility - use log crate for debug output
        self
    }

    /// Build the simulation.
    ///
    /// This compiles the input to LLVM IR if needed and creates the simulation engine.
    pub fn build(self) -> Result<LlvmSimulation, PecosError> {
        // Get source or error
        let source = self.source.ok_or_else(|| {
            PecosError::Input(
                "No source specified. Use .llvm(), .hugr(), or similar method.".to_string(),
            )
        })?;

        // Convert source to LLVM IR
        let llvm_ir = source.to_llvm_ir()?;

        // Create the simulation
        LlvmSimulation::new(llvm_ir, self.config)
    }

    /// Run the simulation directly without building first.
    ///
    /// This is a convenience method that builds and runs the simulation.
    pub fn run(
        self,
        shots: usize,
    ) -> Result<std::collections::HashMap<String, Vec<i64>>, PecosError> {
        let mut sim = self.build()?;
        sim.run(shots)
    }
}
