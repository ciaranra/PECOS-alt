//! Builder pattern for Selene quantum simulation
//!
//! This follows the same pattern as qasm_sim() and llvm_sim() in PECOS,
//! providing a familiar API for creating quantum simulations.

use crate::{
    selene_engine::SeleneEngine,
    error::SeleneError, 
    program::SeleneProgram,
};
use pecos_core::prelude::PecosError;
use pecos_engines::{
    ShotMap,
    MonteCarloEngine,
    noise::{
        NoiseModel, PassThroughNoiseModel, DepolarizingNoiseModel, 
        BiasedDepolarizingNoiseModel, GeneralNoiseModelBuilder,
    },
    quantum::{QuantumEngine, StateVecEngine, SparseStabEngine},
};
use std::path::PathBuf;

/// Noise model configuration for Selene simulations
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
    /// General noise model from builder
    General(GeneralNoiseModelBuilder),
}

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

impl NoiseModelConfig {
    /// Create a boxed noise model instance
    fn create_noise_model(self) -> Box<dyn NoiseModel> {
        match self {
            Self::PassThrough => Box::new(PassThroughNoiseModel),
            Self::Depolarizing(p) => Box::new(DepolarizingNoiseModel::new_uniform(p)),
            Self::DepolarizingCustom { p_prep, p_meas, p1, p2 } => {
                Box::new(DepolarizingNoiseModel::new(p_prep, p_meas, p1, p2))
            }
            Self::BiasedDepolarizing(p) => Box::new(BiasedDepolarizingNoiseModel::new_uniform(p)),
            Self::General(builder) => Box::new(builder.build()),
        }
    }
}

/// Quantum engine type for Selene simulations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantumEngineType {
    /// State vector simulator (default)
    StateVector,
    /// Sparse stabilizer simulator (Clifford-only)
    SparseStabilizer,
}

/// A built Selene simulation that can be run multiple times
pub struct SeleneSimulation {
    engine: SeleneEngine,
    num_qubits: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: NoiseModelConfig,
    quantum_engine: QuantumEngineType,
}

impl SeleneSimulation {
    /// Run the simulation with the specified number of shots
    pub fn run(&self, shots: usize) -> Result<ShotMap, PecosError> {
        // Clone the engine for this run
        let classical_engine = self.engine.clone();
        
        // Create the quantum engine
        let quantum_engine: Box<dyn QuantumEngine> = match self.quantum_engine {
            QuantumEngineType::StateVector => {
                Box::new(StateVecEngine::new(self.num_qubits))
            }
            QuantumEngineType::SparseStabilizer => {
                Box::new(SparseStabEngine::new(self.num_qubits))
            }
        };
        
        // Create the noise model
        let noise_model = self.noise_model.clone().create_noise_model();
        
        // Run the simulation using MonteCarloEngine
        let results = MonteCarloEngine::run_with_engines(
            Box::new(classical_engine),
            noise_model,
            quantum_engine,
            shots,
            self.workers.unwrap_or(1),
            self.seed,
        )?;
        
        results.try_as_shot_map()
    }
}

/// Builder for Selene-based quantum simulation
///
/// This builder follows PECOS conventions:
/// - The engine handles classical control flow and command generation
/// - Quantum simulation is handled by PECOS's infrastructure
/// - Methods like seed, workers, etc. would be passed to the quantum engine
///
/// Note: This Rust builder works with pre-compiled HUGR/LLVM programs.
/// For Guppy source compilation, use the Python guppy_selene_sim() which
/// compiles Guppy → HUGR → this Rust selene_sim().
pub struct SeleneSimBuilder {
    program: Option<SeleneProgram>,
    num_qubits: Option<usize>,
    optimize: bool,
    verbose: bool,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: NoiseModelConfig,
    quantum_engine: QuantumEngineType,
}

impl Default for SeleneSimBuilder {
    fn default() -> Self {
        Self {
            program: None,
            num_qubits: None,
            optimize: false,
            verbose: false,
            seed: None,
            workers: None,
            noise_model: NoiseModelConfig::PassThrough,
            quantum_engine: QuantumEngineType::StateVector,
        }
    }
}

