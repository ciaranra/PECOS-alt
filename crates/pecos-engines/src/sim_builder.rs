//! Unified simulation builder for all engine types
//!
//! This module provides the `SimBuilder` struct that handles common simulation
//! configuration (seed, workers, noise, quantum engine) for any classical control engine.

use crate::engine_builder::ClassicalControlEngineBuilder;
use crate::noise::{
    NoiseModel, PassThroughNoiseModel, PassThroughNoiseModelBuilder,
    DepolarizingNoiseModel, DepolarizingNoiseModelBuilder,
    BiasedDepolarizingNoiseModel, BiasedDepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder,
};
use crate::quantum::{QuantumEngine, SparseStabEngine, StateVecEngine};
use crate::shot_results::ShotVec;
use crate::{ClassicalControlEngine, MonteCarloEngine};
use pecos_core::errors::PecosError;
use std::collections::HashMap;

/// Quantum engine type selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantumEngineType {
    /// State vector simulator (full quantum state)
    StateVector,
    /// Sparse stabilizer simulator (efficient for Clifford circuits)
    SparseStabilizer,
}

impl Default for QuantumEngineType {
    fn default() -> Self {
        Self::SparseStabilizer
    }
}

/// Configuration for simulations
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Number of worker threads
    pub workers: usize,
    /// Quantum engine type
    pub quantum_engine: QuantumEngineType,
    /// Maximum number of qubits allowed
    pub max_qubits: Option<usize>,
    /// Verbose output
    pub verbose: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            seed: None,
            workers: 1,
            quantum_engine: QuantumEngineType::default(),
            max_qubits: None,
            verbose: false,
        }
    }
}

/// A built simulation ready to run
pub struct Simulation<E: ClassicalControlEngine> {
    engine: E,
    config: SimConfig,
    noise_model: Box<dyn NoiseModel>,
}

impl<E: ClassicalControlEngine + Clone + 'static> Simulation<E> {
    /// Run the simulation for the specified number of shots
    ///
    /// If a seed was specified during building, it will be used for each run,
    /// producing identical results. Use `run_with_seed` for different results.
    pub fn run(&self, shots: usize) -> Result<ShotVec, PecosError> {
        self.run_with_seed(shots, self.config.seed)
    }
    
    /// Run the simulation with a specific seed (or None for random)
    ///
    /// This allows overriding the seed configured during building, which is
    /// useful for the reusable simulation pattern when you want different
    /// results from each run.
    ///
    /// # Examples
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder};
    /// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
    /// # 
    /// # struct MyEngineBuilder;
    /// # 
    /// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
    /// #     type Engine = ExternalClassicalEngine;
    /// #     
    /// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
    /// #         Ok(ExternalClassicalEngine::new())
    /// #     }
    /// # }
    /// # 
    /// # let engine = MyEngineBuilder;
    /// let sim = engine.to_sim().build()?;
    /// 
    /// // Different seed each time for different results
    /// let r1 = sim.run_with_seed(1000, Some(42))?;
    /// let r2 = sim.run_with_seed(1000, Some(43))?;
    /// let r3 = sim.run_with_seed(1000, None)?;  // Random
    /// # Ok(())
    /// # }
    /// ```
    pub fn run_with_seed(&self, shots: usize, seed: Option<u64>) -> Result<ShotVec, PecosError> {
        let num_qubits = self.engine.num_qubits();
        
        // Create quantum engine based on config
        let quantum_engine: Box<dyn QuantumEngine> = match self.config.quantum_engine {
            QuantumEngineType::StateVector => {
                if let Some(s) = seed {
                    Box::new(StateVecEngine::with_seed(num_qubits, s))
                } else {
                    Box::new(StateVecEngine::new(num_qubits))
                }
            }
            QuantumEngineType::SparseStabilizer => {
                if let Some(s) = seed {
                    Box::new(SparseStabEngine::with_seed(num_qubits, s))
                } else {
                    Box::new(SparseStabEngine::new(num_qubits))
                }
            }
        };

        // Run using MonteCarloEngine
        MonteCarloEngine::run_with_engines(
            Box::new(self.engine.clone()),
            self.noise_model.clone(),
            quantum_engine,
            shots,
            self.config.workers,
            seed,
        )
    }

    /// Get statistics about the simulation
    pub fn stats(&self) -> (usize, usize) {
        (self.engine.num_qubits(), self.config.workers)
    }
}

