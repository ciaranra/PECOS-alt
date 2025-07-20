//! PyO3 wrappers for engine builders following the unified simulation API
//!
//! This module provides thin wrappers around the Rust engine builders,
//! maintaining the same unified API pattern: engine().program(...).to_sim()

use pecos_engines::ClassicalControlEngineBuilder;
use pecos_llvm_sim::{llvm_engine as rust_llvm_engine, LlvmEngineBuilder as RustLlvmEngineBuilder};
use pecos_selene_ceng::{selene_engine as rust_selene_engine, SeleneEngineBuilder as RustSeleneEngineBuilder};
use pecos_qasm::{qasm_engine as rust_qasm_engine, QasmEngineBuilder as RustQasmEngineBuilder};
use pecos_phir_json::{phir_json_engine as rust_phir_json_engine, PhirJsonEngineBuilder as RustPhirJsonEngineBuilder};
use pecos_programs::{LlvmProgram, HugrProgram, QasmProgram, PhirJsonProgram};
use pecos_engines::quantum_engine_builder::{
    StateVectorEngineBuilder as RustStateVectorEngineBuilder,
    SparseStabilizerEngineBuilder as RustSparseStabilizerEngineBuilder,
    state_vector as rust_state_vector,
    sparse_stabilizer as rust_sparse_stabilizer,
};
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::sync::{Arc, Mutex};

// Import existing shot result types
use crate::shot_results_bindings::PyShotVec;

// Noise builder wrappers
use pecos_engines::noise::{
    GeneralNoiseModelBuilder,
    DepolarizingNoiseModelBuilder, 
    BiasedDepolarizingNoiseModelBuilder,
};

/// Python wrapper for QASM engine builder
#[pyclass(name = "QasmEngineBuilder")]
#[derive(Clone)]
pub struct PyQasmEngineBuilder {
    inner: RustQasmEngineBuilder,
}

#[pymethods]
impl PyQasmEngineBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: rust_qasm_engine(),
        }
    }

    /// Set the program for this engine
    #[pyo3(signature = (program))]
    fn program(&mut self, program: &PyQasmProgram) -> PyResult<Self> {
        self.inner = self.inner.clone().program(program.inner.clone());
        Ok(self.clone())
    }

    /// Convert to simulation builder
    fn to_sim(&self) -> PyResult<PyQasmSimBuilder> {
        Ok(PyQasmSimBuilder {
            engine_builder: Arc::new(Mutex::new(Some(self.inner.clone()))),
            seed: None,
            workers: None,
            quantum_engine_builder: None,
            noise_builder: None,
            explicit_num_qubits: None,
        })
    }
}

/// Python wrapper for LLVM engine builder
#[pyclass(name = "LlvmEngineBuilder")]
#[derive(Clone)]
pub struct PyLlvmEngineBuilder {
    inner: RustLlvmEngineBuilder,
}

#[pymethods]
impl PyLlvmEngineBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: rust_llvm_engine(),
        }
    }

    /// Set the program for this engine
    #[pyo3(signature = (program))]
    fn program(&mut self, program: PyObject, py: Python) -> PyResult<Self> {
        // Check if it's an LlvmProgram
        if let Ok(llvm_prog) = program.extract::<PyLlvmProgram>(py) {
            self.inner = self.inner.clone().program(llvm_prog.inner);
        }
        // Check if it's a HugrProgram
        else if let Ok(hugr_prog) = program.extract::<PyHugrProgram>(py) {
            self.inner = self.inner.clone().program(hugr_prog.inner);
        }
        else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "program must be either an LlvmProgram or HugrProgram instance"
            ));
        }
        Ok(self.clone())
    }

    /// Enable verbose output
    fn verbose(&mut self, verbose: bool) -> PyResult<Self> {
        self.inner = self.inner.clone().verbose(verbose);
        Ok(self.clone())
    }

    /// Convert to simulation builder
    fn to_sim(&self) -> PyResult<PyLlvmSimBuilder> {
        Ok(PyLlvmSimBuilder {
            engine_builder: Arc::new(Mutex::new(Some(self.inner.clone()))),
            seed: None,
            workers: None,
            quantum_engine_builder: None,
            noise_builder: None,
            explicit_num_qubits: None,
        })
    }
}

