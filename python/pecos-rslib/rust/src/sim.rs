//! Simulation API that mirrors the Rust pecos crate
//!
//! This module provides a `sim(program)` function that auto-detects the program type
//! and creates the appropriate simulation builder, following the same pattern as the
//! Rust `pecos::sim()` function.

use pyo3::prelude::*;
use pyo3::exceptions::PyTypeError;
use std::sync::{Arc, Mutex};
use pecos_engines::ClassicalControlEngineBuilder;

use pecos_qasm::qasm_engine as rust_qasm_engine;
use pecos_llvm_sim::llvm_engine as rust_llvm_engine;
use pecos_selene::selene_engine as rust_selene_engine;
use pecos_phir_json::phir_json_engine as rust_phir_json_engine;

use crate::engine_builders::{
    PyQasmProgram, PyLlvmProgram, PyHugrProgram, PyPhirJsonProgram,
    PyQasmSimBuilder, PyLlvmSimBuilder, PySeleneSimBuilder, PyPhirJsonSimBuilder,
    PyQasmEngineBuilder, PyLlvmEngineBuilder, PySeleneEngineBuilder, PyPhirJsonEngineBuilder,
};

/// Main sim function that auto-detects program type and creates appropriate builder
/// 
/// This mirrors the Rust `pecos::sim()` function, providing automatic engine selection
/// based on the program type.
/// 
/// Examples:
///     # QASM simulation
///     results = sim(QasmProgram.from_string("H q[0];")).run(1000)
///     
///     # LLVM simulation
///     results = sim(LlvmProgram.from_string(llvm_ir)).run(1000)
///     
///     # HUGR simulation (via Selene)
///     results = sim(HugrProgram.from_bytes(hugr_bytes)).qubits(2).run(1000)
///     
///     # PHIR JSON simulation
///     results = sim(PhirJsonProgram.from_json(phir_json)).run(1000)
///     
///     # Override auto-selection with explicit engine
///     results = sim(QasmProgram.from_string("H q[0];")).classical(qasm_engine().wasm("custom.wasm")).run(1000)
#[pyfunction]
#[pyo3(signature = (program))]
pub fn sim(py: Python, program: PyObject) -> PyResult<PySimBuilder> {
    // Try to extract each program type and create the appropriate builder
    if let Ok(qasm_prog) = program.extract::<PyQasmProgram>(py) {
        // Create QASM engine builder with program
        let engine_builder = rust_qasm_engine().program(qasm_prog.inner);
        Ok(PySimBuilder {
            inner: SimBuilderInner::Qasm(PyQasmSimBuilder {
                engine_builder: Arc::new(Mutex::new(Some(engine_builder))),
                seed: None,
                workers: None,
                quantum_engine_builder: None,
                noise_builder: None,
                explicit_num_qubits: None,
            }),
        })
    } else if let Ok(llvm_prog) = program.extract::<PyLlvmProgram>(py) {
        // Create LLVM engine builder with program
        let engine_builder = rust_llvm_engine().program(llvm_prog.inner);
        Ok(PySimBuilder {
            inner: SimBuilderInner::Llvm(PyLlvmSimBuilder {
                engine_builder: Arc::new(Mutex::new(Some(engine_builder))),
                seed: None,
                workers: None,
                quantum_engine_builder: None,
                noise_builder: None,
                explicit_num_qubits: None,
            }),
        })
    } else if let Ok(hugr_prog) = program.extract::<PyHugrProgram>(py) {
        // HUGR uses Selene engine
        let engine_builder = rust_selene_engine().program(hugr_prog.inner);
        Ok(PySimBuilder {
            inner: SimBuilderInner::Selene(PySeleneSimBuilder {
                engine_builder: Arc::new(Mutex::new(Some(engine_builder))),
                seed: None,
                workers: None,
                quantum_engine_builder: None,
                noise_builder: None,
                explicit_num_qubits: None,
            }),
        })
    } else if let Ok(phir_prog) = program.extract::<PyPhirJsonProgram>(py) {
        // Create PHIR JSON engine builder with program
        let engine_builder = rust_phir_json_engine().program(phir_prog.inner);
        Ok(PySimBuilder {
            inner: SimBuilderInner::PhirJson(PyPhirJsonSimBuilder {
                engine_builder: Arc::new(Mutex::new(Some(engine_builder))),
                seed: None,
                workers: None,
                quantum_engine_builder: None,
                noise_builder: None,
                explicit_num_qubits: None,
            }),
        })
    } else {
        Err(PyTypeError::new_err(
            "program must be a QasmProgram, LlvmProgram, HugrProgram, or PhirJsonProgram instance"
        ))
    }
}

