use crate::config::{LlvmSimConfig, NoiseModelConfig, QuantumEngineType};
use crate::simulation::LlvmSimulation;
use crate::source::LlvmSource;
use hugr_core::Hugr;
use pecos_core::errors::PecosError;
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

    /// Set the source to LLVM IR text (human-readable format).
    ///
    /// Note: The LLVM IR must be properly formatted without indentation,
    /// as LLVM's parser is strict about formatting.
    pub fn llvm_ir(mut self, ir: impl Into<String>) -> Self {
        self.source = Some(LlvmSource::LlvmIr(ir.into()));
        self
    }
    
    /// Set the source to LLVM bitcode (binary format).
    pub fn llvm_bitcode(mut self, bitcode: impl Into<Vec<u8>>) -> Self {
        self.source = Some(LlvmSource::LlvmBitcode(bitcode.into()));
        self
    }

    /// Set the source to LLVM file (auto-detects .ll or .bc extension).
    pub fn llvm_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the source to LLVM IR text file (.ll).
    pub fn llvm_ir_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmIrFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the source to LLVM bitcode file (.bc).
    pub fn llvm_bitcode_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmBitcodeFile(path.as_ref().to_path_buf()));
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

    /// Set the noise model using any type that implements Into<NoiseModelConfig>.
    ///
    /// This provides a consistent API with qasm_sim().
    ///
    /// # Examples
    /// ```
    /// # use pecos_llvm_sim::{llvm_sim, DepolarizingNoise};
    /// # let llvm_ir = "";
    /// // Using noise structs
    /// let sim = llvm_sim()
    ///     .llvm_ir(llvm_ir)
    ///     .noise(DepolarizingNoise { p: 0.01 });
    /// ```
    #[must_use]
    pub fn noise<N: Into<NoiseModelConfig>>(mut self, noise: N) -> Self {
        self.config.noise_model = noise.into();
        self
    }

    /// Set the quantum engine type.
    ///
    /// This provides a consistent API with qasm_sim().
    ///
    /// # Examples
    /// ```
    /// # use pecos_llvm_sim::{llvm_sim, QuantumEngineType};
    /// # let llvm_ir = "";
    /// let sim = llvm_sim()
    ///     .llvm_ir(llvm_ir)
    ///     .quantum_engine(QuantumEngineType::StateVector);
    /// ```
    #[must_use]
    pub fn quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.config.quantum_engine = engine;
        self
    }

    /// Enable or disable keeping temporary files (no-op for compatibility)
    #[must_use]
    pub fn keep_temp_files(self, _keep: bool) -> Self {
        // No-op for compatibility - temp file management is automatic
        self
    }

    /// Enable verbose output
    #[must_use]
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
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
                "No source specified. Use .llvm_ir(), .llvm_bitcode(), .hugr(), or similar method.".to_string(),
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