/// Python wrapper for Selene engine builder
#[pyclass(name = "SeleneEngineBuilder")]
#[derive(Clone)]
pub struct PySeleneEngineBuilder {
    inner: RustSeleneEngineBuilder,
}

#[pymethods]
impl PySeleneEngineBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: rust_selene_engine(),
        }
    }

    /// Set the program for this engine
    #[pyo3(signature = (program))]
    fn program(&mut self, program: PyObject, py: Python) -> PyResult<Self> {
        // Check if it's an LlvmProgram
        if let Ok(llvm_prog) = program.extract::<PyLlvmProgram>(py) {
            self.inner = self.inner.clone().program(llvm_prog.inner);
        }
        // Check if it's a HugrProgram
        else if let Ok(hugr_prog) = program.extract::<PyHugrProgram>(py) {
            self.inner = self.inner.clone().program(hugr_prog.inner);
        }
        else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "program must be either an LlvmProgram or HugrProgram instance"
            ));
        }
        Ok(self.clone())
    }

    /// Convert to simulation builder
    fn to_sim(&self) -> PyResult<PySeleneSimBuilder> {
        Ok(PySeleneSimBuilder {
            engine_builder: Arc::new(Mutex::new(Some(self.inner.clone()))),
            seed: None,
            workers: None,
            quantum_engine_builder: None,
            noise_builder: None,
            explicit_num_qubits: None,
        })
    }
}

/// Python wrapper for PHIR JSON engine builder
#[pyclass(name = "PhirJsonEngineBuilder")]
#[derive(Clone)]
pub struct PyPhirJsonEngineBuilder {
    inner: RustPhirJsonEngineBuilder,
}

#[pymethods]
impl PyPhirJsonEngineBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: rust_phir_json_engine(),
        }
    }

    /// Set the program for this engine
    #[pyo3(signature = (program))]
    fn program(&mut self, program: &PyPhirJsonProgram) -> PyResult<Self> {
        self.inner = self.inner.clone().program(program.inner.clone());
        Ok(self.clone())
    }

    /// Convert to simulation builder
    fn to_sim(&self) -> PyResult<PyPhirJsonSimBuilder> {
        Ok(PyPhirJsonSimBuilder {
            engine_builder: Arc::new(Mutex::new(Some(self.inner.clone()))),
            seed: None,
            workers: None,
            quantum_engine_builder: None,
            noise_builder: None,
            explicit_num_qubits: None,
        })
    }
}

/// Python wrapper for QASM simulation builder
/// 
/// This stores configuration and rebuilds the Rust SimBuilder when needed,
/// avoiding the FnOnce + Sync issue while maintaining the same API
#[pyclass(name = "QasmSimBuilder")]
pub struct PyQasmSimBuilder {
    engine_builder: Arc<Mutex<Option<RustQasmEngineBuilder>>>,
    seed: Option<u64>,
    workers: Option<usize>,
    quantum_engine_builder: Option<PyObject>,
    noise_builder: Option<PyObject>,
    explicit_num_qubits: Option<usize>,
}

#[pymethods]
impl PyQasmSimBuilder {
    /// Set random seed
    fn seed(slf: Py<Self>, seed: u64, py: Python) -> Py<Self> {
        slf.borrow_mut(py).seed = Some(seed);
        slf
    }