/// Python wrapper for simulation builder
/// 
/// This provides a single interface that can work with any engine type,
/// delegating to the appropriate concrete builder based on the program type.
#[pyclass(name = "SimBuilder")]
pub struct PySimBuilder {
    pub(crate) inner: SimBuilderInner,
}

pub(crate) enum SimBuilderInner {
    Qasm(PyQasmSimBuilder),
    Llvm(PyLlvmSimBuilder),
    Selene(PySeleneSimBuilder),
    PhirJson(PyPhirJsonSimBuilder),
}

#[pymethods]
impl PySimBuilder {
    /// Override the auto-selected classical engine
    /// 
    /// Example:
    ///     # Use custom WASM with QASM
    ///     sim(qasm).classical(qasm_engine().wasm("custom.wasm")).run(1000)
    #[pyo3(signature = (engine_builder))]
    fn classical(&mut self, py: Python, engine_builder: PyObject) -> PyResult<Self> {
        // Extract the engine builder and update our inner builder
        match &mut self.inner {
            SimBuilderInner::Qasm(sim_builder) => {
                if let Ok(qasm_engine) = engine_builder.extract::<PyQasmEngineBuilder>(py) {
                    // Replace the engine builder
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(qasm_engine.inner)));
                    Ok(PySimBuilder { inner: self.inner.clone() })
                } else {
                    Err(PyTypeError::new_err("For QASM programs, classical() requires a QasmEngineBuilder"))
                }
            }
            SimBuilderInner::Llvm(sim_builder) => {
                if let Ok(llvm_engine) = engine_builder.extract::<PyLlvmEngineBuilder>(py) {
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(llvm_engine.inner)));
                    Ok(PySimBuilder { inner: self.inner.clone() })
                } else {
                    Err(PyTypeError::new_err("For LLVM programs, classical() requires an LlvmEngineBuilder"))
                }
            }
            SimBuilderInner::Selene(sim_builder) => {
                if let Ok(selene_engine) = engine_builder.extract::<PySeleneEngineBuilder>(py) {
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(selene_engine.inner)));
                    Ok(PySimBuilder { inner: self.inner.clone() })
                } else {
                    Err(PyTypeError::new_err("For HUGR programs, classical() requires a SeleneEngineBuilder"))
                }
            }
            SimBuilderInner::PhirJson(sim_builder) => {
                if let Ok(phir_engine) = engine_builder.extract::<PyPhirJsonEngineBuilder>(py) {
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(phir_engine.inner)));
                    Ok(PySimBuilder { inner: self.inner.clone() })
                } else {
                    Err(PyTypeError::new_err("For PHIR JSON programs, classical() requires a PhirJsonEngineBuilder"))
                }
            }
        }
    }

    /// Set random seed
    fn seed(&mut self, seed: u64) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.seed = Some(seed),
            SimBuilderInner::Llvm(builder) => builder.seed = Some(seed),
            SimBuilderInner::Selene(builder) => builder.seed = Some(seed),
            SimBuilderInner::PhirJson(builder) => builder.seed = Some(seed),
        }
        Ok(PySimBuilder { inner: self.inner.clone() })
    }

    /// Set number of worker threads
    fn workers(&mut self, workers: usize) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.workers = Some(workers),
            SimBuilderInner::Llvm(builder) => builder.workers = Some(workers),
            SimBuilderInner::Selene(builder) => builder.workers = Some(workers),
            SimBuilderInner::PhirJson(builder) => builder.workers = Some(workers),
        }
        Ok(PySimBuilder { inner: self.inner.clone() })
    }

    /// Use automatic worker count based on available CPUs
    fn auto_workers(&mut self) -> PyResult<Self> {
        let workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        self.workers(workers)
    }

    /// Set quantum simulator/engine
    fn quantum(&mut self, engine: PyObject) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.quantum_engine_builder = Some(engine),
            SimBuilderInner::Llvm(builder) => builder.quantum_engine_builder = Some(engine),
            SimBuilderInner::Selene(builder) => builder.quantum_engine_builder = Some(engine),
            SimBuilderInner::PhirJson(builder) => builder.quantum_engine_builder = Some(engine),
        }
        Ok(PySimBuilder { inner: self.inner.clone() })
    }

    /// Set the number of qubits
    fn qubits(&mut self, num_qubits: usize) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::Llvm(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::Selene(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::PhirJson(builder) => builder.explicit_num_qubits = Some(num_qubits),
        }
        Ok(PySimBuilder { inner: self.inner.clone() })
    }

    /// Set noise model builder
    fn noise(&mut self, noise_builder: PyObject) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::Llvm(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::Selene(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::PhirJson(builder) => builder.noise_builder = Some(noise_builder),
        }
        Ok(PySimBuilder { inner: self.inner.clone() })
    }

    /// Run the simulation
    fn run(&self, shots: usize) -> PyResult<crate::shot_results_bindings::PyShotVec> {
        use crate::shot_results_bindings::PyShotVec;
        use crate::engine_builders::{PyStateVectorEngineBuilder, PySparseStabilizerEngineBuilder};
        use crate::engine_builders::{PyGeneralNoiseModelBuilder, PyDepolarizingNoiseModelBuilder, PyBiasedDepolarizingNoiseModelBuilder};
        use pyo3::exceptions::PyRuntimeError;
        
        match &self.inner {
            SimBuilderInner::Qasm(builder) => {
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let engine_builder = builder_lock.take()
                    .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
                
                // Create the Rust SimBuilder
                let mut sim_builder = engine_builder.to_sim();
                
                // Apply configuration
                if let Some(seed) = builder.seed {
                    sim_builder = sim_builder.seed(seed);
                }
                if let Some(workers) = builder.workers {
                    sim_builder = sim_builder.workers(workers);
                }
                if let Some(n) = builder.explicit_num_qubits {
                    sim_builder = sim_builder.qubits(n);
                }
                
                // Apply quantum engine builder if present
                if let Some(ref qe_py) = builder.quantum_engine_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                            if let Some(inner) = state_vec.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                // Apply noise builder if present
                if let Some(ref noise_py) = builder.noise_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(general.inner.clone()))
                        } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                // Run directly
                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {}", e))),
                }
            }
            SimBuilderInner::Llvm(builder) => {
                // Similar implementation for LLVM
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let engine_builder = builder_lock.take()
                    .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
                
                let mut sim_builder = engine_builder.to_sim();
                
                if let Some(seed) = builder.seed {
                    sim_builder = sim_builder.seed(seed);
                }
                if let Some(workers) = builder.workers {
                    sim_builder = sim_builder.workers(workers);
                }
                if let Some(n) = builder.explicit_num_qubits {
                    sim_builder = sim_builder.qubits(n);
                }
                
                // Apply quantum engine if present
                if let Some(ref qe_py) = builder.quantum_engine_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                            if let Some(inner) = state_vec.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                // Apply noise builder if present
                if let Some(ref noise_py) = builder.noise_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(general.inner.clone()))
                        } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {}", e))),
                }
            }
            SimBuilderInner::Selene(builder) => {
                // Similar implementation for Selene
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let mut engine_builder = builder_lock.take()
                    .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
                
                // Selene requires qubits to be set on the engine builder
                if let Some(n) = builder.explicit_num_qubits {
                    engine_builder = engine_builder.qubits(n);
                }
                
                let mut sim_builder = engine_builder.to_sim();
                
                if let Some(seed) = builder.seed {
                    sim_builder = sim_builder.seed(seed);
                }
                if let Some(workers) = builder.workers {
                    sim_builder = sim_builder.workers(workers);
                }
                // Note: qubits are already set on the engine builder for Selene
                
                // Apply quantum engine if present
                if let Some(ref qe_py) = builder.quantum_engine_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                            if let Some(inner) = state_vec.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                // Apply noise builder if present
                if let Some(ref noise_py) = builder.noise_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(general.inner.clone()))
                        } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {}", e))),
                }
            }
            SimBuilderInner::PhirJson(builder) => {
                // Similar implementation for PHIR JSON
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let engine_builder = builder_lock.take()
                    .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
                
                let mut sim_builder = engine_builder.to_sim();
                
                if let Some(seed) = builder.seed {
                    sim_builder = sim_builder.seed(seed);
                }
                if let Some(workers) = builder.workers {
                    sim_builder = sim_builder.workers(workers);
                }
                if let Some(n) = builder.explicit_num_qubits {
                    sim_builder = sim_builder.qubits(n);
                }
                
                // Apply quantum engine if present
                if let Some(ref qe_py) = builder.quantum_engine_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                            if let Some(inner) = state_vec.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed"
                                ))
                            }
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                // Apply noise builder if present
                if let Some(ref noise_py) = builder.noise_builder {
                    sim_builder = Python::with_gil(|py| -> PyResult<_> {
                        if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(general.inner.clone()))
                        } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }
                
                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {}", e))),
                }
            }
        }
    }

    /// Build the simulation (for multiple runs)
    fn build(&self) -> PyResult<PyObject> {
        use crate::engine_builders::{PyQasmSimulation, PyPhirJsonSimulation};
        use crate::engine_builders::{PyStateVectorEngineBuilder, PySparseStabilizerEngineBuilder};
        use crate::engine_builders::{PyGeneralNoiseModelBuilder, PyDepolarizingNoiseModelBuilder, PyBiasedDepolarizingNoiseModelBuilder};
        use pyo3::exceptions::PyRuntimeError;
        
        Python::with_gil(|py| {
            match &self.inner {
                SimBuilderInner::Qasm(builder) => {
                    let mut builder_lock = builder.engine_builder.lock().unwrap();
                    let engine_builder = builder_lock.take()
                        .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
                    
                    // Create the Rust SimBuilder
                    let mut sim_builder = engine_builder.to_sim();
                    
                    // Apply configuration
                    if let Some(seed) = builder.seed {
                        sim_builder = sim_builder.seed(seed);
                    }
                    if let Some(workers) = builder.workers {
                        sim_builder = sim_builder.workers(workers);
                    }
                    if let Some(n) = builder.explicit_num_qubits {
                        sim_builder = sim_builder.qubits(n);
                    }
                    
                    // Apply quantum engine builder if present
                    if let Some(ref qe_py) = builder.quantum_engine_builder {
                        sim_builder = Python::with_gil(|py| -> PyResult<_> {
                            if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                                if let Some(inner) = state_vec.inner.take() {
                                    Ok(sim_builder.quantum(inner))
                                } else {
                                    Err(PyErr::new::<PyRuntimeError, _>(
                                        "Quantum engine builder has already been consumed"
                                    ))
                                }
                            } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                                if let Some(inner) = sparse_stab.inner.take() {
                                    Ok(sim_builder.quantum(inner))
                                } else {
                                    Err(PyErr::new::<PyRuntimeError, _>(
                                        "Quantum engine builder has already been consumed"
                                    ))
                                }
                            } else {
                                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "quantum_engine must be a valid quantum engine builder"
                                ))
                            }
                        })?;
                    }
                    
                    // Apply noise builder if present
                    if let Some(ref noise_py) = builder.noise_builder {
                        sim_builder = Python::with_gil(|py| -> PyResult<_> {
                            if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                                Ok(sim_builder.noise(general.inner.clone()))
                            } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                                Ok(sim_builder.noise(depolarizing.inner.clone()))
                            } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                                Ok(sim_builder.noise(biased.inner.clone()))
                            } else {
                                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "noise must be a valid noise model builder"
                                ))
                            }
                        })?;
                    }
                    
                    // Build the MonteCarloEngine
                    let engine = sim_builder.build()
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to build simulation: {}", e)))?;
                    
                    Ok(Py::new(py, PyQasmSimulation {
                        inner: Arc::new(Mutex::new(engine)),
                    })?.into_any())
                }
                SimBuilderInner::PhirJson(builder) => {
                    // Similar implementation for PHIR JSON
                    let mut builder_lock = builder.engine_builder.lock().unwrap();
                    let engine_builder = builder_lock.take()
                        .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
                    
                    let mut sim_builder = engine_builder.to_sim();
                    
                    if let Some(seed) = builder.seed {
                        sim_builder = sim_builder.seed(seed);
                    }
                    if let Some(workers) = builder.workers {
                        sim_builder = sim_builder.workers(workers);
                    }
                    if let Some(n) = builder.explicit_num_qubits {
                        sim_builder = sim_builder.qubits(n);
                    }
                    
                    // TODO: Add quantum and noise builder support for PHIR JSON
                    
                    let engine = sim_builder.build()
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to build simulation: {}", e)))?;
                    
                    Ok(Py::new(py, PyPhirJsonSimulation {
                        inner: Arc::new(Mutex::new(engine)),
                    })?.into_any())
                }
                // LLVM and Selene don't have build() methods in current implementation
                SimBuilderInner::Llvm(_) => {
                    Err(PyRuntimeError::new_err("LLVM simulation does not support build() yet - use run() directly"))
                }
                SimBuilderInner::Selene(_) => {
                    Err(PyRuntimeError::new_err("Selene simulation does not support build() yet - use run() directly"))
                }
            }
        })
    }
}