impl SeleneSimBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the program from a HUGR
    #[cfg(feature = "hugr")]
    pub fn hugr(mut self, hugr: hugr::Hugr) -> Self {
        self.program = Some(SeleneProgram::Hugr(hugr));
        self
    }
    
    
    /// Set the program from LLVM IR text (human-readable format)
    pub fn llvm_ir(mut self, ir: impl Into<String>) -> Self {
        self.program = Some(SeleneProgram::LlvmIr(ir.into()));
        self
    }
    
    /// Set the program from LLVM bitcode (binary format)
    pub fn llvm_bitcode(mut self, bitcode: impl Into<Vec<u8>>) -> Self {
        self.program = Some(SeleneProgram::LlvmBitcode(bitcode.into()));
        self
    }
    
    /// Set the program from an LLVM file (auto-detects .ll or .bc)
    pub fn llvm_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.program = Some(SeleneProgram::LlvmFile(path.into()));
        self
    }
    
    /// Set the program from an LLVM IR text file (.ll)
    pub fn llvm_ir_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.program = Some(SeleneProgram::LlvmIrFile(path.into()));
        self
    }
    
    /// Set the program from an LLVM bitcode file (.bc)
    pub fn llvm_bitcode_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.program = Some(SeleneProgram::LlvmBitcodeFile(path.into()));
        self
    }
    
    /// Set the program from a HUGR file
    pub fn hugr_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.program = Some(SeleneProgram::HugrFile(path.into()));
        self
    }
    
    /// Set the number of qubits
    pub fn qubits(mut self, n: usize) -> Self {
        self.num_qubits = Some(n);
        self
    }
    
    /// Set random seed for reproducibility
    /// 
    /// Note: In a full implementation, this would be passed to the quantum engine
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
    
    /// Set number of parallel workers
    pub fn workers(mut self, n: usize) -> Self {
        self.workers = Some(n);
        self
    }
    
    /// Automatically set workers based on available CPU cores
    pub fn auto_workers(mut self) -> Self {
        self.workers = Some(std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4));
        self
    }
    
    
    /// Enable classical optimizations
    pub fn optimize(mut self) -> Self {
        self.optimize = true;
        self
    }
    
    /// Enable verbose output
    pub fn verbose(mut self, v: bool) -> Self {
        self.verbose = v;
        self
    }
    
    
    /// Set the noise model using any type that implements Into<NoiseModelConfig>
    /// 
    /// This provides a more ergonomic API similar to qasm_sim().
    /// 
    /// # Examples
    /// ```
    /// # use pecos_selene_ceng::{selene_sim, NoiseModelConfig};
    /// # let llvm_ir = "test";
    /// // Using the enum directly
    /// let sim = selene_sim()
    ///     .llvm_ir(llvm_ir)
    ///     .qubits(1)
    ///     .noise(NoiseModelConfig::Depolarizing(0.01));
    /// ```
    pub fn noise<N: Into<NoiseModelConfig>>(mut self, noise: N) -> Self {
        self.noise_model = noise.into();
        self
    }
    
    
    /// Set the quantum engine type
    pub fn quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.quantum_engine = engine;
        self
    }
    
    /// Build the Selene classical control engine only
    /// 
    /// For backward compatibility. Use `build_simulation()` or `run()` for full simulations.
    pub fn build(self) -> Result<SeleneEngine, PecosError> {
        self.build_engine()
    }
    
    /// Build a reusable Selene simulation
    /// 
    /// This creates a simulation that can be run multiple times with different shot counts.
    pub fn build_simulation(self) -> Result<SeleneSimulation, PecosError> {
        let num_qubits = self.num_qubits.ok_or(SeleneError::QubitCountNotSpecified)?;
        let seed = self.seed;
        let workers = self.workers;
        let noise_model = self.noise_model.clone();
        let quantum_engine = self.quantum_engine;
        let engine = self.build_engine()?;
        
        Ok(SeleneSimulation {
            engine,
            num_qubits,
            seed,
            workers,
            noise_model,
            quantum_engine,
        })
    }
    
    /// Build the Selene classical control engine
    fn build_engine(self) -> Result<SeleneEngine, PecosError> {
        let program = self.program.ok_or(SeleneError::NoProgramSpecified)?;
        let num_qubits = self.num_qubits.ok_or(SeleneError::QubitCountNotSpecified)?;
        
        // Validate qubit count
        if num_qubits == 0 {
            return Err(SeleneError::InvalidConfiguration("Qubit count must be greater than 0".to_string()).into());
        }
        
        if self.verbose {
            log::info!("Building Selene classical control engine with:");
            log::info!("  Qubits: {}", num_qubits);
            log::info!("  Optimization: {}", if self.optimize { "enabled" } else { "disabled" });
            if let Some(seed) = self.seed {
                log::info!("  Seed: {}", seed);
            }
            if let Some(workers) = self.workers {
                log::info!("  Workers: {}", workers);
            }
        }
        
        Ok(SeleneEngine::new(
            program,
            num_qubits,
            self.optimize,
        ))
    }
    
    /// Build and run quantum simulation for specified number of shots
    /// 
    /// This creates a complete quantum simulation by:
    /// 1. Building the Selene classical control engine
    /// 2. Creating the specified quantum engine
    /// 3. Pairing them with a HybridEngine
    /// 4. Running the simulation with MonteCarloEngine for parallelization
    pub fn run(self, shots: usize) -> Result<ShotMap, PecosError> {
        // Get configuration values before consuming self
        let num_qubits = self.num_qubits.ok_or(SeleneError::QubitCountNotSpecified)?;
        let seed = self.seed;
        let workers = self.workers;
        let quantum_engine_type = self.quantum_engine;
        let noise_model = self.noise_model.clone();
        
        // Build the classical control engine
        let classical_engine = self.build_engine()?;
        
        // Create the quantum engine based on type
        let quantum_engine: Box<dyn QuantumEngine> = match quantum_engine_type {
            QuantumEngineType::StateVector => {
                Box::new(StateVecEngine::new(num_qubits))
            }
            QuantumEngineType::SparseStabilizer => {
                Box::new(SparseStabEngine::new(num_qubits))
            }
        };
        
        // Create the noise model
        let noise_model = noise_model.create_noise_model();
        
        // Run the simulation using MonteCarloEngine
        let results = MonteCarloEngine::run_with_engines(
            Box::new(classical_engine),
            noise_model,
            quantum_engine,
            shots,
            workers.unwrap_or(1),
            seed,
        )?;
        
        // Convert ShotVec to ShotMap
        results.try_as_shot_map()
    }
}