    /// Set number of worker threads
    fn workers(slf: Py<Self>, workers: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Use automatic worker count based on available CPUs
    fn auto_workers(slf: Py<Self>, py: Python) -> Py<Self> {
        let workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Set noise model builder
    fn noise(slf: Py<Self>, noise_builder: PyObject, py: Python) -> Py<Self> {
        slf.borrow_mut(py).noise_builder = Some(noise_builder);
        slf
    }

    /// Set quantum simulator/engine
    fn quantum(slf: Py<Self>, engine: PyObject, py: Python) -> PyResult<Py<Self>> {
        // Store the quantum engine builder object
        slf.borrow_mut(py).quantum_engine_builder = Some(engine);
        Ok(slf)
    }
    
    /// Set the number of qubits
    fn qubits(slf: Py<Self>, num_qubits: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).explicit_num_qubits = Some(num_qubits);
        slf
    }

    /// Build the simulation (for multiple runs)
    fn build(&self) -> PyResult<PyQasmSimulation> {
        let mut builder_lock = self.engine_builder.lock().unwrap();
        let engine_builder = builder_lock.take()
            .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
        
        // Create the Rust SimBuilder
        let mut sim_builder = engine_builder.to_sim();
        
        // Apply configuration
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }
        if let Some(workers) = self.workers {
            sim_builder = sim_builder.workers(workers);
        }
        if let Some(n) = self.explicit_num_qubits {
            sim_builder = sim_builder.qubits(n);
        }
        
        // Apply quantum engine builder if present
        if let Some(ref qe_py) = self.quantum_engine_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known quantum engine builder types
                if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                    if let Some(inner) = state_vec.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                    if let Some(inner) = sparse_stab.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
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
        if let Some(ref noise_py) = self.noise_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known noise builder types
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
        
        // Build the simulation
        let simulation = sim_builder.build()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to build simulation: {}", e)))?;
        
        Ok(PyQasmSimulation {
            inner: Arc::new(simulation),
        })
    }

    /// Run the simulation
    fn run(&self, shots: usize) -> PyResult<PyShotVec> {
        let mut builder_lock = self.engine_builder.lock().unwrap();
        let engine_builder = builder_lock.take()
            .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
        
        // Create the Rust SimBuilder
        let mut sim_builder = engine_builder.to_sim();
        
        // Apply configuration
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }
        if let Some(workers) = self.workers {
            sim_builder = sim_builder.workers(workers);
        }
        if let Some(n) = self.explicit_num_qubits {
            sim_builder = sim_builder.qubits(n);
        }
        
        // Apply quantum engine builder if present
        if let Some(ref qe_py) = self.quantum_engine_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known quantum engine builder types
                if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                    if let Some(inner) = state_vec.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                    if let Some(inner) = sparse_stab.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else {
                    // For run(), we can skip unknown quantum engine types and use default
                    Ok(sim_builder)
                }
            })?;
        }
        
        // Apply noise builder if present
        if let Some(ref noise_py) = self.noise_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(general.inner.clone()))
                } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(depolarizing.inner.clone()))
                } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(biased.inner.clone()))
                } else {
                    // For run(), we can skip unknown noise types
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
}

/// Python wrapper for built QASM simulation
#[pyclass(name = "QasmSimulation")]
pub struct PyQasmSimulation {
    inner: Arc<pecos_engines::Simulation<pecos_qasm::engine::QASMEngine>>,
}

#[pymethods]
impl PyQasmSimulation {
    /// Run the simulation
    fn run(&self, shots: usize) -> PyResult<PyShotVec> {
        match self.inner.run(shots) {
            Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
            Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {}", e))),
        }
    }
}

/// Python wrapper for built PHIR JSON simulation
#[pyclass(name = "PhirJsonSimulation")]
pub struct PyPhirJsonSimulation {
    inner: Arc<pecos_engines::Simulation<pecos_phir_json::PhirJsonEngine>>,
}

#[pymethods]
impl PyPhirJsonSimulation {
    /// Run the simulation
    fn run(&self, shots: usize) -> PyResult<PyShotVec> {
        match self.inner.run(shots) {
            Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
            Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {}", e))),
        }
    }
}

/// Python wrapper for LLVM simulation builder
#[pyclass(name = "LlvmSimBuilder")]
pub struct PyLlvmSimBuilder {
    engine_builder: Arc<Mutex<Option<RustLlvmEngineBuilder>>>,
    seed: Option<u64>,
    workers: Option<usize>,
    quantum_engine_builder: Option<PyObject>,
    noise_builder: Option<PyObject>,
    explicit_num_qubits: Option<usize>,
}

#[pymethods]
impl PyLlvmSimBuilder {
    /// Set random seed
    fn seed(slf: Py<Self>, seed: u64, py: Python) -> Py<Self> {
        slf.borrow_mut(py).seed = Some(seed);
        slf
    }