// Clone implementations for the inner types
impl Clone for SimBuilderInner {
    fn clone(&self) -> Self {
        Python::with_gil(|py| {
            match self {
                SimBuilderInner::Qasm(builder) => SimBuilderInner::Qasm(PyQasmSimBuilder {
                    engine_builder: builder.engine_builder.clone(),
                    seed: builder.seed,
                    workers: builder.workers,
                    quantum_engine_builder: builder.quantum_engine_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    explicit_num_qubits: builder.explicit_num_qubits,
                }),
                SimBuilderInner::Llvm(builder) => SimBuilderInner::Llvm(PyLlvmSimBuilder {
                    engine_builder: builder.engine_builder.clone(),
                    seed: builder.seed,
                    workers: builder.workers,
                    quantum_engine_builder: builder.quantum_engine_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    explicit_num_qubits: builder.explicit_num_qubits,
                }),
                SimBuilderInner::Selene(builder) => SimBuilderInner::Selene(PySeleneSimBuilder {
                    engine_builder: builder.engine_builder.clone(),
                    seed: builder.seed,
                    workers: builder.workers,
                    quantum_engine_builder: builder.quantum_engine_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    explicit_num_qubits: builder.explicit_num_qubits,
                }),
                SimBuilderInner::PhirJson(builder) => SimBuilderInner::PhirJson(PyPhirJsonSimBuilder {
                    engine_builder: builder.engine_builder.clone(),
                    seed: builder.seed,
                    workers: builder.workers,
                    quantum_engine_builder: builder.quantum_engine_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    explicit_num_qubits: builder.explicit_num_qubits,
                }),
            }
        })
    }
}

/// Register the sim module
pub fn register_sim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySimBuilder>()?;
    m.add_function(wrap_pyfunction!(sim, m)?)?;
    Ok(())
}