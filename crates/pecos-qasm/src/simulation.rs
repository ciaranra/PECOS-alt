//! Builder-based simulation runner for QASM
//!
//! This module provides a fluent builder API for running QASM simulations
//! with support for various noise models and quantum engines.
//!
//! The implementation now uses the unified simulation API internally while
//! maintaining backward compatibility with the existing interface.
//!
//! ## Implementation Note
//!
//! This module is now a thin wrapper around the unified simulation API
//! (`qasm_engine().to_sim()`). All configuration options are passed through
//! to the underlying unified builders.

use crate::engine::QASMEngine;
use crate::unified_engine_builder::{qasm_engine, QasmEngineBuilder};
use pecos_core::errors::PecosError;
use pecos_engines::noise::{
    BiasedDepolarizingNoiseModelBuilder, DepolarizingNoiseModelBuilder, GeneralNoiseModelBuilder,
    NoiseModel, PassThroughNoiseModel, PassThroughNoiseModelBuilder,
};
use pecos_engines::quantum::{QuantumEngine, SparseStabEngine, StateVecEngine};
use pecos_engines::sim_builder::{
    QuantumEngineType as UnifiedQuantumEngineType, Simulation,
};
use pecos_engines::shot_results::ShotVec;
use pecos_engines::ClassicalControlEngineBuilder;

/// Noise model configuration
///
/// This enum holds builders for different noise models.
#[derive(Debug, Clone)]
pub enum NoiseModelType {
    /// No noise (ideal simulation)
    PassThrough(Box<PassThroughNoiseModelBuilder>),
    /// Depolarizing noise model
    Depolarizing(Box<DepolarizingNoiseModelBuilder>),
    /// Biased depolarizing noise model
    BiasedDepolarizing(Box<BiasedDepolarizingNoiseModelBuilder>),
    /// General noise model
    General(Box<GeneralNoiseModelBuilder>),
}

impl NoiseModelType {
    /// Create a boxed noise model instance
    #[must_use]
    pub fn create_noise_model(self) -> Box<dyn NoiseModel> {
        match self {
            Self::PassThrough(builder) => Box::new(builder.build()),
            Self::Depolarizing(builder) => Box::new(builder.build()),
            Self::BiasedDepolarizing(builder) => Box::new(builder.build()),
            Self::General(builder) => Box::new(builder.build()),
        }
    }
}