    /// Set number of worker threads
    fn workers(slf: Py<Self>, workers: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Use automatic worker count based on available CPUs
    fn auto_workers(slf: Py<Self>, py: Python) -> Py<Self> {
        let workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Set noise model builder
    fn noise(slf: Py<Self>, noise_builder: PyObject, py: Python) -> Py<Self> {
        slf.borrow_mut(py).noise_builder = Some(noise_builder);
        slf
    }

    /// Set quantum simulator/engine
    fn quantum(slf: Py<Self>, engine: PyObject, py: Python) -> PyResult<Py<Self>> {
        // Store the quantum engine builder object
        slf.borrow_mut(py).quantum_engine_builder = Some(engine);
        Ok(slf)
    }
    
    /// Set the number of qubits
    fn qubits(slf: Py<Self>, num_qubits: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).explicit_num_qubits = Some(num_qubits);
        slf
    }

    /// Run the simulation
    fn run(&self, shots: usize) -> PyResult<PyShotVec> {
        let mut builder_lock = self.engine_builder.lock().unwrap();
        let engine_builder = builder_lock.take()
            .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
        
        let mut sim_builder = engine_builder.to_sim();
        
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }
        if let Some(workers) = self.workers {
            sim_builder = sim_builder.workers(workers);
        }
        if let Some(n) = self.explicit_num_qubits {
            sim_builder = sim_builder.qubits(n);
        }
        
        // Apply quantum engine builder if present
        if let Some(ref qe_py) = self.quantum_engine_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known quantum engine builder types
                if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                    if let Some(inner) = state_vec.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                    if let Some(inner) = sparse_stab.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else {
                    // For run(), we can skip unknown quantum engine types and use default
                    Ok(sim_builder)
                }
            })?;
        }
        
        // Note: LLVM engine might support different noise models
        // For now, we'll handle the same ones as QASM
        if let Some(ref noise_py) = self.noise_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(general.inner.clone()))
                } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(depolarizing.inner.clone()))
                } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(biased.inner.clone()))
                } else {
                    // Skip unknown noise types
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

/// Python wrapper for Selene simulation builder
#[pyclass(name = "SeleneSimBuilder")]
pub struct PySeleneSimBuilder {
    engine_builder: Arc<Mutex<Option<RustSeleneEngineBuilder>>>,
    seed: Option<u64>,
    workers: Option<usize>,
    quantum_engine_builder: Option<PyObject>,
    noise_builder: Option<PyObject>,
    explicit_num_qubits: Option<usize>,
}

#[pymethods]
impl PySeleneSimBuilder {
    /// Set random seed
    fn seed(slf: Py<Self>, seed: u64, py: Python) -> Py<Self> {
        slf.borrow_mut(py).seed = Some(seed);
        slf
    }

    /// Set number of worker threads
    fn workers(slf: Py<Self>, workers: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Use automatic worker count based on available CPUs
    fn auto_workers(slf: Py<Self>, py: Python) -> Py<Self> {
        let workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Set noise model builder
    fn noise(slf: Py<Self>, noise_builder: PyObject, py: Python) -> Py<Self> {
        slf.borrow_mut(py).noise_builder = Some(noise_builder);
        slf
    }

    /// Set quantum simulator/engine
    fn quantum(slf: Py<Self>, engine: PyObject, py: Python) -> PyResult<Py<Self>> {
        // Store the quantum engine builder object
        slf.borrow_mut(py).quantum_engine_builder = Some(engine);
        Ok(slf)
    }
    
    /// Set the number of qubits
    fn qubits(slf: Py<Self>, num_qubits: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).explicit_num_qubits = Some(num_qubits);
        slf
    }

    /// Run the simulation
    fn run(&self, shots: usize) -> PyResult<PyShotVec> {
        let mut builder_lock = self.engine_builder.lock().unwrap();
        let engine_builder = builder_lock.take()
            .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
        
        let mut sim_builder = engine_builder.to_sim();
        
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }
        if let Some(workers) = self.workers {
            sim_builder = sim_builder.workers(workers);
        }
        if let Some(n) = self.explicit_num_qubits {
            sim_builder = sim_builder.qubits(n);
        }
        
        // Apply quantum engine builder if present
        if let Some(ref qe_py) = self.quantum_engine_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known quantum engine builder types
                if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                    if let Some(inner) = state_vec.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                    if let Some(inner) = sparse_stab.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else {
                    // For run(), we can skip unknown quantum engine types and use default
                    Ok(sim_builder)
                }
            })?;
        }
        
        // Selene might support different noise models
        if let Some(ref noise_py) = self.noise_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(general.inner.clone()))
                } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(depolarizing.inner.clone()))
                } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(biased.inner.clone()))
                } else {
                    // Skip unknown noise types
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

/// Python wrapper for PHIR JSON simulation builder
#[pyclass(name = "PhirJsonSimBuilder")]
pub struct PyPhirJsonSimBuilder {
    engine_builder: Arc<Mutex<Option<RustPhirJsonEngineBuilder>>>,
    seed: Option<u64>,
    workers: Option<usize>,
    quantum_engine_builder: Option<PyObject>,
    noise_builder: Option<PyObject>,
    explicit_num_qubits: Option<usize>,
}

#[pymethods]
impl PyPhirJsonSimBuilder {
    /// Set random seed
    fn seed(slf: Py<Self>, seed: u64, py: Python) -> Py<Self> {
        slf.borrow_mut(py).seed = Some(seed);
        slf
    }

    /// Set number of worker threads
    fn workers(slf: Py<Self>, workers: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Use automatic worker count based on available CPUs
    fn auto_workers(slf: Py<Self>, py: Python) -> Py<Self> {
        let workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        slf.borrow_mut(py).workers = Some(workers);
        slf
    }

    /// Set noise model builder
    fn noise(slf: Py<Self>, noise_builder: PyObject, py: Python) -> Py<Self> {
        slf.borrow_mut(py).noise_builder = Some(noise_builder);
        slf
    }

    /// Set quantum simulator/engine
    fn quantum(slf: Py<Self>, engine: PyObject, py: Python) -> PyResult<Py<Self>> {
        slf.borrow_mut(py).quantum_engine_builder = Some(engine);
        Ok(slf)
    }
    
    /// Set the number of qubits
    fn qubits(slf: Py<Self>, num_qubits: usize, py: Python) -> Py<Self> {
        slf.borrow_mut(py).explicit_num_qubits = Some(num_qubits);
        slf
    }

    /// Build the simulation (for multiple runs)
    fn build(&self) -> PyResult<PyPhirJsonSimulation> {
        let mut builder_lock = self.engine_builder.lock().unwrap();
        let engine_builder = builder_lock.take()
            .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
        
        // Create the Rust SimBuilder
        let mut sim_builder = engine_builder.to_sim();
        
        // Apply configuration
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }
        if let Some(workers) = self.workers {
            sim_builder = sim_builder.workers(workers);
        }
        if let Some(n) = self.explicit_num_qubits {
            sim_builder = sim_builder.qubits(n);
        }
        
        // Apply quantum engine builder if present
        if let Some(ref qe_py) = self.quantum_engine_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known quantum engine builder types
                if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                    if let Some(inner) = state_vec.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                    if let Some(inner) = sparse_stab.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
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
        if let Some(ref noise_py) = self.noise_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known noise builder types
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
        
        // Build the simulation
        let simulation = sim_builder.build()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to build simulation: {}", e)))?;
        
        Ok(PyPhirJsonSimulation {
            inner: Arc::new(simulation),
        })
    }

    /// Run the simulation immediately
    fn run(&self, shots: usize) -> PyResult<PyShotVec> {
        // Similar implementation to build() but calls run() instead
        let mut builder_lock = self.engine_builder.lock().unwrap();
        let engine_builder = builder_lock.take()
            .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;
        
        // Create the Rust SimBuilder
        let mut sim_builder = engine_builder.to_sim();
        
        // Apply configuration
        if let Some(seed) = self.seed {
            sim_builder = sim_builder.seed(seed);
        }
        if let Some(workers) = self.workers {
            sim_builder = sim_builder.workers(workers);
        }
        if let Some(n) = self.explicit_num_qubits {
            sim_builder = sim_builder.qubits(n);
        }
        
        // Apply quantum engine builder if present
        if let Some(ref qe_py) = self.quantum_engine_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known quantum engine builder types
                if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
                    if let Some(inner) = state_vec.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
                    if let Some(inner) = sparse_stab.inner.take() {
                        Ok(sim_builder.quantum(inner))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            "Quantum engine builder has already been consumed"
                        ))
                    }
                } else {
                    // For run(), we can skip unknown quantum engine types and use default
                    Ok(sim_builder)
                }
            })?;
        }
        
        // Apply noise builder if present
        if let Some(ref noise_py) = self.noise_builder {
            sim_builder = Python::with_gil(|py| -> PyResult<_> {
                // Try to extract known noise builder types
                if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(general.inner.clone()))
                } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(depolarizing.inner.clone()))
                } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
                    Ok(sim_builder.noise(biased.inner.clone()))
                } else {
                    // Skip unknown noise types
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

/// Python wrapper for program types
#[pyclass(name = "QasmProgram")]
#[derive(Clone)]
pub struct PyQasmProgram {
    pub(crate) inner: QasmProgram,
}

#[pymethods]
impl PyQasmProgram {
    #[staticmethod]
    fn from_string(source: String) -> Self {
        PyQasmProgram {
            inner: QasmProgram::from_string(source),
        }
    }
}

#[pyclass(name = "LlvmProgram")]
#[derive(Clone)]
pub struct PyLlvmProgram {
    pub(crate) inner: LlvmProgram,
}

#[pymethods]
impl PyLlvmProgram {
    #[staticmethod]
    fn from_string(source: String) -> Self {
        PyLlvmProgram {
            inner: LlvmProgram::from_string(source),
        }
    }
    
    #[staticmethod]
    fn from_ir(source: String) -> Self {
        PyLlvmProgram {
            inner: LlvmProgram::from_ir(source),
        }
    }
}

#[pyclass(name = "HugrProgram")]
#[derive(Clone)]
pub struct PyHugrProgram {
    pub(crate) inner: HugrProgram,
}

#[pymethods]
impl PyHugrProgram {
    #[staticmethod]
    fn from_bytes(bytes: Vec<u8>) -> Self {
        PyHugrProgram {
            inner: HugrProgram::from_bytes(bytes),
        }
    }
}

#[pyclass(name = "PhirJsonProgram")]
#[derive(Clone)]
pub struct PyPhirJsonProgram {
    pub(crate) inner: PhirJsonProgram,
}

#[pymethods]
impl PyPhirJsonProgram {
    #[staticmethod]
    fn from_string(source: String) -> Self {
        PyPhirJsonProgram {
            inner: PhirJsonProgram::from_string(source),
        }
    }
    
    #[staticmethod]
    fn from_json(source: String) -> Self {
        PyPhirJsonProgram {
            inner: PhirJsonProgram::from_json(source),
        }
    }
}

/// Create a QASM engine builder
#[pyfunction]
pub fn qasm_engine() -> PyQasmEngineBuilder {
    PyQasmEngineBuilder {
        inner: rust_qasm_engine(),
    }
}

/// Create an LLVM engine builder
#[pyfunction]
pub fn llvm_engine() -> PyLlvmEngineBuilder {
    PyLlvmEngineBuilder {
        inner: rust_llvm_engine(),
    }
}

/// Create a Selene engine builder
#[pyfunction]
pub fn selene_engine() -> PySeleneEngineBuilder {
    PySeleneEngineBuilder {
        inner: rust_selene_engine(),
    }
}

/// Create a PHIR JSON engine builder
#[pyfunction]
pub fn phir_json_engine() -> PyPhirJsonEngineBuilder {
    PyPhirJsonEngineBuilder {
        inner: rust_phir_json_engine(),
    }
}

/// Create a general noise model builder
#[pyfunction]
pub fn general_noise() -> PyGeneralNoiseModelBuilder {
    PyGeneralNoiseModelBuilder::new()
}

/// Create a depolarizing noise model builder
#[pyfunction]
pub fn depolarizing_noise() -> PyDepolarizingNoiseModelBuilder {
    PyDepolarizingNoiseModelBuilder::new()
}

/// Create a biased depolarizing noise model builder
#[pyfunction]
pub fn biased_depolarizing_noise() -> PyBiasedDepolarizingNoiseModelBuilder {
    PyBiasedDepolarizingNoiseModelBuilder::new()
}

/// Python wrapper for GeneralNoiseModelBuilder  
#[pyclass(name = "GeneralNoiseModelBuilder")]
#[derive(Clone)]
pub struct PyGeneralNoiseModelBuilder {
    pub(crate) inner: GeneralNoiseModelBuilder,
}

#[pymethods]
impl PyGeneralNoiseModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: GeneralNoiseModelBuilder::new()
        }
    }

    /// Set single-qubit gate error probability
    fn with_p1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p1_probability(p)
        })
    }

    /// Set two-qubit gate error probability
    fn with_p2_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p2_probability(p)
        })
    }
    
    /// Set preparation error probability
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }
    
    /// Set measurement error probability for |0⟩ state
    fn with_meas_0_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_0_probability(p)
        })
    }
    
    /// Set measurement error probability for |1⟩ state
    fn with_meas_1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_1_probability(p)
        })
    }
    
    /// Set seed for reproducibility
    fn with_seed(&self, seed: u64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_seed(seed)
        })
    }
}

