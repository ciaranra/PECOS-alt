//! Builder-based simulation runner for QASM
//!
//! This module provides a fluent builder API for running QASM simulations
//! with support for various noise models and quantum engines.

use crate::QASMEngine;
use pecos_core::errors::PecosError;
use pecos_engines::noise::{
    BiasedDepolarizingNoiseModel, BiasedMeasurementNoiseModel, DepolarizingNoiseModel,
    GeneralNoiseModel, GeneralNoiseModelBuilder, NoiseModel, PassThroughNoiseModel,
};
use pecos_engines::quantum::{QuantumEngine, SparseStabEngine, StateVecEngine};
use pecos_engines::shot_results::ShotVec;
use pecos_engines::{ClassicalEngine, MonteCarloEngine};
use std::str::FromStr;

/// Noise model configuration
///
/// This enum holds the configuration for different noise models.
/// Use the config structs (e.g., `DepolarizingNoise`) for a more ergonomic API.
#[derive(Debug, Clone)]
pub enum NoiseModelConfig {
    /// No noise (ideal simulation)
    PassThrough(PassThroughNoise),
    /// Standard depolarizing noise
    Depolarizing(DepolarizingNoise),
    /// Depolarizing noise with custom probabilities
    DepolarizingCustom(DepolarizingCustomNoise),
    /// Biased depolarizing noise
    BiasedDepolarizing(BiasedDepolarizingNoise),
    /// Biased measurement noise
    BiasedMeasurement(BiasedMeasurementNoise),
    /// General noise model
    General(GeneralNoise),
    /// General noise model from builder
    GeneralFromBuilder(Box<GeneralNoiseModelBuilder>),
}

// Keep the old type alias for backward compatibility during migration
pub type NoiseModelType = NoiseModelConfig;

impl NoiseModelType {
    /// Create a boxed noise model instance
    #[must_use]
    pub fn create_noise_model(self) -> Box<dyn NoiseModel> {
        match self {
            Self::PassThrough(_) => Box::new(PassThroughNoiseModel),
            Self::Depolarizing(config) => Box::new(DepolarizingNoiseModel::new_uniform(config.p)),
            Self::DepolarizingCustom(config) => Box::new(DepolarizingNoiseModel::new(
                config.p_prep,
                config.p_meas,
                config.p1,
                config.p2,
            )),
            Self::BiasedDepolarizing(config) => {
                Box::new(BiasedDepolarizingNoiseModel::new_uniform(config.p))
            }
            Self::BiasedMeasurement(config) => {
                Box::new(BiasedMeasurementNoiseModel::new(config.p0, config.p1))
            }
            Self::General(_) => Box::new(GeneralNoiseModel::default()),
            Self::GeneralFromBuilder(builder) => Box::new(builder.build()),
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

// Noise model configuration structs

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

/// Biased measurement noise configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiasedMeasurementNoise {
    /// Probability of measuring 1 when the true state is 0
    pub p0: f64,
    /// Probability of measuring 0 when the true state is 1
    pub p1: f64,
}

/// General noise configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneralNoise;

// Implement From traits for converting noise configs to NoiseModelType

impl From<PassThroughNoise> for NoiseModelType {
    fn from(noise: PassThroughNoise) -> Self {
        NoiseModelType::PassThrough(noise)
    }
}

impl From<DepolarizingNoise> for NoiseModelType {
    fn from(noise: DepolarizingNoise) -> Self {
        NoiseModelType::Depolarizing(noise)
    }
}

impl From<DepolarizingCustomNoise> for NoiseModelType {
    fn from(noise: DepolarizingCustomNoise) -> Self {
        NoiseModelType::DepolarizingCustom(noise)
    }
}

impl From<BiasedDepolarizingNoise> for NoiseModelType {
    fn from(noise: BiasedDepolarizingNoise) -> Self {
        NoiseModelType::BiasedDepolarizing(noise)
    }
}

impl From<BiasedMeasurementNoise> for NoiseModelType {
    fn from(noise: BiasedMeasurementNoise) -> Self {
        NoiseModelType::BiasedMeasurement(noise)
    }
}

impl From<GeneralNoise> for NoiseModelType {
    fn from(noise: GeneralNoise) -> Self {
        NoiseModelType::General(noise)
    }
}

impl From<GeneralNoiseModelBuilder> for NoiseModelType {
    fn from(builder: GeneralNoiseModelBuilder) -> Self {
        NoiseModelType::GeneralFromBuilder(Box::new(builder))
    }
}

/// A built QASM simulation that can be run multiple times
pub struct QasmSimulation {
    engine: QASMEngine,
    seed: Option<u64>,
    workers: usize,
    noise_model: NoiseModelType,
    quantum_engine_type: QuantumEngineType,
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
        let num_qubits = self.engine.num_qubits();

        // Create fresh engine instance for this run
        let engine = self.engine.clone();

        // Create noise model
        let noise_model = self.noise_model.clone().create_noise_model();

        // Create quantum engine with seed if provided
        let quantum_engine = if let Some(seed) = self.seed {
            self.quantum_engine_type
                .create_quantum_engine_with_seed(num_qubits, seed)
        } else {
            self.quantum_engine_type.create_quantum_engine(num_qubits)
        };

        // Run simulation
        MonteCarloEngine::run_with_engines(
            Box::new(engine),
            noise_model,
            quantum_engine,
            shots,
            self.workers,
            self.seed,
        )
    }
}

/// Builder for QASM simulation
pub struct QasmSimulationBuilder<'a> {
    qasm: &'a str,
    seed: Option<u64>,
    workers: usize,
    noise_model: NoiseModelType,
    quantum_engine: QuantumEngineType,
}

