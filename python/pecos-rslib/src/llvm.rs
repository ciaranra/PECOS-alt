//! Python bindings for LLVM simulation with full feature parity with qasm_sim

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use pecos_llvm_sim::{llvm_sim, NoiseModelConfig, QuantumEngineType, LlvmSimBuilder as RustLlvmSimBuilder};

/// Noise model options for LLVM simulation
#[pyclass]
#[derive(Debug, Clone)]
pub enum LlvmNoiseModel {
    /// No noise (ideal simulation)
    PassThrough,
    /// Uniform depolarizing noise
    Depolarizing { p: f64 },
    /// Custom depolarizing noise
    DepolarizingCustom { p_prep: f64, p_meas: f64, p1: f64, p2: f64 },
    /// Biased depolarizing noise
    BiasedDepolarizing { p: f64 },
}

impl From<LlvmNoiseModel> for NoiseModelConfig {
    fn from(model: LlvmNoiseModel) -> Self {
        match model {
            LlvmNoiseModel::PassThrough => NoiseModelConfig::PassThrough,
            LlvmNoiseModel::Depolarizing { p } => NoiseModelConfig::Depolarizing(p),
            LlvmNoiseModel::DepolarizingCustom { p_prep, p_meas, p1, p2 } => {
                NoiseModelConfig::DepolarizingCustom { p_prep, p_meas, p1, p2 }
            }
            LlvmNoiseModel::BiasedDepolarizing { p } => NoiseModelConfig::BiasedDepolarizing(p),
        }
    }
}

/// Quantum engine options for LLVM simulation
#[pyclass]
#[derive(Debug, Clone)]
pub enum LlvmQuantumEngine {
    /// State vector simulator
    StateVector,
    /// Sparse stabilizer simulator
    SparseStabilizer,
}

impl From<LlvmQuantumEngine> for QuantumEngineType {
    fn from(engine: LlvmQuantumEngine) -> Self {
        match engine {
            LlvmQuantumEngine::StateVector => QuantumEngineType::StateVector,
            LlvmQuantumEngine::SparseStabilizer => QuantumEngineType::SparseStabilizer,
        }
    }
}

/// Builder for LLVM simulations
#[pyclass]
pub struct PyLlvmSimBuilder {
    builder: RustLlvmSimBuilder,
}

#[pymethods]
impl PyLlvmSimBuilder {
    /// Set random seed for reproducibility
    fn seed(&mut self, seed: u64) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().seed(seed),
        })
    }

    /// Set number of worker threads
    fn workers(&mut self, workers: usize) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().workers(workers),
        })
    }

    /// Set noise model
    fn noise(&mut self, noise_model: LlvmNoiseModel) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().noise(noise_model.into()),
        })
    }

    /// Enable uniform depolarizing noise
    fn with_depolarizing_noise(&mut self, p: f64) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().with_depolarizing_noise(p),
        })
    }

    /// Enable custom depolarizing noise
    fn with_custom_depolarizing_noise(
        &mut self,
        p_prep: f64,
        p_meas: f64,
        p1: f64,
        p2: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().with_custom_depolarizing_noise(p_prep, p_meas, p1, p2),
        })
    }

    /// Enable biased depolarizing noise
    fn with_biased_depolarizing_noise(&mut self, p: f64) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().with_biased_depolarizing_noise(p),
        })
    }

    /// Set quantum engine type
    fn quantum_engine(&mut self, engine: LlvmQuantumEngine) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().quantum_engine(engine.into()),
        })
    }

    /// Use state vector quantum engine
    fn with_state_vector_engine(&mut self) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().with_state_vector_engine(),
        })
    }

    /// Use sparse stabilizer quantum engine
    fn with_sparse_stabilizer_engine(&mut self) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().with_sparse_stabilizer_engine(),
        })
    }

    /// Enable verbose output
    fn verbose(&mut self, verbose: bool) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().verbose(verbose),
        })
    }

    /// Enable debug information
    fn debug(&mut self, debug: bool) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().debug(debug),
        })
    }

    /// Keep temporary files
    fn keep_temp_files(&mut self, keep: bool) -> PyResult<Self> {
        Ok(Self {
            builder: self.builder.clone().keep_temp_files(keep),
        })
    }

    /// Build the simulation
    fn build(&self) -> PyResult<PyLlvmSimulation> {
        let sim = self.builder.clone().build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to build LLVM simulation: {}", e)
            ))?;
        
        Ok(PyLlvmSimulation {
            simulation: Box::new(sim),
        })
    }

    /// Build and run in one call
    fn run(&self, py: Python<'_>, shots: usize) -> PyResult<PyObject> {
        let results = self.builder.clone().run(shots)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to run LLVM simulation: {}", e)
            ))?;
        
        // Convert HashMap to Python dict
        let py_dict = PyDict::new(py);
        for (key, values) in results {
            py_dict.set_item(key, values)?;
        }
        
        Ok(py_dict.into())
    }
}

/// A built LLVM simulation ready to run
#[pyclass]
pub struct PyLlvmSimulation {
    simulation: Box<pecos_llvm_sim::LlvmSimulation>,
}

#[pymethods]
impl PyLlvmSimulation {
    /// Run the simulation with the given number of shots
    fn run(&mut self, py: Python<'_>, shots: usize) -> PyResult<PyObject> {
        let results = self.simulation.run(shots)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to run LLVM simulation: {}", e)
            ))?;
        
        // Convert HashMap to Python dict
        let py_dict = PyDict::new(py);
        for (key, values) in results {
            py_dict.set_item(key, values)?;
        }
        
        Ok(py_dict.into())
    }

    /// Get statistics about the simulation
    fn stats(&self) -> PyResult<(usize, usize)> {
        Ok(self.simulation.stats())
    }
}

/// Create an LLVM simulation builder
///
/// This is the main entry point for LLVM-based quantum simulations with full
/// feature parity with qasm_sim.
///
/// Args:
///     source: LLVM IR string or file path
///
/// Returns:
///     PyLlvmSimBuilder: Builder for configuring the simulation
///
/// Examples:
///     >>> # From LLVM IR string
///     >>> results = llvm_sim(llvm_ir).seed(42).run(1000)
///     
///     >>> # With noise and parallelization
///     >>> results = llvm_sim(llvm_ir) \
///     ...     .seed(42) \
///     ...     .workers(8) \
///     ...     .with_depolarizing_noise(0.01) \
///     ...     .run(10000)
///     
///     >>> # Build once, run many
///     >>> sim = llvm_sim(llvm_ir).seed(42).build()
///     >>> results1 = sim.run(100)
///     >>> results2 = sim.run(1000)
#[pyfunction]
pub fn llvm_sim_builder(source: String) -> PyResult<PyLlvmSimBuilder> {
    Ok(PyLlvmSimBuilder {
        builder: llvm_sim(source),
    })
}