/// Python wrapper for DepolarizingNoiseModelBuilder
#[pyclass(name = "DepolarizingNoiseModelBuilder")]
#[derive(Clone)]
pub struct PyDepolarizingNoiseModelBuilder {
    pub(crate) inner: DepolarizingNoiseModelBuilder,
}

#[pymethods]
impl PyDepolarizingNoiseModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: DepolarizingNoiseModelBuilder::new()
        }
    }

    /// Set preparation error probability
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }

    /// Set measurement error probability
    fn with_meas_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_probability(p)
        })
    }

    /// Set single-qubit gate error probability
    fn with_p1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p1_probability(p)
        })
    }

    /// Set two-qubit gate error probability
    fn with_p2_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p2_probability(p)
        })
    }
    
    /// Set uniform probability for all error types
    fn with_uniform_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_uniform_probability(p)
        })
    }
    
    /// Set seed for reproducibility
    fn with_seed(&self, seed: u64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_seed(seed)
        })
    }
}

/// Python wrapper for BiasedDepolarizingNoiseModelBuilder
#[pyclass(name = "BiasedDepolarizingNoiseModelBuilder")]
#[derive(Clone)]
pub struct PyBiasedDepolarizingNoiseModelBuilder {
    pub(crate) inner: BiasedDepolarizingNoiseModelBuilder,
}

