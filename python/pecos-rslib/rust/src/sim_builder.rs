//! Python bindings for the simulation builder pattern
//!
//! This module provides thin Python wrappers around the Rust engine().to_sim() pattern.

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pecos_engines::{
    SimBuilder, sim_builder::QuantumEngineType,
    ClassicalControlEngineBuilder,
};
use pecos_engines::noise::{
    DepolarizingNoiseModelBuilder, BiasedDepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder, PassThroughNoiseModelBuilder, IntoNoiseModel
};
use pecos_qasm::{qasm_engine, QasmEngineBuilder};
use pecos_llvm_sim::{llvm_engine, LlvmEngineBuilder};
use pecos_selene::{selene_engine, SeleneEngineBuilder};
use pecos_programs::{QasmProgram, LlvmProgram};
use crate::shot_results_bindings::PyShotVec;

/// Python wrapper for QASM engine builder
#[pyclass(name = "QasmEngineBuilder")]
pub struct PyQasmEngineBuilder {
    builder: Option<QasmEngineBuilder>,
}

#[pymethods]
impl PyQasmEngineBuilder {
    /// Set QASM program
    pub fn program(&mut self, source: &str) -> PyResult<&mut Self> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.program(QasmProgram::from_string(source)));
            Ok(self)
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Convert to simulation builder
    pub fn to_sim(&mut self) -> PyResult<PySimBuilder> {
        if let Some(builder) = self.builder.take() {
            Ok(PySimBuilder {
                inner: SimBuilderInner::Qasm(Some(builder.to_sim())),
            })
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }
}

/// Python wrapper for LLVM engine builder
#[pyclass(name = "LlvmEngineBuilder")]
pub struct PyLlvmEngineBuilder {
    builder: Option<LlvmEngineBuilder>,
}

#[pymethods]
impl PyLlvmEngineBuilder {
    /// Set LLVM program
    pub fn program(&mut self, source: &str) -> PyResult<&mut Self> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.program(LlvmProgram::from_string(source)));
            Ok(self)
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Convert to simulation builder
    pub fn to_sim(&mut self) -> PyResult<PySimBuilder> {
        if let Some(builder) = self.builder.take() {
            Ok(PySimBuilder {
                inner: SimBuilderInner::Llvm(Some(builder.to_sim())),
            })
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }
}

/// Python wrapper for Selene engine builder
#[pyclass(name = "SeleneEngineBuilder")]
pub struct PySeleneEngineBuilder {
    builder: Option<SeleneEngineBuilder>,
}

#[pymethods]
impl PySeleneEngineBuilder {
    /// Set LLVM program
    pub fn program(&mut self, source: &str) -> PyResult<&mut Self> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.program(LlvmProgram::from_string(source)));
            Ok(self)
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Set number of qubits
    pub fn qubits(&mut self, n: usize) -> PyResult<&mut Self> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.qubits(n));
            Ok(self)
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Enable optimization
    pub fn optimize(&mut self, opt: bool) -> PyResult<&mut Self> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.optimize(opt));
            Ok(self)
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Convert to simulation builder
    pub fn to_sim(&mut self) -> PyResult<PySimBuilder> {
        if let Some(builder) = self.builder.take() {
            Ok(PySimBuilder {
                inner: SimBuilderInner::Selene(Some(builder.to_sim())),
            })
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }
}

/// Python wrapper for SimBuilder with type erasure
#[pyclass(name = "SimBuilder")]
pub struct PySimBuilder {
    // We use an enum to handle different concrete types
    inner: SimBuilderInner,
}

enum SimBuilderInner {
    Qasm(Option<SimBuilder<QasmEngineBuilder>>),
    Llvm(Option<SimBuilder<LlvmEngineBuilder>>),
    Selene(Option<SimBuilder<SeleneEngineBuilder>>),
}