impl<'a> QasmSimulationBuilder<'a> {
    /// Create a new builder with the given QASM code
    fn new(qasm: &'a str) -> Self {
        Self {
            qasm,
            seed: None,
            workers: 1,
            noise_model: NoiseModelType::PassThrough(PassThroughNoise),
            quantum_engine: QuantumEngineType::SparseStabilizer,
        }
    }

    /// Set the random seed for reproducible results
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the number of worker threads
    #[must_use]
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    /// Automatically set workers based on available CPU cores
    #[must_use]
    pub fn auto_workers(mut self) -> Self {
        self.workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        self
    }

    /// Set the noise model using any type that implements Into<NoiseModelType>
    #[must_use]
    pub fn noise<N: Into<NoiseModelType>>(mut self, noise: N) -> Self {
        self.noise_model = noise.into();
        self
    }

    /// Set the quantum engine type
    #[must_use]
    pub fn quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.quantum_engine = engine;
        self
    }

    /// Build the simulation for repeated execution
    ///
    /// This parses the QASM code and prepares the simulation
    /// to be run multiple times with different shot counts.
    ///
    /// # Errors
    ///
    /// Returns an error if QASM parsing fails.
    pub fn build(self) -> Result<QasmSimulation, PecosError> {
        let engine = QASMEngine::from_str(self.qasm)?;

        Ok(QasmSimulation {
            engine,
            seed: self.seed,
            workers: self.workers,
            noise_model: self.noise_model,
            quantum_engine_type: self.quantum_engine,
        })
    }

    /// Run the simulation with the specified number of shots
    ///
    /// This is a convenience method that builds and runs in one step.
    ///
    /// # Errors
    ///
    /// Returns an error if QASM parsing or simulation fails.
    pub fn run(self, shots: usize) -> Result<ShotVec, PecosError> {
        self.build()?.run(shots)
    }
}