#[pymethods]
impl PyBiasedDepolarizingNoiseModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: BiasedDepolarizingNoiseModelBuilder::new()
        }
    }

    /// Set preparation error probability
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }

    /// Set measurement 0->1 flip probability
    fn with_meas_0_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_0_probability(p)
        })
    }

    /// Set measurement 1->0 flip probability
    fn with_meas_1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_1_probability(p)
        })
    }

    /// Set single-qubit gate error probability
    fn with_p1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p1_probability(p)
        })
    }
    
    /// Set two-qubit gate error probability
    fn with_p2_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p2_probability(p)
        })
    }
    
    /// Set uniform probability for all error types
    fn with_uniform_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_uniform_probability(p)
        })
    }
    
    /// Set seed for reproducibility
    fn with_seed(&self, seed: u64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_seed(seed)
        })
    }
}

/// Python wrapper for StateVectorEngineBuilder
#[pyclass(name = "StateVectorEngineBuilder")]
#[derive(Clone)]
pub struct PyStateVectorEngineBuilder {
    pub(crate) inner: Option<RustStateVectorEngineBuilder>,
}

#[pymethods]
impl PyStateVectorEngineBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: Some(rust_state_vector())
        }
    }
    
    /// Set the number of qubits
    fn qubits(slf: Py<Self>, num_qubits: usize, py: Python) -> PyResult<Py<Self>> {
        let mut borrowed = slf.borrow_mut(py);
        if let Some(inner) = borrowed.inner.take() {
            borrowed.inner = Some(inner.qubits(num_qubits));
            drop(borrowed);
            Ok(slf)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Builder has already been consumed"
            ))
        }
    }
}

