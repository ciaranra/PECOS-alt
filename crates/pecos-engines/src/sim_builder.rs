//! Unified simulation builder for all engine types
//!
//! This module provides the `SimBuilder` struct that handles common simulation
//! configuration (seed, workers, noise, quantum engine) for any classical control engine.

use crate::classical::{ClassicalEngine, ClassicalControlEngine};
use crate::engine_builder::ClassicalControlEngineBuilder;
use crate::noise::{
    NoiseModel, PassThroughNoiseModel, PassThroughNoiseModelBuilder,
    DepolarizingNoiseModel, DepolarizingNoiseModelBuilder,
    BiasedDepolarizingNoiseModel, BiasedDepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder,
};
use crate::quantum::QuantumEngine;
use crate::quantum_engine_builder::{QuantumEngineBuilder, IntoQuantumEngineBuilder, sparse_stab};
use crate::shot_results::ShotVec;
use crate::MonteCarloEngine;
use pecos_core::errors::PecosError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Removed QuantumEngineType enum - using builders instead

/// Configuration for simulations
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Number of worker threads
    pub workers: usize,
    /// Verbose output
    pub verbose: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            seed: None,
            workers: 1,
            verbose: false,
        }
    }
}

/// Statistics tracking for simulation runs
#[derive(Debug, Clone, Default)]
struct RunStats {
    total_shots: usize,
    run_count: usize,
}

/// A built simulation ready to run
pub struct Simulation<E: ClassicalControlEngine> {
    engine: E,
    quantum_engine: Box<dyn QuantumEngine>,
    noise_model: Box<dyn NoiseModel>,
    config: SimConfig,
    stats: Arc<Mutex<RunStats>>,
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
    /// ```rust
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
    /// let r1 = sim.run_with_seed(10, Some(42))?;
    /// let r2 = sim.run_with_seed(10, Some(43))?;
    /// let r3 = sim.run_with_seed(10, None)?;  // Random
    /// 
    /// // Verify we got results
    /// assert_eq!(r1.len(), 10);
    /// assert_eq!(r2.len(), 10);
    /// assert_eq!(r3.len(), 10);
    /// # Ok(())
    /// # }
    /// ```
    pub fn run_with_seed(&self, shots: usize, seed: Option<u64>) -> Result<ShotVec, PecosError> {
        // Handle zero shots case
        if shots == 0 {
            // Update statistics even for zero shots
            if let Ok(mut stats) = self.stats.lock() {
                stats.run_count += 1;
            }
            return Ok(ShotVec::new());
        }
        
        // Use pre-built quantum engine (cloned for thread safety)
        let quantum_engine = self.quantum_engine.clone();

        // Run using MonteCarloEngine
        let result = MonteCarloEngine::run_with_engines(
            Box::new(self.engine.clone()),
            self.noise_model.clone(),
            quantum_engine,
            shots,
            self.config.workers,
            seed,
        )?;
        
        // Update statistics
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_shots += shots;
            stats.run_count += 1;
        }
        
        Ok(result)
    }

    /// Get statistics about the simulation runs
    /// 
    /// Returns (total_shots_run, number_of_runs)
    pub fn stats(&self) -> (usize, usize) {
        if let Ok(stats) = self.stats.lock() {
            (stats.total_shots, stats.run_count)
        } else {
            (0, 0)
        }
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
/// ```rust
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
/// let results = engine.to_sim().seed(42).run(10)?;
/// assert_eq!(results.len(), 10);
/// # Ok(())
/// # }
/// ```
///
/// ## Build once, run multiple times
/// ```rust
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
/// let results_10 = sim.run(10)?;
/// let results_20 = sim.run(20)?;
/// let results_30 = sim.run(30)?;
/// 
/// assert_eq!(results_10.len(), 10);
/// assert_eq!(results_20.len(), 20);
/// assert_eq!(results_30.len(), 30);
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
/// ```rust
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
/// let r1 = sim.run_with_seed(10, Some(42))?;
/// let r2 = sim.run_with_seed(10, Some(43))?;
/// assert_eq!(r1.len(), 10);
/// assert_eq!(r2.len(), 10);
/// # Ok(())
/// # }
/// ```
/// 
/// ```rust
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
/// let r1 = sim.run(10)?;  // Random
/// let r2 = sim.run(10)?;  // Random, different from r1
/// assert_eq!(r1.len(), 10);
/// assert_eq!(r2.len(), 10);
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
    noise_model_factory: Option<Box<dyn FnOnce() -> Box<dyn NoiseModel> + Send>>,
    quantum_engine_builder: Option<Box<dyn QuantumEngineBuilder>>,
    explicit_num_qubits: Option<usize>,
}