impl Default for NoiseModelType {
    fn default() -> Self {
        Self::PassThrough(Box::new(PassThroughNoiseModel::builder()))
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


// Implement From traits for converting noise builders to NoiseModelType

impl From<PassThroughNoiseModelBuilder> for NoiseModelType {
    fn from(builder: PassThroughNoiseModelBuilder) -> Self {
        NoiseModelType::PassThrough(Box::new(builder))
    }
}

impl From<DepolarizingNoiseModelBuilder> for NoiseModelType {
    fn from(builder: DepolarizingNoiseModelBuilder) -> Self {
        NoiseModelType::Depolarizing(Box::new(builder))
    }
}

impl From<BiasedDepolarizingNoiseModelBuilder> for NoiseModelType {
    fn from(builder: BiasedDepolarizingNoiseModelBuilder) -> Self {
        NoiseModelType::BiasedDepolarizing(Box::new(builder))
    }
}

impl From<GeneralNoiseModelBuilder> for NoiseModelType {
    fn from(builder: GeneralNoiseModelBuilder) -> Self {
        NoiseModelType::General(Box::new(builder))
    }
}

/// A built QASM simulation that can be run multiple times
pub struct QasmSimulation {
    simulation: Simulation<QASMEngine>,
}

impl QasmSimulation {
    /// Run the simulation with the specified number of shots
    ///
    /// This can be called multiple times to run the same simulation
    /// with different numbers of shots.
    ///
    /// # Errors
    ///
    /// Returns an error if simulation fails.
    pub fn run(&self, shots: usize) -> Result<ShotVec, PecosError> {
        self.simulation.run(shots)
    }
}

/// Builder for configuring and running QASM simulations
///
/// This builder now wraps the unified API internally while maintaining
/// backward compatibility with the existing interface.
#[derive(Debug)]
pub struct QasmSimulationBuilder {
    engine_builder: QasmEngineBuilder,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<NoiseModelType>,
    quantum_engine_type: Option<QuantumEngineType>,
}

impl QasmSimulationBuilder {
    /// Create a new builder from QASM source
    #[must_use]
    pub fn new(qasm: impl Into<String>) -> Self {
        Self {
            engine_builder: qasm_engine().qasm(qasm),
            seed: None,
            workers: None,
            noise_model: None,
            quantum_engine_type: None,
        }
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

    /// Use automatic worker count based on available CPUs
    #[must_use]
    pub fn auto_workers(mut self) -> Self {
        self.workers = Some(0); // 0 means auto
        self
    }

    /// Set the noise model
    #[must_use]
    pub fn noise<N>(mut self, noise: N) -> Self
    where
        N: Into<NoiseModelType>,
    {
        self.noise_model = Some(noise.into());
        self
    }

    /// Set the quantum engine type
    #[must_use]
    pub fn quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.quantum_engine_type = Some(engine);
        self
    }


    /// Set the path to a WebAssembly file (.wasm or .wat) for foreign function calls
    #[cfg(feature = "wasm")]
    #[must_use]
    pub fn wasm(mut self, wasm_path: impl Into<String>) -> Self {
        self.engine_builder = self.engine_builder.wasm(wasm_path);
        self
    }

    /// Build the simulation (for reusable execution)
    ///
    /// # Errors
    ///
    /// Returns an error if the QASM cannot be parsed.
    pub fn build(self) -> Result<QasmSimulation, PecosError> {
        // Convert to SimBuilder through the unified API
        let mut sim_builder = self.engine_builder.to_sim();

        // Apply seed if specified
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }

        // Apply workers configuration
        match self.workers {
            Some(0) | None => {
                // Auto-workers or unspecified (default is 1)
                if self.workers == Some(0) {
                    sim_builder = sim_builder.auto_workers();
                }
            }
            Some(n) => {
                sim_builder = sim_builder.workers(n);
            }
        }

        // Apply noise model if specified
        if let Some(noise) = self.noise_model {
            sim_builder = match noise {
                NoiseModelType::PassThrough(builder) => sim_builder.noise(*builder),
                NoiseModelType::Depolarizing(builder) => sim_builder.noise(*builder),
                NoiseModelType::BiasedDepolarizing(builder) => sim_builder.noise(*builder),
                NoiseModelType::General(builder) => sim_builder.noise(*builder),
            };
        }

        // Apply quantum engine type
        if let Some(engine_type) = self.quantum_engine_type {
            let unified_type = match engine_type {
                QuantumEngineType::StateVector => UnifiedQuantumEngineType::StateVector,
                QuantumEngineType::SparseStabilizer => UnifiedQuantumEngineType::SparseStabilizer,
            };
            sim_builder = sim_builder.quantum_engine(unified_type);
        }

        // Build the simulation
        let simulation = sim_builder.build()?;

        Ok(QasmSimulation { simulation })
    }

    /// Run the simulation directly with the specified number of shots
    ///
    /// # Errors
    ///
    /// Returns an error if simulation fails.
    pub fn run(self, shots: usize) -> Result<ShotVec, PecosError> {
        let sim = self.build()?;
        sim.run(shots)
    }
}

/// Create a new QASM simulation builder
///
/// This is the primary entry point for running QASM simulations.
///
/// # Example
///
/// ```
/// use pecos_qasm::prelude::*;
///
/// let qasm = r#"
///     OPENQASM 2.0;
///     include "qelib1.inc";
///     qreg q[2];
///     creg c[2];
///     h q[0];
///     cx q[0], q[1];
///     measure q -> c;
/// "#;
///
/// // Run with default settings (no noise)
/// let results = qasm_sim(qasm).run(100).unwrap();
///
/// // Run with noise
/// let noise = GeneralNoiseModel::builder()
///     .with_p1_probability(0.001)
///     .with_p2_probability(0.01);
///
/// let results = qasm_sim(qasm)
///     .seed(42)
///     .noise(noise)
///     .run(1000)
///     .unwrap();
/// ```
///
/// ## Full configuration
/// ```
/// # use pecos_qasm::simulation::{qasm_sim, QuantumEngineType};
/// # use pecos_qasm::prelude::NoiseConfig;
/// # let qasm = "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[2]; creg c[2]; h q[0]; cx q[0], q[1]; measure q -> c;";
/// let sim = qasm_sim(qasm)
///     .seed(42)
///     .auto_workers()
///     .quantum_engine(QuantumEngineType::StateVector)
///     .noise(NoiseConfig::BiasedDepolarizingNoise { p: 0.01 })
///     .build()
///     .unwrap();
///
/// // Run multiple simulations
/// for shots in [100, 500, 1000] {
///     let results = sim.run(shots).unwrap();
///     println!("Got {} results", results.len());
/// }
/// ```
///
/// # Performance Tips
///
/// 1. **Build once, run multiple times**: Parse QASM once and reuse the simulation
///    for multiple runs or parameter sweeps.
/// 2. **Use `auto_workers()`** for CPU-bound simulations with many shots to utilize all available cores.
/// 3. **Choose the right engine**:
///    - `SparseStabilizer` for Clifford-only circuits (exponentially faster)
///    - `StateVector` for circuits with non-Clifford gates
/// 4. **Batch similar simulations**: Use the same noise model and engine settings when possible
///    to reduce overhead.
#[must_use]
pub fn qasm_sim(qasm: impl Into<String>) -> QasmSimulationBuilder {
    QasmSimulationBuilder::new(qasm)
}