/// Builder for unified simulations
///
/// The unified API returns `ShotVec` as the standard result format because:
/// - It preserves all shot information
/// - Can be converted to `ShotMap` via `try_as_shot_map()`
/// - Can be converted to columnar format via `shots_to_columnar()`
/// 
/// This provides compatibility with all existing PECOS result formats.
///
/// # Reusable Simulations
///
/// The builder pattern supports two usage modes:
///
/// ## One-shot execution
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder};
/// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
/// # 
/// # // Example engine builder that builds ExternalClassicalEngine
/// # struct MyEngineBuilder;
/// # 
/// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
/// #     type Engine = ExternalClassicalEngine;
/// #     
/// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
/// #         Ok(ExternalClassicalEngine::new())
/// #     }
/// # }
/// # 
/// # let engine = MyEngineBuilder;
/// let results = engine.to_sim().seed(42).run(1000)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Build once, run multiple times
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder, DepolarizingNoise};
/// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
/// # 
/// # struct MyEngineBuilder;
/// # 
/// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
/// #     type Engine = ExternalClassicalEngine;
/// #     
/// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
/// #         Ok(ExternalClassicalEngine::new())
/// #     }
/// # }
/// # 
/// # let engine = MyEngineBuilder;
/// // Build a reusable simulation
/// let sim = engine.to_sim()
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .build()?;
///
/// // Run multiple times with different shot counts
/// let results_100 = sim.run(100)?;
/// let results_1000 = sim.run(1000)?;
/// let results_10000 = sim.run(10000)?;
/// # Ok(())
/// # }
/// ```
///
/// **Important Note on Seeding**: When using a fixed seed with the reusable pattern,
/// each `run()` call will produce identical results (for the same number of shots).
/// This is because the seed is used to initialize the RNG at the start of each run.
/// If you need different results from each run, you can:
/// - Use `run_with_seed()` to override the seed for each run
/// - Don't specify a seed during building (uses system randomness)
/// - Build a new simulation for each run with different seeds
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder};
/// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
/// # 
/// # struct MyEngineBuilder;
/// # 
/// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
/// #     type Engine = ExternalClassicalEngine;
/// #     
/// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
/// #         Ok(ExternalClassicalEngine::new())
/// #     }
/// # }
/// # 
/// # let engine = MyEngineBuilder;
/// // Option 1: Override seed per run
/// let sim = engine.to_sim().build()?;
/// let r1 = sim.run_with_seed(1000, Some(42))?;
/// let r2 = sim.run_with_seed(1000, Some(43))?;
/// # Ok(())
/// # }
/// ```
/// 
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder};
/// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
/// # 
/// # struct MyEngineBuilder;
/// # 
/// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
/// #     type Engine = ExternalClassicalEngine;
/// #     
/// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
/// #         Ok(ExternalClassicalEngine::new())
/// #     }
/// # }
/// # 
/// # let engine = MyEngineBuilder;
/// // Option 2: No seed = random each time
/// let sim = engine.to_sim().build()?;
/// let r1 = sim.run(1000)?;  // Random
/// let r2 = sim.run(1000)?;  // Random, different from r1
/// # Ok(())
/// # }
/// ```
///
/// The reusable pattern is particularly useful for:
/// - Parameter sweeps over shot counts
/// - Statistical analysis requiring multiple runs
/// - Benchmarking and performance testing
/// - Production scenarios where the same circuit is run repeatedly
pub struct SimBuilder<B: ClassicalControlEngineBuilder> {
    engine_builder: B,
    config: SimConfig,
    noise_model: Option<Box<dyn NoiseModel>>,
}

impl<B: ClassicalControlEngineBuilder> SimBuilder<B> {
    /// Create a new simulation builder
    pub fn new(engine_builder: B) -> Self {
        Self {
            engine_builder,
            config: SimConfig::default(),
            noise_model: None,
        }
    }

    /// Set the random seed
    pub fn seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self
    }

    /// Set the number of worker threads
    pub fn workers(mut self, workers: usize) -> Self {
        self.config.workers = workers;
        self
    }

    /// Use automatic worker count based on available CPUs
    pub fn auto_workers(mut self) -> Self {
        self.config.workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        self
    }

    /// Set the noise model
    ///
    /// This method accepts any type that can be converted into a noise model,
    /// including noise structs and builders.
    pub fn noise<N>(mut self, noise: N) -> Self
    where
        N: Into<Box<dyn NoiseModel>>,
    {
        self.noise_model = Some(noise.into());
        self
    }

    /// Set the quantum engine type
    pub fn quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.config.quantum_engine = engine;
        self
    }

    /// Set maximum number of qubits allowed
    pub fn max_qubits(mut self, max_qubits: usize) -> Self {
        self.config.max_qubits = Some(max_qubits);
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }

    /// Build the simulation
    ///
    /// This creates a reusable simulation object that can be run multiple times.
    pub fn build(self) -> Result<Simulation<B::Engine>, PecosError> {
        // Build the classical engine
        let engine = self.engine_builder.build()?;

        // Get noise model or use default
        let noise_model = self.noise_model
            .unwrap_or_else(|| Box::new(PassThroughNoiseModel::new()));

        Ok(Simulation {
            engine,
            config: self.config,
            noise_model,
        })
    }

    /// Run the simulation directly
    ///
    /// This is a convenience method that builds and runs the simulation.
    pub fn run(self, shots: usize) -> Result<ShotVec, PecosError> {
        let sim = self.build()?;
        sim.run(shots)
    }
}

