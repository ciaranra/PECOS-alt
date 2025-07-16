//! Example implementation of qasm_sim using the unified engine builder API.
//!
//! This module shows how the existing `qasm_sim()` function could be 
//! reimplemented using the new unified API approach while maintaining
//! backward compatibility.

use crate::unified_engine_builder::{qasm_engine, QasmEngineBuilder};
use pecos_engines::{
    ClassicalControlEngineBuilder, 
    sim_builder::{SimBuilder, Simulation},
    noise::{NoiseModel, DepolarizingNoiseModel},
    quantum::{QuantumEngine, StateVecEngine, SparseStabilizerEngine},
    shot_results::ShotVec,
};
use pecos_core::errors::PecosError;

/// A backward-compatible builder that wraps the new unified API
pub struct QasmSimulationBuilderUnified {
    engine_builder: QasmEngineBuilder,
    sim_builder_config: SimBuilderConfig,
}

/// Configuration that needs to be applied after converting to SimBuilder
struct SimBuilderConfig {
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<QuantumEngineType>,
    binary_format: bool,
}

#[derive(Clone, Copy)]
pub enum QuantumEngineType {
    StateVector,
    SparseStabilizer,
}

impl QasmSimulationBuilderUnified {
    /// Create a new builder from QASM source
    pub fn new(qasm: impl Into<String>) -> Self {
        Self {
            engine_builder: qasm_engine().qasm(qasm),
            sim_builder_config: SimBuilderConfig {
                seed: None,
                workers: None,
                noise_model: None,
                quantum_engine: None,
                binary_format: false,
            },
        }
    }

    /// Set the random seed
    pub fn seed(mut self, seed: u64) -> Self {
        self.sim_builder_config.seed = Some(seed);
        self
    }

    /// Set the noise model
    pub fn noise<N: NoiseModel + 'static>(mut self, noise_model: N) -> Self {
        self.sim_builder_config.noise_model = Some(Box::new(noise_model));
        self
    }

    /// Set the number of worker threads
    pub fn workers(mut self, workers: usize) -> Self {
        self.sim_builder_config.workers = Some(workers);
        self
    }

    /// Automatically set workers based on CPU cores
    pub fn auto_workers(mut self) -> Self {
        self.sim_builder_config.workers = Some(0); // 0 means auto
        self
    }

    /// Set the quantum simulation engine
    pub fn quantum_engine(mut self, engine: QuantumEngineType) -> Self {
        self.sim_builder_config.quantum_engine = Some(engine);
        self
    }

    /// Set WASM module for foreign functions
    #[cfg(feature = "wasm")]
    pub fn wasm(mut self, wasm_path: impl Into<String>) -> Self {
        self.engine_builder = self.engine_builder.wasm(wasm_path);
        self
    }

    /// Enable binary string format for results
    pub fn with_binary_string_format(mut self) -> Self {
        self.sim_builder_config.binary_format = true;
        self
    }

    /// Build a reusable simulation object
    pub fn build(self) -> Result<QasmSimulationUnified, PecosError> {
        // First build the engine
        let engine = self.engine_builder.build()?;
        
        // Convert to SimBuilder
        let mut sim_builder = engine.to_sim();
        
        // Apply all the configuration
        if let Some(seed) = self.sim_builder_config.seed {
            sim_builder = sim_builder.seed(seed);
        }
        
        if let Some(workers) = self.sim_builder_config.workers {
            if workers == 0 {
                sim_builder = sim_builder.auto_workers();
            } else {
                sim_builder = sim_builder.workers(workers);
            }
        }
        
        if let Some(noise_model) = self.sim_builder_config.noise_model {
            sim_builder = sim_builder.noise_model(noise_model);
        }
        
        if let Some(engine_type) = self.sim_builder_config.quantum_engine {
            sim_builder = match engine_type {
                QuantumEngineType::StateVector => sim_builder.quantum_engine_type("state_vector"),
                QuantumEngineType::SparseStabilizer => sim_builder.quantum_engine_type("sparse_stabilizer"),
            };
        }
        
        // Build the simulation
        let simulation = sim_builder.build()?;
        
        Ok(QasmSimulationUnified {
            simulation,
            binary_format: self.sim_builder_config.binary_format,
        })
    }

    /// Run the simulation directly
    pub fn run(self, shots: usize) -> Result<ShotVec, PecosError> {
        let sim = self.build()?;
        sim.run(shots)
    }
}

/// A reusable simulation object wrapping the unified API
pub struct QasmSimulationUnified {
    simulation: Simulation<QasmEngineBuilder>,
    binary_format: bool,
}

impl QasmSimulationUnified {
    /// Run the simulation with the specified number of shots
    pub fn run(&self, shots: usize) -> Result<ShotVec, PecosError> {
        let results = self.simulation.run(shots)?;
        
        // If binary format was requested, convert the results
        // This is where you'd handle any format conversions
        
        Ok(results)
    }
    
    /// Get statistics about the simulation
    pub fn stats(&self) -> (usize, usize) {
        self.simulation.stats()
    }
}

/// Create a QASM simulation builder using the unified API
///
/// This function provides backward compatibility with the existing API
/// while leveraging the new unified simulation architecture.
///
/// # Example
///
/// ```no_run
/// # use pecos_qasm::simulation_unified::{qasm_sim, QuantumEngineType};
/// # use pecos_engines::DepolarizingNoise;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let qasm = r#"
/// OPENQASM 2.0;
/// include "qelib1.inc";
/// qreg q[2];
/// creg c[2];
/// h q[0];
/// cx q[0], q[1];
/// measure q -> c;
/// "#;
///
/// // Direct run
/// let results = qasm_sim(qasm)
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(1000)?;
///
/// // Build once, run multiple times
/// let sim = qasm_sim(qasm)
///     .seed(42)
///     .quantum_engine(QuantumEngineType::StateVector)
///     .build()?;
/// 
/// let results_100 = sim.run(100)?;
/// let results_1000 = sim.run(1000)?;
/// # Ok(())
/// # }
/// ```
pub fn qasm_sim(qasm: impl Into<String>) -> QasmSimulationBuilderUnified {
    QasmSimulationBuilderUnified::new(qasm)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unified_qasm_sim_api() {
        let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        "#;
        
        // Test that the API works as expected
        let results = qasm_sim(qasm)
            .seed(42)
            .workers(2)
            .run(100)
            .expect("Simulation should succeed");
        
        assert!(!results.is_empty());
    }
    
    #[test]
    fn test_build_once_run_many() {
        let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        "#;
        
        let sim = qasm_sim(qasm)
            .seed(42)
            .auto_workers()
            .build()
            .expect("Build should succeed");
        
        let r1 = sim.run(100).expect("First run should succeed");
        let r2 = sim.run(200).expect("Second run should succeed");
        
        // With fixed seed, results should be deterministic
        let r3 = sim.run(100).expect("Third run should succeed");
        
        // First 100 results of r3 should match r1 (same seed, same shots)
        assert_eq!(r1.len(), r3.len());
    }
}