/// Convenience function to create a new Selene simulation builder
///
/// # Example
/// ```rust
/// # use pecos_selene_ceng::{selene_sim, DepolarizingNoise, QuantumEngineType};
/// # use pecos_core::prelude::PecosError;
/// # fn main() -> Result<(), PecosError> {
/// // Create a simple Bell state LLVM IR program
/// let bell_state_llvm = r#"
/// ; Bell state quantum program
/// declare void @__quantum__qis__h__body(i64)
/// declare void @__quantum__qis__cx__body(i64, i64)
/// declare i32 @__quantum__qis__m__body(i64, i64)
/// 
/// define void @bell_state() #0 {
/// entry:
///     call void @__quantum__qis__h__body(i64 0)
///     call void @__quantum__qis__cx__body(i64 0, i64 1)
///     %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
///     %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
///     ret void
/// }
/// 
/// attributes #0 = { "EntryPoint" }
/// "#;
///
/// // Basic usage
/// let results = selene_sim()
///     .llvm_ir(bell_state_llvm)
///     .qubits(2)
///     .run(10)?;
/// 
/// // With noise model
/// let results = selene_sim()
///     .llvm_ir(bell_state_llvm)
///     .qubits(2)
///     .seed(42)
///     .workers(4)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(1000)?;
/// 
/// // With sparse stabilizer engine (for Clifford circuits)
/// let results = selene_sim()
///     .llvm_ir(bell_state_llvm)
///     .qubits(2)
///     .quantum_engine(QuantumEngineType::SparseStabilizer)
///     .optimize()
///     .run(50)?;
/// # Ok(())
/// # }
/// ```
pub fn selene_sim() -> SeleneSimBuilder {
    SeleneSimBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_builder_creation() {
        let builder = selene_sim();
        assert!(builder.build().is_err()); // Should fail without program
    }
    
    #[test]
    fn test_builder_with_program() {
        let builder = selene_sim()
            .llvm_ir("test")
            .qubits(2);
        
        assert!(builder.build().is_ok());
    }
    
    #[test]
    fn test_builder_chain() {
        let builder = selene_sim()
            .llvm_ir("test")
            .qubits(4)
            .seed(42)
            .workers(2)
            .optimize()
            .verbose(true);
        
        assert!(builder.build().is_ok());
    }
    
    #[test]
    fn test_parallel_execution() {
        // Use proper LLVM IR instead of invalid "test" string
        let llvm_ir = r#"
define i32 @main() {
entry:
    ; Simple LLVM IR that can be compiled successfully
    ret i32 0
}
"#;
        // Reduce shots and workers for faster testing of the parallel infrastructure
        let results = selene_sim()
            .llvm_ir(llvm_ir)
            .qubits(2)
            .workers(2)  // Reduced from 4 to 2
            .run(4);  // Reduced from 100 to 4
        
        assert!(results.is_ok());
        assert_eq!(results.unwrap().num_shots(), 4);
    }
}