/// Convert ShotVec to columnar format
///
/// This is a helper for engines that need to return HashMap<String, Vec<i64>>
pub fn shots_to_columnar(shots: ShotVec) -> HashMap<String, Vec<i64>> {
    let mut columnar = HashMap::new();
    
    if shots.is_empty() {
        return columnar;
    }

    // Get all register names from first shot
    let register_names: Vec<String> = if let Some(first_shot) = shots.shots.first() {
        first_shot.data.keys().cloned().collect()
    } else {
        return columnar;
    };

    // Initialize columns
    for name in &register_names {
        columnar.insert(name.clone(), Vec::with_capacity(shots.len()));
    }

    // Fill columns
    for shot in &shots.shots {
        for name in &register_names {
            if let Some(data) = shot.data.get(name) {
                use crate::shot_results::Data;
                let value = match data {
                    Data::U32(v) => i64::from(*v),
                    Data::I64(v) => *v,
                    Data::F64(v) => *v as i64,
                    Data::Bool(v) => i64::from(*v),
                    _ => 0,
                };
                columnar.get_mut(name).unwrap().push(value);
            } else {
                columnar.get_mut(name).unwrap().push(0);
            }
        }
    }

    // If no named registers, create a default "_result" register
    if columnar.is_empty() {
        let values: Vec<i64> = shots.shots.iter().map(|_| 0).collect();
        columnar.insert("_result".to_string(), values);
    }

    columnar
}

// ============================================================================
// Noise Model Structs for Ergonomic API
// ============================================================================

/// Pass-through noise configuration (no noise)
#[derive(Debug, Clone, Copy)]
pub struct PassThroughNoise;

/// Depolarizing noise configuration
#[derive(Debug, Clone, Copy)]
pub struct DepolarizingNoise {
    /// The depolarizing probability
    pub p: f64,
}

/// Custom depolarizing noise configuration
#[derive(Debug, Clone, Copy)]
pub struct DepolarizingCustomNoise {
    /// Preparation error probability
    pub p_prep: f64,
    /// Measurement error probability
    pub p_meas: f64,
    /// Single-qubit gate error probability
    pub p1: f64,
    /// Two-qubit gate error probability
    pub p2: f64,
}

/// Biased depolarizing noise configuration
#[derive(Debug, Clone, Copy)]
pub struct BiasedDepolarizingNoise {
    /// The depolarizing probability
    pub p: f64,
}

// ============================================================================
// Conversions to Box<dyn NoiseModel>
// ============================================================================

impl From<PassThroughNoise> for Box<dyn NoiseModel> {
    fn from(_: PassThroughNoise) -> Self {
        Box::new(PassThroughNoiseModel::new())
    }
}

impl From<PassThroughNoiseModelBuilder> for Box<dyn NoiseModel> {
    fn from(builder: PassThroughNoiseModelBuilder) -> Self {
        Box::new(builder.build())
    }
}

impl From<DepolarizingNoise> for Box<dyn NoiseModel> {
    fn from(noise: DepolarizingNoise) -> Self {
        Box::new(DepolarizingNoiseModel::new_uniform(noise.p))
    }
}

impl From<DepolarizingNoiseModelBuilder> for Box<dyn NoiseModel> {
    fn from(builder: DepolarizingNoiseModelBuilder) -> Self {
        Box::new(builder.build())
    }
}

impl From<DepolarizingCustomNoise> for Box<dyn NoiseModel> {
    fn from(noise: DepolarizingCustomNoise) -> Self {
        Box::new(DepolarizingNoiseModel::new(
            noise.p_prep,
            noise.p_meas,
            noise.p1,
            noise.p2,
        ))
    }
}

impl From<BiasedDepolarizingNoise> for Box<dyn NoiseModel> {
    fn from(noise: BiasedDepolarizingNoise) -> Self {
        Box::new(BiasedDepolarizingNoiseModel::new_uniform(noise.p))
    }
}

impl From<BiasedDepolarizingNoiseModelBuilder> for Box<dyn NoiseModel> {
    fn from(builder: BiasedDepolarizingNoiseModelBuilder) -> Self {
        Box::new(builder.build())
    }
}

impl From<GeneralNoiseModelBuilder> for Box<dyn NoiseModel> {
    fn from(builder: GeneralNoiseModelBuilder) -> Self {
        Box::new(builder.build())
    }
}