/// Python wrapper for SparseStabilizerEngineBuilder
#[pyclass(name = "SparseStabilizerEngineBuilder")]
#[derive(Clone)]
pub struct PySparseStabilizerEngineBuilder {
    pub(crate) inner: Option<RustSparseStabilizerEngineBuilder>,
}

#[pymethods]
impl PySparseStabilizerEngineBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: Some(rust_sparse_stabilizer())
        }
    }
    
    /// Set the number of qubits
    fn qubits(slf: Py<Self>, num_qubits: usize, py: Python) -> PyResult<Py<Self>> {
        let mut borrowed = slf.borrow_mut(py);
        if let Some(inner) = borrowed.inner.take() {
            borrowed.inner = Some(inner.qubits(num_qubits));
            drop(borrowed);
            Ok(slf)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Builder has already been consumed"
            ))
        }
    }
}

/// Create a state vector quantum engine builder
#[pyfunction]
pub fn state_vector() -> PyStateVectorEngineBuilder {
    PyStateVectorEngineBuilder::new()
}

/// Create a sparse stabilizer quantum engine builder
#[pyfunction]
pub fn sparse_stabilizer() -> PySparseStabilizerEngineBuilder {
    PySparseStabilizerEngineBuilder::new()
}

/// Alias for sparse_stabilizer
#[pyfunction]
pub fn sparse_stab() -> PySparseStabilizerEngineBuilder {
    sparse_stabilizer()
}