#[pymethods]
impl PySimBuilder {
    /// Set random seed
    pub fn seed(&mut self, seed: u64) -> PyResult<&mut Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.seed(seed));
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.seed(seed));
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.seed(seed));
                }
            }
        }
        Ok(self)
    }

    /// Set number of workers
    pub fn workers(&mut self, workers: usize) -> PyResult<&mut Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.workers(workers));
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.workers(workers));
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.workers(workers));
                }
            }
        }
        Ok(self)
    }

    /// Use automatic worker count
    pub fn auto_workers(&mut self) -> PyResult<&mut Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.auto_workers());
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.auto_workers());
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.auto_workers());
                }
            }
        }
        Ok(self)
    }

    /// Set quantum engine type
    pub fn quantum_engine(&mut self, engine: &str) -> PyResult<&mut Self> {
        let engine_type = match engine.to_lowercase().as_str() {
            "statevector" | "state_vector" => QuantumEngineType::StateVector,
            "sparsestabilizer" | "sparse_stabilizer" => QuantumEngineType::SparseStabilizer,
            _ => return Err(PyValueError::new_err(format!("Unknown quantum engine: {}", engine))),
        };

        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.quantum_engine(engine_type));
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.quantum_engine(engine_type));
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.quantum_engine(engine_type));
                }
            }
        }
        Ok(self)
    }

    /// Set number of qubits for quantum engine and allocation limit
    pub fn qubits(&mut self, num_qubits: usize) -> PyResult<&mut Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.qubits(num_qubits));
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.qubits(num_qubits));
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.qubits(num_qubits));
                }
            }
        }
        Ok(self)
    }

    /// Set verbose mode
    pub fn verbose(&mut self, verbose: bool) -> PyResult<&mut Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.verbose(verbose));
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.verbose(verbose));
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.verbose(verbose));
                }
            }
        }
        Ok(self)
    }

    /// Set depolarizing noise
    pub fn noise_depolarizing(&mut self, p: f64) -> PyResult<&mut Self> {
        let noise_builder = DepolarizingNoiseModelBuilder::new()
            .with_p1_probability(p)
            .with_p2_probability(p);

        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.noise(noise_builder.clone()));
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.noise(noise_builder.clone()));
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    *builder = Some(b.noise(noise_builder));
                }
            }
        }
        Ok(self)
    }

    /// Run the simulation
    pub fn run(&mut self, shots: usize) -> PyResult<PyShotVec> {
        let result = match &mut self.inner {
            SimBuilderInner::Qasm(builder) => {
                if let Some(b) = builder.take() {
                    b.run(shots).map_err(|e| PyRuntimeError::new_err(e.to_string()))
                } else {
                    Err(PyRuntimeError::new_err("Builder already consumed"))
                }
            }
            SimBuilderInner::Llvm(builder) => {
                if let Some(b) = builder.take() {
                    b.run(shots).map_err(|e| PyRuntimeError::new_err(e.to_string()))
                } else {
                    Err(PyRuntimeError::new_err("Builder already consumed"))
                }
            }
            SimBuilderInner::Selene(builder) => {
                if let Some(b) = builder.take() {
                    b.run(shots).map_err(|e| PyRuntimeError::new_err(e.to_string()))
                } else {
                    Err(PyRuntimeError::new_err("Builder already consumed"))
                }
            }
        }?;

        Ok(PyShotVec::from(result))
    }
}

/// Create engine builder functions
#[pyfunction]
pub fn py_qasm_engine() -> PyQasmEngineBuilder {
    PyQasmEngineBuilder {
        builder: Some(qasm_engine()),
    }
}

#[pyfunction]
pub fn py_llvm_engine() -> PyLlvmEngineBuilder {
    PyLlvmEngineBuilder {
        builder: Some(llvm_engine()),
    }
}

#[pyfunction]
pub fn py_selene_engine() -> PySeleneEngineBuilder {
    PySeleneEngineBuilder {
        builder: Some(selene_engine()),
    }
}

/// Main sim function that takes an engine builder (deprecated - use .to_sim() instead)
#[pyfunction]
pub fn py_sim(_py: Python, engine_builder: &Bound<'_, PyAny>) -> PyResult<PySimBuilder> {
    // Check which type of engine builder we have
    if let Ok(qasm_builder) = engine_builder.extract::<PyRef<PyQasmEngineBuilder>>() {
        if let Some(builder) = qasm_builder.builder.clone() {
            Ok(PySimBuilder {
                inner: SimBuilderInner::Qasm(Some(builder.to_sim())),
            })
        } else {
            Err(PyRuntimeError::new_err("QASM engine builder already consumed"))
        }
    } else if let Ok(llvm_builder) = engine_builder.extract::<PyRef<PyLlvmEngineBuilder>>() {
        if let Some(builder) = llvm_builder.builder.clone() {
            Ok(PySimBuilder {
                inner: SimBuilderInner::Llvm(Some(builder.to_sim())),
            })
        } else {
            Err(PyRuntimeError::new_err("LLVM engine builder already consumed"))
        }
    } else if let Ok(selene_builder) = engine_builder.extract::<PyRef<PySeleneEngineBuilder>>() {
        if let Some(builder) = selene_builder.builder.clone() {
            Ok(PySimBuilder {
                inner: SimBuilderInner::Selene(Some(builder.to_sim())),
            })
        } else {
            Err(PyRuntimeError::new_err("Selene engine builder already consumed"))
        }
    } else {
        Err(PyValueError::new_err("Unknown engine builder type"))
    }
}

/// Register the sim builder module
pub fn register_sim_builder_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PyQasmEngineBuilder>()?;
    parent_module.add_class::<PyLlvmEngineBuilder>()?;
    parent_module.add_class::<PySeleneEngineBuilder>()?;
    parent_module.add_class::<PySimBuilder>()?;

    parent_module.add_function(wrap_pyfunction!(py_qasm_engine, parent_module)?)?;
    parent_module.add_function(wrap_pyfunction!(py_llvm_engine, parent_module)?)?;
    parent_module.add_function(wrap_pyfunction!(py_selene_engine, parent_module)?)?;
    parent_module.add_function(wrap_pyfunction!(py_sim, parent_module)?)?;

    Ok(())
}