impl<B: ClassicalControlEngineBuilder> SimBuilder<B> {
    /// Create a new simulation builder
    pub fn new(engine_builder: B) -> Self {
        Self {
            engine_builder,
            config: SimConfig::default(),
            noise_model_factory: None,
            quantum_engine_builder: None,
            explicit_num_qubits: None,
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

    /// Set the noise model from a builder
    ///
    /// This method accepts noise model builders, which are stored for lazy evaluation
    /// and built later when the simulation is created.
    pub fn noise<N>(mut self, noise_builder: N) -> Self
    where
        N: crate::noise::IntoNoiseModel + 'static,
    {
        self.noise_model_factory = Some(Box::new(move || noise_builder.into_noise_model()));
        self
    }

    /// Set the quantum engine using any type that implements IntoQuantumEngineBuilder
    /// 
    /// This method accepts quantum engine builders from this crate or custom
    /// engine builders from other crates.
    /// 
    /// # Examples
    /// ```rust
    /// # use pecos_core::errors::PecosError;
    /// # use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder};
    /// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
    /// # 
    /// # struct MyEngineBuilder;
    /// # 
    /// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
    /// #     type Engine = ExternalClassicalEngine;
    /// #     
    /// #     fn build(self) -> Result<Self::Engine, PecosError> {
    /// #         Ok(ExternalClassicalEngine::new())
    /// #     }
    /// # }
    /// # 
    /// # fn example() -> Result<(), PecosError> {
    /// use pecos_engines::quantum_engine_builder::{state_vector, sparse_stabilizer};
    /// 
    /// // Using builder functions
    /// let sim1 = MyEngineBuilder.to_sim()
    ///     .quantum(state_vector())
    ///     .build()?;
    ///     
    /// // Using builder with configuration - note: qubits() is on SimBuilder, not quantum engine builder
    /// let sim2 = MyEngineBuilder.to_sim()
    ///     .quantum(sparse_stabilizer())
    ///     .qubits(20)
    ///     .build()?;
    ///     
    /// // Run a quick test to verify they work
    /// let r1 = sim1.run(1)?;
    /// let r2 = sim2.run(1)?;
    /// assert_eq!(r1.len(), 1);
    /// assert_eq!(r2.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn quantum<Q>(mut self, engine: Q) -> Self
    where
        Q: IntoQuantumEngineBuilder + 'static,
        Q::Builder: 'static,
    {
        self.quantum_engine_builder = Some(Box::new(engine.into_quantum_engine_builder()));
        self
    }
    
    /// Set the number of qubits for the simulation
    /// 
    /// This overrides any qubit count from the quantum engine builder or program.
    /// The last .qubits() call wins.
    pub fn qubits(mut self, num_qubits: usize) -> Self {
        self.explicit_num_qubits = Some(num_qubits);
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
        
        // Determine the number of qubits
        // Priority: 1. explicit_num_qubits, 2. engine.num_qubits()
        let num_qubits = if let Some(n) = self.explicit_num_qubits {
            n
        } else {
            engine.num_qubits()
        };
        

        // Build quantum engine
        let quantum_engine = if let Some(mut builder) = self.quantum_engine_builder {
            // Ensure the builder has qubits set
            builder.set_qubits_if_needed(num_qubits);
            builder.build()?
        } else {
            // Default to sparse stabilizer
            let mut default_builder = sparse_stab().qubits(num_qubits);
            default_builder.build()?
        };

        // Build noise model from factory or use default
        let noise_model = if let Some(factory) = self.noise_model_factory {
            factory()
        } else {
            Box::new(PassThroughNoiseModel::new())
        };

        Ok(Simulation {
            engine,
            quantum_engine,
            noise_model,
            config: self.config,
            stats: Arc::new(Mutex::new(RunStats::default())),
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

impl<B: ClassicalControlEngineBuilder> From<B> for SimBuilder<B> {
    fn from(builder: B) -> Self {
        SimBuilder::new(builder)
    }
}

/// Helper function to create a simulation builder from any engine builder
///
/// This provides a functional-style API as an alternative to the `.to_sim()` method.
/// Both approaches are equivalent and can be used based on preference.
///
/// # Equivalent Patterns
///
/// These three patterns all create the same simulation:
/// ```rust
/// # use pecos_engines::{sim, SimBuilder, ClassicalControlEngineBuilder};
/// # struct MyEngineBuilder;
/// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
/// #     type Engine = pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
/// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
/// #         Ok(pecos_engines::monte_carlo::engine::ExternalClassicalEngine::new())
/// #     }
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Pattern 1: Method chaining (original)
/// let r1 = MyEngineBuilder.to_sim().seed(42).run(10)?;
/// 
/// // Pattern 2: Using From trait
/// let r2 = SimBuilder::from(MyEngineBuilder).seed(42).run(10)?;
/// 
/// // Pattern 3: Using sim() helper function
/// let r3 = sim(MyEngineBuilder).seed(42).run(10)?;
/// 
/// assert_eq!(r1.len(), 10);
/// assert_eq!(r2.len(), 10);
/// assert_eq!(r3.len(), 10);
/// # Ok(())
/// # }
/// ```
///
/// # When to Use Each Pattern
///
/// - **`.to_sim()`**: Traditional method chaining, discoverable via IDE autocomplete
/// - **`SimBuilder::from()`**: Explicit conversion, useful when you need the type
/// - **`sim()`**: Functional style, concise for nested expressions
///
/// # Examples
///
/// ## With concrete engine builders
/// ```rust
/// # #[cfg(feature = "qasm")]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use pecos_engines::{sim, DepolarizingNoise};
/// use pecos_qasm::qasm_engine;
/// use pecos_programs::QasmProgram;
/// 
/// let qasm_code = r#"
/// OPENQASM 2.0;
/// include "qelib1.inc";
/// qreg q[1];
/// creg c[1];
/// h q[0];
/// measure q[0] -> c[0];
/// "#;
/// 
/// let results = sim(qasm_engine().program(QasmProgram::from_string(qasm_code)))
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(10)?;
///     
/// assert_eq!(results.len(), 10);
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "qasm"))]
/// # fn main() {}
/// ```
///
/// ## With dynamic engine builders  
/// ```rust
/// # use pecos_engines::{sim, ClassicalControlEngineBuilder};
/// # use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
/// # struct MyEngineBuilder;
/// # impl ClassicalControlEngineBuilder for MyEngineBuilder {
/// #     type Engine = ExternalClassicalEngine;
/// #     fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
/// #         Ok(ExternalClassicalEngine::new())
/// #     }
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let results = sim(MyEngineBuilder).seed(42).run(10)?;
/// assert_eq!(results.len(), 10);
/// # Ok(())
/// # }
/// ```
pub fn sim<B: ClassicalControlEngineBuilder>(builder: B) -> SimBuilder<B> {
    SimBuilder::from(builder)
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

// ============================================================================
// IntoNoiseModel implementations for convenience structs
// ============================================================================

impl crate::noise::IntoNoiseModel for PassThroughNoise {
    fn into_noise_model(self) -> Box<dyn NoiseModel> {
        Box::new(PassThroughNoiseModel::new())
    }
}

impl crate::noise::IntoNoiseModel for DepolarizingNoise {
    fn into_noise_model(self) -> Box<dyn NoiseModel> {
        Box::new(DepolarizingNoiseModel::new_uniform(self.p))
    }
}

impl crate::noise::IntoNoiseModel for DepolarizingCustomNoise {
    fn into_noise_model(self) -> Box<dyn NoiseModel> {
        Box::new(DepolarizingNoiseModel::new(
            self.p_prep,
            self.p_meas,
            self.p1,
            self.p2,
        ))
    }
}

impl crate::noise::IntoNoiseModel for BiasedDepolarizingNoise {
    fn into_noise_model(self) -> Box<dyn NoiseModel> {
        Box::new(BiasedDepolarizingNoiseModel::new_uniform(self.p))
    }
}