/// Entry point for QASM simulation
///
/// # Examples
///
/// ## Simple usage
/// ```
/// # use pecos_qasm::simulation::qasm_sim;
/// # let qasm = "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; creg c[1]; x q[0]; measure q[0] -> c[0];";
/// // Simple case - just run with shots
/// let results = qasm_sim(qasm).run(1000).unwrap();
/// assert_eq!(results.len(), 1000);
/// ```
///
/// ## Build once, run multiple times
/// ```
/// # use pecos_qasm::simulation::qasm_sim;
/// # let qasm = "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; creg c[1]; h q[0]; measure q[0] -> c[0];";
/// let sim = qasm_sim(qasm).seed(42).build().unwrap();
///
/// // Run with different shot counts
/// let results_100 = sim.run(100).unwrap();
/// let results_1000 = sim.run(1000).unwrap();
/// assert_eq!(results_100.len(), 100);
/// assert_eq!(results_1000.len(), 1000);
/// ```
///
/// ## With noise
/// ```
/// # use pecos_qasm::simulation::{qasm_sim, DepolarizingNoise};
/// # let qasm = "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[2]; creg c[2]; h q[0]; cx q[0], q[1]; measure q -> c;";
/// let results = qasm_sim(qasm)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(1000)
///     .unwrap();
/// ```
///
/// ## Full configuration
/// ```
/// # use pecos_qasm::simulation::{qasm_sim, BiasedMeasurementNoise, QuantumEngineType};
/// # let qasm = "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[2]; creg c[2]; h q[0]; cx q[0], q[1]; measure q -> c;";
/// let sim = qasm_sim(qasm)
///     .seed(42)
///     .auto_workers()
///     .quantum_engine(QuantumEngineType::StateVector)
///     .noise(BiasedMeasurementNoise { p0: 0.01, p1: 0.02 })
///     .build()
///     .unwrap();
///
/// // Run multiple simulations
/// for shots in [100, 1000, 10000] {
///     let results = sim.run(shots).unwrap();
///     println!("Got {} results", results.len());
/// }
/// ```
#[must_use]
pub fn qasm_sim(qasm: &str) -> QasmSimulationBuilder {
    QasmSimulationBuilder::new(qasm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::run_qasm;

    #[test]
    fn test_noise_model_creation() {
        // Test that each noise model type can be created
        let _pass_through = NoiseModelType::PassThrough(PassThroughNoise).create_noise_model();
        let _depolarizing =
            NoiseModelType::Depolarizing(DepolarizingNoise { p: 0.01 }).create_noise_model();
        let _depolarizing_custom = NoiseModelType::DepolarizingCustom(DepolarizingCustomNoise {
            p_prep: 0.01,
            p_meas: 0.02,
            p1: 0.03,
            p2: 0.04,
        })
        .create_noise_model();
        let _biased_depolarizing =
            NoiseModelType::BiasedDepolarizing(BiasedDepolarizingNoise { p: 0.01 })
                .create_noise_model();
        let _biased_measurement =
            NoiseModelType::BiasedMeasurement(BiasedMeasurementNoise { p0: 0.01, p1: 0.02 })
                .create_noise_model();
        let _general = NoiseModelType::General(GeneralNoise).create_noise_model();
    }

    #[test]
    fn test_quantum_engine_creation() {
        // Test that each quantum engine type can be created
        let _state_vec = QuantumEngineType::StateVector.create_quantum_engine(5);
        let _sparse_stab = QuantumEngineType::SparseStabilizer.create_quantum_engine(5);

        // Test with seed
        let _state_vec_seeded =
            QuantumEngineType::StateVector.create_quantum_engine_with_seed(5, 42);
        let _sparse_stab_seeded =
            QuantumEngineType::SparseStabilizer.create_quantum_engine_with_seed(5, 42);
    }

    #[test]
    fn test_builder_api() {
        let qasm = r#"
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
        "#;

        // Test simple usage
        let result = qasm_sim(qasm).run(10);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 10);

        // Test build and run multiple times
        let sim = qasm_sim(qasm).seed(42).build().unwrap();
        let result1 = sim.run(100).unwrap();
        let result2 = sim.run(100).unwrap();
        // Should be deterministic
        assert_eq!(
            result1
                .try_as_shot_map()
                .unwrap()
                .try_bits_as_u64("c")
                .unwrap(),
            result2
                .try_as_shot_map()
                .unwrap()
                .try_bits_as_u64("c")
                .unwrap()
        );

        // Run with different shot counts
        let result_50 = sim.run(50).unwrap();
        let result_200 = sim.run(200).unwrap();
        assert_eq!(result_50.len(), 50);
        assert_eq!(result_200.len(), 200);

        // Test with noise struct
        let result = qasm_sim(qasm)
            .noise(DepolarizingNoise { p: 0.01 })
            .run(1000);
        assert!(result.is_ok());

        // Test with custom noise
        let result = qasm_sim(qasm)
            .noise(DepolarizingCustomNoise {
                p_prep: 0.001,
                p_meas: 0.002,
                p1: 0.003,
                p2: 0.004,
            })
            .run(100);
        assert!(result.is_ok());

        // Test auto workers
        let result = qasm_sim(qasm).auto_workers().run(100);
        assert!(result.is_ok());

        // Test full configuration with build
        let sim = qasm_sim(qasm)
            .seed(123)
            .workers(2)
            .quantum_engine(QuantumEngineType::StateVector)
            .noise(BiasedMeasurementNoise { p0: 0.01, p1: 0.02 })
            .build()
            .unwrap();

        let result = sim.run(500);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deterministic_simulation() {
        let qasm = r#"
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            h q[0];
            measure q[0] -> c[0];
        "#;

        // Run twice with same seed
        let result1 = qasm_sim(qasm).seed(42).run(100).unwrap();
        let result2 = qasm_sim(qasm).seed(42).run(100).unwrap();

        // Convert to shot maps for comparison
        let map1 = result1.try_as_shot_map().unwrap();
        let map2 = result2.try_as_shot_map().unwrap();

        // Results should be identical
        let bits1 = map1.try_bits_as_u64("c").unwrap();
        let bits2 = map2.try_bits_as_u64("c").unwrap();
        assert_eq!(bits1, bits2);
    }

    #[test]
    fn test_builder_integration() {
        let qasm = r#"
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
        "#;

        // Create a custom noise model using the builder
        let noise_builder = GeneralNoiseModel::builder()
            .with_prep_probability(0.001)
            .with_meas_0_probability(0.005)
            .with_meas_1_probability(0.01)
            .with_p1_probability(0.0001)
            .with_p2_probability(0.01)
            .with_seed(42);

        // Use with qasm_sim
        let results = qasm_sim(qasm).noise(noise_builder).seed(42).run(1000);

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 1000);
    }

    #[test]
    fn test_builder_with_run_qasm() {
        let qasm = r#"
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[1];
            creg c[1];
            x q[0];
            measure q[0] -> c[0];
        "#;

        // Create a noise model with builder
        let noise = GeneralNoiseModel::builder()
            .with_meas_0_probability(0.1) // 10% chance to flip 0->1
            .with_meas_1_probability(0.05); // 5% chance to flip 1->0

        // Use with run_qasm
        let results = run_qasm(qasm, 1000, noise, None, None, Some(42)).unwrap();

        // Check that we get some errors due to measurement noise
        let shot_map = results.try_as_shot_map().unwrap();
        let values = shot_map.try_bits_as_u64("c").unwrap();

        // Count how many times we measured 0 (should be ~5% due to measurement error on |1>)
        let zeros = values.iter().filter(|&&v| v == 0).count();

        // With 5% measurement error on |1>, we expect around 50 zeros out of 1000
        assert!(zeros > 20); // Should have some errors
        assert!(zeros < 100); // But not too many
    }
}