/// Register the engine builder module with PyO3
pub fn register_engine_builders(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Engine builders
    m.add_class::<PyQasmEngineBuilder>()?;
    m.add_class::<PyLlvmEngineBuilder>()?;
    m.add_class::<PySeleneEngineBuilder>()?;
    m.add_class::<PyPhirJsonEngineBuilder>()?;
    
    // Simulation builders
    m.add_class::<PyQasmSimBuilder>()?;
    m.add_class::<PyLlvmSimBuilder>()?;
    m.add_class::<PySeleneSimBuilder>()?;
    m.add_class::<PyPhirJsonSimBuilder>()?;
    
    // Built simulations
    m.add_class::<PyQasmSimulation>()?;
    m.add_class::<PyPhirJsonSimulation>()?;
    
    // Program types
    m.add_class::<PyQasmProgram>()?;
    m.add_class::<PyLlvmProgram>()?;
    m.add_class::<PyHugrProgram>()?;
    m.add_class::<PyPhirJsonProgram>()?;
    
    // Noise builders
    m.add_class::<PyGeneralNoiseModelBuilder>()?;
    m.add_class::<PyDepolarizingNoiseModelBuilder>()?;
    m.add_class::<PyBiasedDepolarizingNoiseModelBuilder>()?;
    
    // Quantum engine builders
    m.add_class::<PyStateVectorEngineBuilder>()?;
    m.add_class::<PySparseStabilizerEngineBuilder>()?;
    
    // Engine functions
    m.add_function(wrap_pyfunction!(qasm_engine, m)?)?;
    m.add_function(wrap_pyfunction!(llvm_engine, m)?)?;
    m.add_function(wrap_pyfunction!(selene_engine, m)?)?;
    m.add_function(wrap_pyfunction!(phir_json_engine, m)?)?;
    
    // Noise builder functions
    m.add_function(wrap_pyfunction!(general_noise, m)?)?;
    m.add_function(wrap_pyfunction!(depolarizing_noise, m)?)?;
    m.add_function(wrap_pyfunction!(biased_depolarizing_noise, m)?)?;
    
    // Quantum engine builder functions
    m.add_function(wrap_pyfunction!(state_vector, m)?)?;
    m.add_function(wrap_pyfunction!(sparse_stabilizer, m)?)?;
    m.add_function(wrap_pyfunction!(sparse_stab, m)?)?;
    
    Ok(())
}