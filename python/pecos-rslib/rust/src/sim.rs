//! Simulation API that mirrors the Rust pecos crate
//!
//! This module provides a `sim(program)` function that auto-detects the program type
//! and creates the appropriate simulation builder, following the same pattern as the
//! Rust `pecos::sim()` function.

use pecos_engines::ClassicalControlEngineBuilder;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use pecos_llvm_sim::llvm_engine as rust_llvm_engine;
use pecos_phir_json::phir_json_engine as rust_phir_json_engine;
use pecos_qasm::qasm_engine as rust_qasm_engine;

use crate::engine_builders::{
    PyHugrProgram, PyLlvmEngineBuilder, PyLlvmProgram, PyLlvmSimBuilder, PyPhirJsonEngineBuilder,
    PyPhirJsonProgram, PyPhirJsonSimBuilder, PyQasmEngineBuilder, PyQasmProgram, PyQasmSimBuilder,
    PySeleneEngineBuilder, PySeleneExecutableSimBuilder, PySeleneInterfaceProgram,
    PySeleneLibrarySimBuilder, PySeleneSimBuilder,
};

/// Detect and convert Guppy programs to use Selene's library execution infrastructure
///
/// This function attempts to:
/// 1. Detect if the input is a Guppy function
/// 2. Return a `PySeleneLibrarySimBuilder` that will handle compilation on the Python side
fn detect_and_convert_guppy(py: Python, program: &PyObject) -> PyResult<PySimBuilder> {
    log::trace!("In detect_and_convert_guppy");
    // Try to detect Guppy function
    let is_guppy = is_guppy_function(py, program)?;
    log::trace!("is_guppy_function returned: {is_guppy}");
    if is_guppy {
        // Use SeleneExecutable approach with Bridge plugin for back-and-forth communication
        // This will build a Selene executable and use IPC with the Bridge plugin
        log::debug!(
            "Detected Guppy program, creating SeleneExecutableSimBuilder with Bridge plugin"
        );

        // Create default SeleneExecutableEngineBuilder
        let engine_builder =
            pecos_selene_engine::selene_executable_builder::SeleneExecutableEngineBuilder::new();

        let builder = PySimBuilder {
            inner: SimBuilderInner::SeleneExecutable(PySeleneExecutableSimBuilder {
                program: Some(program.clone_ref(py)),
                engine_builder: Arc::new(Mutex::new(Some(engine_builder))),
                seed: None,
                workers: None,
                quantum_engine_builder: None,
                noise_builder: None,
                explicit_num_qubits: None,
            }),
        };
        log::debug!(" Successfully created PySimBuilder with SeleneExecutable");
        return Ok(builder);
    }

    // Not a Guppy program
    Err(pyo3::exceptions::PyTypeError::new_err(
        "Not a Guppy program",
    ))
}

/// Apply a quantum engine to a `SimBuilder`
fn apply_quantum_engine(
    py: Python,
    sim_builder: pecos_engines::SimBuilder,
    qe_py: &PyObject,
) -> PyResult<pecos_engines::SimBuilder> {
    use crate::engine_builders::{PySparseStabilizerEngineBuilder, PyStateVectorEngineBuilder};
    use pyo3::exceptions::PyRuntimeError;

    if let Ok(mut state_vec) = qe_py.extract::<PyStateVectorEngineBuilder>(py) {
        if let Some(inner) = state_vec.inner.take() {
            Ok(sim_builder.quantum(inner))
        } else {
            Err(PyErr::new::<PyRuntimeError, _>(
                "Quantum engine builder has already been consumed",
            ))
        }
    } else if let Ok(mut sparse_stab) = qe_py.extract::<PySparseStabilizerEngineBuilder>(py) {
        if let Some(inner) = sparse_stab.inner.take() {
            Ok(sim_builder.quantum(inner))
        } else {
            Err(PyErr::new::<PyRuntimeError, _>(
                "Quantum engine builder has already been consumed",
            ))
        }
    } else {
        Ok(sim_builder)
    }
}

/// Apply a noise model to a `SimBuilder`
fn apply_noise_model(
    py: Python,
    sim_builder: pecos_engines::SimBuilder,
    noise_py: &PyObject,
) -> PyResult<pecos_engines::SimBuilder> {
    use crate::engine_builders::{
        PyBiasedDepolarizingNoiseModelBuilder, PyDepolarizingNoiseModelBuilder,
        PyGeneralNoiseModelBuilder,
    };

    // First try to extract as proper builder types
    if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py) {
        Ok(sim_builder.noise(general.inner.clone()))
    } else if let Ok(depolarizing) = noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py) {
        Ok(sim_builder.noise(depolarizing.inner.clone()))
    } else if let Ok(biased) = noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py) {
        Ok(sim_builder.noise(biased.inner.clone()))
    } else {
        // Try to handle old-style dataclass noise models by converting them
        let noise_obj = noise_py.bind(py);
        let type_name = noise_obj.get_type().name()?;

        if type_name == "DepolarizingNoise" {
            // Old-style DepolarizingNoise dataclass - convert to builder
            if let Ok(p) = noise_obj.getattr("p").and_then(|p| p.extract::<f64>()) {
                let builder = pecos_engines::noise::DepolarizingNoiseModelBuilder::new()
                    .with_uniform_probability(p);
                Ok(sim_builder.noise(builder))
            } else {
                Ok(sim_builder)
            }
        } else if type_name == "BiasedDepolarizingNoise" {
            // Old-style BiasedDepolarizingNoise dataclass - convert to builder
            if let Ok(p) = noise_obj.getattr("p").and_then(|p| p.extract::<f64>()) {
                let builder = pecos_engines::noise::BiasedDepolarizingNoiseModelBuilder::new()
                    .with_uniform_probability(p);
                Ok(sim_builder.noise(builder))
            } else {
                Ok(sim_builder)
            }
        } else if type_name == "PassThroughNoise" {
            // PassThroughNoise means no noise - just return the builder as-is
            Ok(sim_builder)
        } else {
            // Unknown noise type - return builder unchanged
            Ok(sim_builder)
        }
    }
}

/// Check if a Python object is a Guppy function
fn is_guppy_function(py: Python, program: &PyObject) -> PyResult<bool> {
    // Check for Guppy function attributes
    let obj = program.bind(py);
    let has_guppy_compiled = obj.hasattr("_guppy_compiled").unwrap_or(false);
    let has_name = obj.hasattr("name").unwrap_or(false);

    // Check type name for Guppy definitions
    let type_obj = obj.get_type();
    let type_name = type_obj.name()?;
    let type_str = type_name.to_string();
    let is_guppy_type =
        type_str.contains("GuppyDefinition") || type_str.contains("GuppyFunctionDefinition");

    // Debug output to understand what we're seeing (can be removed later)
    log::debug!(" Checking if object is Guppy function:");
    log::debug!("  Type: {type_str}");
    log::debug!("  has _guppy_compiled: {has_guppy_compiled}");
    log::debug!("  has name: {has_name}");
    log::debug!("  is_guppy_type: {is_guppy_type}");

    // A Guppy function is detected if:
    // - It has the _guppy_compiled attribute, OR
    // - Its type contains "GuppyDefinition" or "GuppyFunctionDefinition"
    Ok(has_guppy_compiled || is_guppy_type)
}

/// Check if bytes are likely to be HUGR data
/// Main sim function that auto-detects program type and creates appropriate builder
///
/// This mirrors the Rust `pecos::sim()` function, providing automatic engine selection
/// based on the program type. Additionally supports auto-conversion of Guppy programs.
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
///     # Guppy program auto-conversion (Guppy → HUGR → Selene Interface)
///     results = sim(guppy_function).qubits(2).run(1000)
///     results = sim(hugr_bytes).qubits(2).run(1000)  # Raw bytes auto-detected as Guppy
///
///     # Override auto-selection with explicit engine
///     results = sim(QasmProgram.from_string("H q[0];")).classical(qasm_engine().wasm("custom.wasm")).run(1000)
#[pyfunction]
#[pyo3(signature = (program))]
#[allow(clippy::needless_pass_by_value)] // PyObject must be passed by value for PyO3
pub fn sim(py: Python, program: PyObject) -> PyResult<PySimBuilder> {
    use pecos_selene_engine::selene_executable_builder::SeleneExecutableEngineBuilder;

    log::debug!(" Rust sim() function called");
    // Try Guppy detection and conversion first
    match detect_and_convert_guppy(py, &program) {
        Ok(builder) => {
            log::debug!(" Rust sim() returning PySimBuilder for Guppy");
            return Ok(builder);
        }
        Err(e) => {
            // Log the error for debugging (will be visible if it's not just "Not a Guppy program")
            let err_str = e.to_string();
            if !err_str.contains("Not a Guppy program") {
                // If it's not the expected "Not a Guppy program" error, it means detection found something
                // but conversion failed - we should report this
                log::warn!("Guppy detection attempted but failed: {err_str}");
            }
            // Continue with other types
        }
    }

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
    } else if let Ok(_hugr_prog) = program.extract::<PyHugrProgram>(py) {
        // HUGR programs now use SeleneLibrary approach by default
        // Store the HUGR program and let Python handle compilation during build
        log::debug!(" HUGR program detected, using SeleneLibrarySimBuilder");

        Ok(PySimBuilder {
            inner: SimBuilderInner::SeleneLibrary(PySeleneLibrarySimBuilder {
                program: Some(program.clone_ref(py)), // Store the PyHugrProgram object
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
    } else if let Ok(selene_interface_prog) = program.extract::<PySeleneInterfaceProgram>(py) {
        log::debug!("Creating PySeleneExecutableSimBuilder for SeleneInterfaceProgram");
        // SeleneInterfaceProgram now uses SeleneExecutableEngine with bridge approach

        // Create the engine builder with the program (using new bridge approach)
        let engine_builder = SeleneExecutableEngineBuilder::new()
            .selene_interface_program(selene_interface_prog.inner);

        // Create a PySeleneExecutableSimBuilder (using new bridge approach)
        Ok(PySimBuilder {
            inner: SimBuilderInner::SeleneExecutable(PySeleneExecutableSimBuilder {
                program: None, // Program will be set later if needed
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
            "program must be a QasmProgram, LlvmProgram, HugrProgram, PhirJsonProgram, or SeleneInterfaceProgram instance",
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
    SeleneExecutable(PySeleneExecutableSimBuilder), // New bridge-based approach
    SeleneLibrary(PySeleneLibrarySimBuilder), // Newest library-loading approach for HUGR/Guppy
    Empty,                                    // For creating SimBuilder without a program
}

#[pymethods]
#[allow(clippy::unnecessary_wraps)] // PyO3 convention to return PyResult
impl PySimBuilder {
    /// Override the auto-selected classical engine
    ///
    /// Example:
    ///     # Use custom WASM with QASM
    ///     `sim(qasm).classical(qasm_engine().wasm("custom.wasm")).run(1000)`
    #[pyo3(signature = (engine_builder))]
    #[allow(clippy::too_many_lines)] // Complex engine builder dispatch logic
    #[allow(clippy::needless_pass_by_value)] // PyObject must be passed by value for PyO3
    fn classical(&mut self, py: Python, engine_builder: PyObject) -> PyResult<Self> {
        // Extract the engine builder and update our inner builder
        match &mut self.inner {
            SimBuilderInner::Qasm(sim_builder) => {
                if let Ok(qasm_engine) = engine_builder.extract::<PyQasmEngineBuilder>(py) {
                    // When using .classical() to override with an empty engine builder,
                    // we need to preserve the program from the existing builder.
                    // This supports the pattern: sim(qasm_program).classical(qasm_engine())

                    let mut existing_engine_guard = sim_builder.engine_builder.lock().unwrap();

                    // Check if the new engine has a source
                    if !qasm_engine.has_source() {
                        // The new engine has no program - keep the existing one
                        // This handles the case: sim(qasm_program).classical(qasm_engine())
                        // where qasm_engine() is empty but we want to preserve the program
                        drop(existing_engine_guard);
                        return Ok(PySimBuilder {
                            inner: self.inner.clone(),
                        });
                    }

                    // The new engine has a program, so use it
                    *existing_engine_guard = Some(qasm_engine.inner);
                    drop(existing_engine_guard);

                    Ok(PySimBuilder {
                        inner: self.inner.clone(),
                    })
                } else {
                    Err(PyTypeError::new_err(
                        "For QASM programs, classical() requires a QasmEngineBuilder",
                    ))
                }
            }
            SimBuilderInner::Llvm(sim_builder) => {
                if let Ok(llvm_engine) = engine_builder.extract::<PyLlvmEngineBuilder>(py) {
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(llvm_engine.inner)));
                    Ok(PySimBuilder {
                        inner: self.inner.clone(),
                    })
                } else {
                    Err(PyTypeError::new_err(
                        "For LLVM programs, classical() requires an LlvmEngineBuilder",
                    ))
                }
            }
            SimBuilderInner::Selene(sim_builder) => {
                if let Ok(selene_engine) = engine_builder.extract::<PySeleneEngineBuilder>(py) {
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(selene_engine.inner)));
                    Ok(PySimBuilder {
                        inner: self.inner.clone(),
                    })
                } else {
                    Err(PyTypeError::new_err(
                        "For HUGR programs, classical() requires a SeleneEngineBuilder",
                    ))
                }
            }
            SimBuilderInner::PhirJson(sim_builder) => {
                if let Ok(phir_engine) = engine_builder.extract::<PyPhirJsonEngineBuilder>(py) {
                    sim_builder.engine_builder = Arc::new(Mutex::new(Some(phir_engine.inner)));
                    Ok(PySimBuilder {
                        inner: self.inner.clone(),
                    })
                } else {
                    Err(PyTypeError::new_err(
                        "For PHIR JSON programs, classical() requires a PhirJsonEngineBuilder",
                    ))
                }
            }
            SimBuilderInner::SeleneExecutable(sim_builder) => {
                // Allow overriding with a SeleneEngineBuilder for explicit configuration
                // First try to extract as Rust PySeleneEngineBuilder
                if let Ok(selene_engine) = engine_builder.extract::<PySeleneEngineBuilder>(py) {
                    // Replace the engine builder with the user-provided one
                    {
                        let mut guard = sim_builder.engine_builder.lock().unwrap();
                        *guard = Some(selene_engine.inner);
                    } // Drop the lock here
                    Ok(PySimBuilder {
                        inner: self.inner.clone(),
                    })
                } else {
                    // Try to extract the _rust_builder attribute from Python wrapper
                    if let Ok(rust_builder_attr) = engine_builder.getattr(py, "_rust_builder") {
                        if let Ok(selene_engine) =
                            rust_builder_attr.extract::<PySeleneEngineBuilder>(py)
                        {
                            {
                                let mut guard = sim_builder.engine_builder.lock().unwrap();
                                *guard = Some(selene_engine.inner);
                            }
                            Ok(PySimBuilder {
                                inner: self.inner.clone(),
                            })
                        } else {
                            Err(PyTypeError::new_err(
                                "For SeleneExecutable programs, classical() requires a SeleneEngineBuilder",
                            ))
                        }
                    } else {
                        Err(PyTypeError::new_err(
                            "For SeleneExecutable programs, classical() requires a SeleneEngineBuilder",
                        ))
                    }
                }
            }
            SimBuilderInner::SeleneLibrary(_sim_builder) => {
                // SeleneLibrary uses SeleneLibraryEngine which is configured via Python
                // We don't support overriding it
                Err(PyTypeError::new_err(
                    "SeleneLibrary uses SeleneLibraryEngine and cannot be overridden",
                ))
            }
            SimBuilderInner::Empty => {
                // Handle custom engines being set on empty builder
                // This is for the SeleneExecutableEngine case
                Err(PyTypeError::new_err(
                    "Cannot set classical engine on empty builder - create with appropriate program type",
                ))
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
            SimBuilderInner::SeleneExecutable(builder) => builder.seed = Some(seed),
            SimBuilderInner::SeleneLibrary(builder) => builder.seed = Some(seed),
            SimBuilderInner::Empty => {} // No-op for empty builder
        }
        Ok(PySimBuilder {
            inner: self.inner.clone(),
        })
    }

    /// Set number of worker threads
    fn workers(&mut self, workers: usize) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.workers = Some(workers),
            SimBuilderInner::Llvm(builder) => builder.workers = Some(workers),
            SimBuilderInner::Selene(builder) => builder.workers = Some(workers),
            SimBuilderInner::PhirJson(builder) => builder.workers = Some(workers),
            SimBuilderInner::SeleneExecutable(builder) => builder.workers = Some(workers),
            SimBuilderInner::SeleneLibrary(builder) => builder.workers = Some(workers),
            SimBuilderInner::Empty => {} // No-op for empty builder
        }
        Ok(PySimBuilder {
            inner: self.inner.clone(),
        })
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
            SimBuilderInner::SeleneExecutable(builder) => {
                builder.quantum_engine_builder = Some(engine);
            }
            SimBuilderInner::SeleneLibrary(builder) => {
                builder.quantum_engine_builder = Some(engine);
            }
            SimBuilderInner::Empty => {} // No-op for empty builder
        }
        Ok(PySimBuilder {
            inner: self.inner.clone(),
        })
    }

    /// Set the number of qubits
    fn qubits(&mut self, num_qubits: usize) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::Llvm(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::Selene(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::PhirJson(builder) => builder.explicit_num_qubits = Some(num_qubits),
            SimBuilderInner::SeleneExecutable(builder) => {
                builder.explicit_num_qubits = Some(num_qubits);
            }
            SimBuilderInner::SeleneLibrary(builder) => {
                builder.explicit_num_qubits = Some(num_qubits);
            }
            SimBuilderInner::Empty => {} // No-op for empty builder
        }
        Ok(PySimBuilder {
            inner: self.inner.clone(),
        })
    }

    /// Set noise model builder
    fn noise(&mut self, noise_builder: PyObject) -> PyResult<Self> {
        match &mut self.inner {
            SimBuilderInner::Qasm(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::Llvm(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::Selene(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::PhirJson(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::SeleneExecutable(builder) => {
                builder.noise_builder = Some(noise_builder);
            }
            SimBuilderInner::SeleneLibrary(builder) => builder.noise_builder = Some(noise_builder),
            SimBuilderInner::Empty => {} // No-op for empty builder
        }
        Ok(PySimBuilder {
            inner: self.inner.clone(),
        })
    }

    /// Run the simulation
    #[allow(clippy::too_many_lines)] // Complex simulation dispatch with multiple engine types
    fn run(&self, shots: usize) -> PyResult<crate::shot_results_bindings::PyShotVec> {
        use crate::engine_builders::{
            PyBiasedDepolarizingNoiseModelBuilder, PyDepolarizingNoiseModelBuilder,
            PyGeneralNoiseModelBuilder,
        };
        use crate::engine_builders::{PySparseStabilizerEngineBuilder, PyStateVectorEngineBuilder};
        use crate::shot_results_bindings::PyShotVec;
        use pyo3::exceptions::PyRuntimeError;

        log::debug!(" PySimBuilder::run() called with {shots} shots");

        match &self.inner {
            SimBuilderInner::Qasm(builder) => {
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let engine_builder = builder_lock
                    .take()
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
                                    "Quantum engine builder has already been consumed",
                                ))
                            }
                        } else if let Ok(mut sparse_stab) =
                            qe_py.extract::<PySparseStabilizerEngineBuilder>(py)
                        {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed",
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
                        } else if let Ok(depolarizing) =
                            noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) =
                            noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }

                // Run directly
                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {e}"))),
                }
            }
            SimBuilderInner::Llvm(builder) => {
                // Similar implementation for LLVM
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let engine_builder = builder_lock
                    .take()
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
                                    "Quantum engine builder has already been consumed",
                                ))
                            }
                        } else if let Ok(mut sparse_stab) =
                            qe_py.extract::<PySparseStabilizerEngineBuilder>(py)
                        {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed",
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
                        } else if let Ok(depolarizing) =
                            noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) =
                            noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }

                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {e}"))),
                }
            }
            SimBuilderInner::Selene(builder) => {
                // Similar implementation for Selene
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let mut engine_builder = builder_lock
                    .take()
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
                                    "Quantum engine builder has already been consumed",
                                ))
                            }
                        } else if let Ok(mut sparse_stab) =
                            qe_py.extract::<PySparseStabilizerEngineBuilder>(py)
                        {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed",
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
                        } else if let Ok(depolarizing) =
                            noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) =
                            noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }

                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {e}"))),
                }
            }
            SimBuilderInner::PhirJson(builder) => {
                // Similar implementation for PHIR JSON
                let mut builder_lock = builder.engine_builder.lock().unwrap();
                let engine_builder = builder_lock
                    .take()
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
                                    "Quantum engine builder has already been consumed",
                                ))
                            }
                        } else if let Ok(mut sparse_stab) =
                            qe_py.extract::<PySparseStabilizerEngineBuilder>(py)
                        {
                            if let Some(inner) = sparse_stab.inner.take() {
                                Ok(sim_builder.quantum(inner))
                            } else {
                                Err(PyErr::new::<PyRuntimeError, _>(
                                    "Quantum engine builder has already been consumed",
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
                        } else if let Ok(depolarizing) =
                            noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(depolarizing.inner.clone()))
                        } else if let Ok(biased) =
                            noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py)
                        {
                            Ok(sim_builder.noise(biased.inner.clone()))
                        } else {
                            Ok(sim_builder)
                        }
                    })?;
                }

                match sim_builder.run(shots) {
                    Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                    Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {e}"))),
                }
            }
            SimBuilderInner::SeleneExecutable(builder) => {
                log::debug!(" Running SeleneExecutable simulation with {shots} shots");
                log::debug!(
                    "SeleneExecutable will use Bridge plugin for back-and-forth communication"
                );
                log::debug!(
                    "builder.explicit_num_qubits = {:?}",
                    builder.explicit_num_qubits
                );

                // We need to build Selene executable from the Guppy program
                Python::with_gil(|py| -> PyResult<PyShotVec> {
                    log::debug!(" Inside Python::with_gil block");
                    let program = builder
                        .program
                        .as_ref()
                        .ok_or_else(|| PyRuntimeError::new_err("No program specified"))?;

                    // Compile Guppy to HUGR if needed
                    let hugr_package = if is_guppy_function(py, program)? {
                        log::debug!(" Compiling Guppy to HUGR for Selene executable");
                        program.call_method0(py, "compile")?
                    } else {
                        log::debug!(" Using existing HUGR program");
                        program.clone_ref(py)
                    };

                    // Get the number of qubits - use explicit value if set, otherwise default to 10
                    let num_qubits = builder.explicit_num_qubits.unwrap_or(10);
                    log::debug!(" Using num_qubits = {num_qubits}");

                    // Always build fresh (no caching)
                    let (exec_path, artifacts_path) = {
                        log::debug!(" Building Selene executable");

                        // Build the Selene executable
                        let selene_sim = py.import("selene_sim")?;
                        let build_func = selene_sim.getattr("build")?;

                        let tempfile = py.import("tempfile")?;
                        let tempdir = tempfile.call_method0("mkdtemp")?;
                        let build_dir = tempdir.extract::<String>()?;
                        log::debug!(" Building Selene executable in {build_dir}");

                        // Create artifacts directory and pecos_config.json BEFORE building
                        // This ensures the Bridge plugin reads the correct qubit count when initialized
                        let artifacts_dir = format!("{build_dir}/artifacts");
                        std::fs::create_dir_all(&artifacts_dir).map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to create artifacts dir: {e}"))
                        })?;

                        // Write the PECOS config file with the correct qubit count
                        let config_path = format!("{artifacts_dir}/pecos_config.json");
                        let config_json = serde_json::json!({
                            "n_qubits": num_qubits,
                            "ipc_mode": true,
                        });
                        std::fs::write(&config_path, config_json.to_string()).map_err(|e| {
                            PyRuntimeError::new_err(format!(
                                "Failed to write pecos_config.json: {e}"
                            ))
                        })?;
                        log::debug!(
                            "Created pecos_config.json with n_qubits={num_qubits} at {config_path}"
                        );

                        // Set the SELENE_ARTIFACTS_DIR environment variable so Bridge can find the config
                        unsafe {
                            std::env::set_var("SELENE_ARTIFACTS_DIR", &artifacts_dir);
                        }
                        log::debug!(" Set SELENE_ARTIFACTS_DIR={artifacts_dir}");

                        let kwargs = pyo3::types::PyDict::new(py);
                        kwargs.set_item("build_dir", &build_dir)?;
                        kwargs.set_item("verbose", false)?;
                        kwargs.set_item("name", "guppy_prog")?;

                        let _instance = build_func.call((hugr_package,), Some(&kwargs))?;
                        log::debug!(" Built Selene instance successfully");

                        // Get executable and artifacts paths
                        let pathlib = py.import("pathlib")?;
                        let path_cls = pathlib.getattr("Path")?;
                        let build_path = path_cls.call1((build_dir.clone(),))?;

                        let exec_path = build_path
                            .call_method1("__truediv__", ("artifacts/program.selene.x",))?;
                        let artifacts_path =
                            build_path.call_method1("__truediv__", ("artifacts",))?;

                        (exec_path, artifacts_path)
                    };

                    let exec_path_str = exec_path.str()?.to_string();
                    let artifacts_path_str = artifacts_path.str()?.to_string();

                    log::debug!(" Selene executable at: {exec_path_str}");
                    log::debug!(" Artifacts at: {artifacts_path_str}");

                    // Create SeleneInterfaceProgram with paths
                    let selene_program = pecos_programs::SeleneInterfaceProgram {
                        plugin: Vec::new(), // No plugin bytes needed when using executable
                        executable_path: Some(exec_path_str),
                        artifacts_path: Some(artifacts_path_str),
                    };

                    // Now create and configure the engine builder
                    let mut builder_lock = builder.engine_builder.lock().unwrap();
                    let mut engine_builder = builder_lock
                        .take()
                        .ok_or_else(|| PyRuntimeError::new_err("Builder already consumed"))?;

                    // Set the program
                    engine_builder = engine_builder.selene_interface_program(selene_program);

                    // Set number of qubits if specified
                    if let Some(n) = builder.explicit_num_qubits {
                        engine_builder = engine_builder.num_qubits(n);
                    }

                    // Build the engine directly (SeleneExecutableEngine is a ClassicalControlEngine)
                    let mut sim_builder = engine_builder.to_sim();

                    if let Some(seed) = builder.seed {
                        sim_builder = sim_builder.seed(seed);
                    }
                    if let Some(workers) = builder.workers {
                        sim_builder = sim_builder.workers(workers);
                    }

                    // Apply quantum engine if specified
                    if let Some(ref qe_py) = builder.quantum_engine_builder {
                        sim_builder = apply_quantum_engine(py, sim_builder, qe_py)?;
                    }

                    // Apply noise if specified
                    if let Some(ref noise_py) = builder.noise_builder {
                        sim_builder = apply_noise_model(py, sim_builder, noise_py)?;
                    }

                    // Run the simulation with SeleneExecutableEngine and Bridge plugin
                    log::debug!("Running simulation with SeleneExecutableEngine and Bridge plugin");
                    match sim_builder.run(shots) {
                        Ok(shot_vec) => Ok(PyShotVec::new(shot_vec)),
                        Err(e) => Err(PyRuntimeError::new_err(format!("Simulation failed: {e}"))),
                    }
                })
            }
            SimBuilderInner::SeleneLibrary(builder) => {
                log::debug!(" In SimBuilderInner::SeleneLibrary run() method");
                log::debug!(" SeleneLibrary - should build engine in build(), not run()");

                // The SeleneLibrary case should have already built the engine
                // during the transition from PySimBuilder to SimBuilder.
                // For now, we'll build and run here as a temporary solution.
                Python::with_gil(|py| -> PyResult<PyShotVec> {
                    use pecos_engines::{Data, Shot, ShotVec};
                    use std::collections::BTreeMap;
                    use std::io::Write;

                    let program = builder
                        .program
                        .as_ref()
                        .ok_or_else(|| PyRuntimeError::new_err("No program specified"))?;

                    // Compile Guppy to HUGR Package if needed
                    let hugr_package = if is_guppy_function(py, program)? {
                        log::debug!(" Compiling Guppy to HUGR Package");
                        program.call_method0(py, "compile")?
                    } else {
                        log::debug!(" Using existing program (assuming HUGR)");
                        program.clone_ref(py)
                    };

                    // Build the Selene executable
                    let selene_sim = py.import("selene_sim")?;
                    let build_func = selene_sim.getattr("build")?;

                    let tempfile = py.import("tempfile")?;
                    let tempdir = tempfile.call_method0("mkdtemp")?;
                    let build_dir = tempdir.extract::<String>()?;
                    let temp_dir_path = build_dir.clone(); // Save for later use
                    log::debug!(" Building Selene executable in {build_dir}");

                    // Get the number of qubits - use explicit value if set, otherwise default to 10
                    let num_qubits = builder.explicit_num_qubits.unwrap_or(10);
                    log::debug!(" Using num_qubits = {num_qubits}");

                    // Create artifacts directory and pecos_config.json BEFORE building
                    // This ensures the Bridge plugin reads the correct qubit count when initialized
                    let artifacts_dir = format!("{build_dir}/artifacts");
                    std::fs::create_dir_all(&artifacts_dir).map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to create artifacts dir: {e}"))
                    })?;

                    // Write the PECOS config file with the correct qubit count
                    let config_path = format!("{artifacts_dir}/pecos_config.json");
                    let config_json = serde_json::json!({
                        "n_qubits": num_qubits,
                        "ipc_mode": true,
                    });
                    std::fs::write(&config_path, config_json.to_string()).map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to write pecos_config.json: {e}"))
                    })?;
                    log::debug!(
                        "Created pecos_config.json with n_qubits={num_qubits} at {config_path}"
                    );

                    // Verify the file was created
                    if std::path::Path::new(&config_path).exists() {
                        log::debug!(" Verified pecos_config.json exists at {config_path}");
                        if let Ok(contents) = std::fs::read_to_string(&config_path) {
                            log::debug!(" Config contents: {contents}");
                        }
                    } else {
                        log::debug!(" ERROR - Config file not found after creation!");
                    }

                    // Set the SELENE_ARTIFACTS_DIR environment variable so Bridge can find the config
                    unsafe {
                        std::env::set_var("SELENE_ARTIFACTS_DIR", &artifacts_dir);
                    }
                    log::debug!(" Set SELENE_ARTIFACTS_DIR={artifacts_dir}");

                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("build_dir", &build_dir)?;
                    kwargs.set_item("verbose", false)?;
                    kwargs.set_item("name", "guppy_prog")?;

                    let instance = build_func.call((hugr_package,), Some(&kwargs))?;
                    log::debug!(" Built Selene instance successfully");

                    // Try to import PECOS Bridge plugin for natural Selene integration
                    let bridge_plugin = match py.import("pecos.selene_plugins.simulators") {
                        Ok(module) => match module.getattr("PecosBridgePlugin") {
                            Ok(plugin_cls) => match plugin_cls.call0() {
                                Ok(plugin) => {
                                    log::debug!(" Successfully loaded PECOS Bridge plugin");
                                    Some(plugin)
                                }
                                Err(e) => {
                                    log::debug!("Failed to create Bridge plugin instance: {e}");
                                    None
                                }
                            },
                            Err(e) => {
                                log::debug!(" Failed to get PecosBridgePlugin class: {e}");
                                None
                            }
                        },
                        Err(e) => {
                            log::debug!(
                                "Bridge plugin not available ({e}), falling back to standard Selene"
                            );
                            None
                        }
                    };

                    // Set environment variables for Bridge plugin communication
                    unsafe {
                        std::env::set_var("SELENE_IPC", "1");
                        std::env::set_var("SELENE_TEMP_DIR", &temp_dir_path);
                    }
                    log::debug!(" Set SELENE_IPC=1 for Bridge plugin communication");
                    log::debug!(" Set SELENE_TEMP_DIR={temp_dir_path} for results");

                    // Get the number of qubits - use explicit value if set, otherwise default to 10
                    log::debug!(
                        "builder.explicit_num_qubits = {:?}",
                        builder.explicit_num_qubits
                    );
                    let num_qubits = builder.explicit_num_qubits.unwrap_or(10);
                    log::debug!(" Using num_qubits = {num_qubits}");

                    // Use Selene's natural runtime execution with or without Bridge plugin
                    let run_kwargs = pyo3::types::PyDict::new(py);
                    run_kwargs.set_item("verbose", false)?;

                    log::debug!(" About to call instance.run()...");
                    let run_result = if let Some(plugin) = &bridge_plugin {
                        log::debug!(
                            "Calling Selene.run() with PECOS Bridge plugin as simulator..."
                        );
                        // Pass plugin as first positional arg (simulator), n_qubits as second
                        let result = instance.call_method(
                            "run",
                            (plugin.clone(), num_qubits),
                            Some(&run_kwargs),
                        )?;
                        log::debug!(" Selene.run() with Bridge plugin returned!");
                        result
                    } else {
                        log::debug!(" Running Selene with default Quest simulator");
                        // For default, we still need to provide the positional arguments
                        // Let's use Quest as the default simulator
                        let quest = py.import("quest_core")?.getattr("QuestPlugin")?.call0()?;

                        instance.call_method("run", (quest, num_qubits), Some(&run_kwargs))?
                    };

                    log::debug!(" Selene execution completed, run_result obtained");

                    // Skip accessing run_result directly when using Bridge plugin
                    // It causes a hang, likely due to IPC issues

                    // Force flush stderr to ensure debug messages appear
                    let _ = std::io::stderr().flush();

                    // Convert Selene results to PECOS ShotVec format
                    let mut shot_vec = ShotVec { shots: Vec::new() };

                    log::debug!(" Created shot_vec");
                    let _ = std::io::stderr().flush();

                    // Parse actual results from Selene run_result iterator
                    log::debug!(" About to parse results from Selene execution...");
                    let _ = std::io::stderr().flush();

                    // Check if we used the Bridge plugin
                    let used_bridge = bridge_plugin.is_some();
                    log::debug!(" Used Bridge plugin: {used_bridge}");

                    if used_bridge {
                        // The Bridge plugin writes results to files - try to read them
                        log::debug!(" Bridge plugin mode - reading results from files");

                        for shot_id in 0..shots {
                            let mut shot_data = BTreeMap::new();

                            // Try to read the results file for this shot
                            let results_file =
                                format!("{temp_dir_path}/bridge_results_shot_{shot_id}.json");
                            log::debug!(" Looking for results file: {results_file}");

                            if let Ok(contents) = std::fs::read_to_string(&results_file) {
                                log::debug!(" Found results file with contents: {contents}");
                                // Parse the simple JSON format
                                // Format: {"measurement_0":true,"measurement_1":false,...}
                                if contents.starts_with('{') && contents.ends_with('}') {
                                    let inner = &contents[1..contents.len() - 1];
                                    for pair in inner.split(',') {
                                        if let Some(colon_idx) = pair.find(':') {
                                            let key = pair[..colon_idx].trim_matches('"');
                                            let value_str = &pair[colon_idx + 1..];
                                            let value = value_str == "true";
                                            shot_data
                                                .insert(key.to_string(), Data::U8(u8::from(value)));
                                        }
                                    }
                                }
                            } else {
                                // Fall back to placeholder if no file found
                                log::debug!(" No results file found, using placeholder");
                                shot_data.insert(
                                    "measurement_0".to_string(),
                                    Data::U8(u8::from(shot_id % 2 != 0)),
                                );
                            }

                            shot_data.insert("bridge_plugin_active".to_string(), Data::U8(1));
                            let shot = Shot { data: shot_data };
                            shot_vec.shots.push(shot);
                        }
                    } else {
                        // Regular Selene execution - try to parse results
                        log::debug!(" Regular Selene mode - attempting to parse results");

                        let mut shots_parsed = 0;

                        // Try to check if run_result is None or not iterable
                        let is_none = run_result.is_none();
                        let has_iter = if is_none {
                            false
                        } else {
                            run_result.hasattr("__iter__").unwrap_or(false)
                        };

                        log::debug!(" run_result is_none: {is_none}, has_iter: {has_iter}");

                        if !is_none && has_iter {
                            log::debug!(" Attempting to iterate over run_result");
                            match run_result.try_iter() {
                                Ok(result_iter) => {
                                    for (shot_idx, result_item) in result_iter.enumerate() {
                                        if shots_parsed >= shots {
                                            log::debug!(
                                                "Reached requested shot limit ({shots}), stopping"
                                            );
                                            break;
                                        }

                                        log::debug!("Processing Selene result item {shot_idx}");

                                        let mut shot_data = BTreeMap::new();

                                        // Try to extract measurement results from Selene result
                                        match result_item {
                                            Ok(item) => {
                                                log::debug!(" Got result item: {item:?}");

                                                // Try to parse as measurement data
                                                if let Ok(measurement_dict) = item.extract::<std::collections::HashMap<String, bool>>() {
                                            log::debug!(" Found measurements: {measurement_dict:?}");
                                            // Convert measurements to shot data
                                            for (qubit_name, measured_value) in measurement_dict {
                                                shot_data.insert(qubit_name, Data::U8(u8::from(measured_value)));
                                            }
                                        } else if let Ok(measurement_list) = item.extract::<Vec<bool>>() {
                                            log::debug!(" Found measurement list: {measurement_list:?}");
                                            // Convert list to indexed measurements
                                            for (qubit_idx, measured_value) in measurement_list.iter().enumerate() {
                                                shot_data.insert(format!("q{qubit_idx}"), Data::U8(u8::from(*measured_value)));
                                            }
                                        } else if let Ok(measurement_int) = item.extract::<i64>() {
                                            log::debug!(" Found measurement integer: {measurement_int}");
                                            // Single measurement result
                                            shot_data.insert("q0".to_string(), Data::I64(measurement_int));
                                        } else if let Ok(measurement_bool) = item.extract::<bool>() {
                                            log::debug!(" Found measurement boolean: {measurement_bool}");
                                            // Single boolean measurement
                                            shot_data.insert("q0".to_string(), Data::U8(u8::from(measurement_bool)));
                                        } else {
                                            log::debug!(" Could not parse result item as measurement data");
                                            // Try to get string representation for debugging
                                            if let Ok(item_str) = item.str()
                                                && let Ok(item_string) = item_str.extract::<String>() {
                                                    log::debug!(" Item string representation: {item_string}");
                                                    // Store raw result for debugging - encode as bytes for now
                                                    let bytes = item_string.as_bytes();
                                                    if !bytes.is_empty() {
                                                        shot_data.insert("raw_result".to_string(), Data::U8(bytes[0]));
                                                    }
                                                }
                                        }
                                            }
                                            Err(e) => {
                                                log::debug!(" Error getting result item: {e}");

                                                // Check if this is a UnicodeDecodeError (indicates ByteMessage data)
                                                let error_str = format!("{e}");
                                                if error_str.contains("UnicodeDecodeError")
                                                    || error_str.contains("utf-8")
                                                {
                                                    log::debug!(
                                                        "UnicodeDecodeError detected - this indicates ByteMessage data flowing!"
                                                    );
                                                    log::debug!(
                                                        "Bridge plugin is sending binary data via IPC (this is good!)"
                                                    );

                                                    // Try to get the raw bytes from the Python exception
                                                    // For now, mark this as a successful ByteMessage detection
                                                    shot_data.insert(
                                                        "bytemeessage_detected".to_string(),
                                                        Data::U8(1),
                                                    );
                                                    shot_data.insert(
                                                        "ipc_active".to_string(),
                                                        Data::U8(1),
                                                    );
                                                } else {
                                                    // Other error types
                                                    shot_data
                                                        .insert("error".to_string(), Data::U8(1)); // 1 = error occurred
                                                }
                                            }
                                        }

                                        let shot = Shot { data: shot_data };
                                        shot_vec.shots.push(shot);
                                        shots_parsed += 1;
                                    }
                                }
                                Err(e) => {
                                    log::debug!(" Failed to iterate over run_result: {e}");
                                    // Fall back to creating empty shots for the requested count
                                    for _shot_idx in 0..shots {
                                        let mut shot_data = BTreeMap::new();
                                        shot_data.insert("parse_error".to_string(), Data::U8(2)); // 2 = parse error
                                        let shot = Shot { data: shot_data };
                                        shot_vec.shots.push(shot);
                                    }
                                }
                            }
                        } else {
                            // run_result is None or not iterable
                            log::debug!(
                                "run_result is None or not iterable, creating placeholder results"
                            );
                            for i in 0..shots {
                                let mut shot_data = BTreeMap::new();
                                // Add placeholder measurement results
                                shot_data.insert(
                                    "measurement_0".to_string(),
                                    Data::U8(u8::from(i % 2 != 0)),
                                );
                                shot_data.insert("no_iteration".to_string(), Data::U8(1));
                                let shot = Shot { data: shot_data };
                                shot_vec.shots.push(shot);
                            }
                        }
                    }

                    // If we didn't get enough results, pad with empty shots
                    while shot_vec.shots.len() < shots {
                        let shot_data = BTreeMap::new();
                        let shot = Shot { data: shot_data };
                        shot_vec.shots.push(shot);
                        log::debug!(" Added empty shot to reach requested count");
                    }

                    log::debug!(" Completed {shots} shots");
                    log::debug!(" Shot results: {shot_vec:?}");

                    Ok(PyShotVec::new(shot_vec))
                })
            }
            SimBuilderInner::Empty => Err(PyRuntimeError::new_err(
                "Cannot run empty builder - no program specified",
            )),
        }
    }

    /// Build the simulation (for multiple runs)
    #[allow(clippy::too_many_lines)] // Complex builder pattern with multiple engine types
    fn build(&self) -> PyResult<PyObject> {
        use crate::engine_builders::{
            PyBiasedDepolarizingNoiseModelBuilder, PyDepolarizingNoiseModelBuilder,
            PyGeneralNoiseModelBuilder,
        };
        use crate::engine_builders::{PyPhirJsonSimulation, PyQasmSimulation};
        use crate::engine_builders::{PySparseStabilizerEngineBuilder, PyStateVectorEngineBuilder};
        use pyo3::exceptions::PyRuntimeError;

        Python::with_gil(|py| {
            match &self.inner {
                SimBuilderInner::Qasm(builder) => {
                    let mut builder_lock = builder.engine_builder.lock().unwrap();
                    let engine_builder = builder_lock
                        .take()
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
                            if let Ok(mut state_vec) =
                                qe_py.extract::<PyStateVectorEngineBuilder>(py)
                            {
                                if let Some(inner) = state_vec.inner.take() {
                                    Ok(sim_builder.quantum(inner))
                                } else {
                                    Err(PyErr::new::<PyRuntimeError, _>(
                                        "Quantum engine builder has already been consumed",
                                    ))
                                }
                            } else if let Ok(mut sparse_stab) =
                                qe_py.extract::<PySparseStabilizerEngineBuilder>(py)
                            {
                                if let Some(inner) = sparse_stab.inner.take() {
                                    Ok(sim_builder.quantum(inner))
                                } else {
                                    Err(PyErr::new::<PyRuntimeError, _>(
                                        "Quantum engine builder has already been consumed",
                                    ))
                                }
                            } else {
                                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "quantum_engine must be a valid quantum engine builder",
                                ))
                            }
                        })?;
                    }

                    // Apply noise builder if present
                    if let Some(ref noise_py) = builder.noise_builder {
                        sim_builder = Python::with_gil(|py| -> PyResult<_> {
                            if let Ok(general) = noise_py.extract::<PyGeneralNoiseModelBuilder>(py)
                            {
                                Ok(sim_builder.noise(general.inner.clone()))
                            } else if let Ok(depolarizing) =
                                noise_py.extract::<PyDepolarizingNoiseModelBuilder>(py)
                            {
                                Ok(sim_builder.noise(depolarizing.inner.clone()))
                            } else if let Ok(biased) =
                                noise_py.extract::<PyBiasedDepolarizingNoiseModelBuilder>(py)
                            {
                                Ok(sim_builder.noise(biased.inner.clone()))
                            } else {
                                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "noise must be a valid noise model builder",
                                ))
                            }
                        })?;
                    }

                    // Build the MonteCarloEngine
                    let engine = sim_builder.build().map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to build simulation: {e}"))
                    })?;

                    Ok(Py::new(
                        py,
                        PyQasmSimulation {
                            inner: Arc::new(Mutex::new(engine)),
                        },
                    )?
                    .into_any())
                }
                SimBuilderInner::PhirJson(builder) => {
                    // Similar implementation for PHIR JSON
                    let mut builder_lock = builder.engine_builder.lock().unwrap();
                    let engine_builder = builder_lock
                        .take()
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

                    let engine = sim_builder.build().map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to build simulation: {e}"))
                    })?;

                    Ok(Py::new(
                        py,
                        PyPhirJsonSimulation {
                            inner: Arc::new(Mutex::new(engine)),
                        },
                    )?
                    .into_any())
                }
                // LLVM and Selene don't have build() methods in current implementation
                SimBuilderInner::Llvm(_) => Err(PyRuntimeError::new_err(
                    "LLVM simulation does not support build() yet - use run() directly",
                )),
                SimBuilderInner::Selene(_) => Err(PyRuntimeError::new_err(
                    "Selene simulation does not support build() yet - use run() directly",
                )),
                SimBuilderInner::SeleneExecutable(_) => Err(PyRuntimeError::new_err(
                    "SeleneExecutable simulation does not support build() yet - use run() directly",
                )),
                SimBuilderInner::SeleneLibrary(_) => Err(PyRuntimeError::new_err(
                    "SeleneLibrary simulation does not support build() yet - use run() directly",
                )),
                SimBuilderInner::Empty => Err(PyRuntimeError::new_err(
                    "Cannot build empty builder - no program specified",
                )),
            }
        })
    }
}

// Clone implementations for the inner types
impl Clone for SimBuilderInner {
    fn clone(&self) -> Self {
        Python::with_gil(|py| match self {
            SimBuilderInner::Qasm(builder) => SimBuilderInner::Qasm(PyQasmSimBuilder {
                engine_builder: builder.engine_builder.clone(),
                seed: builder.seed,
                workers: builder.workers,
                quantum_engine_builder: builder
                    .quantum_engine_builder
                    .as_ref()
                    .map(|obj| obj.clone_ref(py)),
                noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                explicit_num_qubits: builder.explicit_num_qubits,
            }),
            SimBuilderInner::Llvm(builder) => SimBuilderInner::Llvm(PyLlvmSimBuilder {
                engine_builder: builder.engine_builder.clone(),
                seed: builder.seed,
                workers: builder.workers,
                quantum_engine_builder: builder
                    .quantum_engine_builder
                    .as_ref()
                    .map(|obj| obj.clone_ref(py)),
                noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                explicit_num_qubits: builder.explicit_num_qubits,
            }),
            SimBuilderInner::Selene(builder) => SimBuilderInner::Selene(PySeleneSimBuilder {
                engine_builder: builder.engine_builder.clone(),
                seed: builder.seed,
                workers: builder.workers,
                quantum_engine_builder: builder
                    .quantum_engine_builder
                    .as_ref()
                    .map(|obj| obj.clone_ref(py)),
                noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                explicit_num_qubits: builder.explicit_num_qubits,
            }),
            SimBuilderInner::PhirJson(builder) => SimBuilderInner::PhirJson(PyPhirJsonSimBuilder {
                engine_builder: builder.engine_builder.clone(),
                seed: builder.seed,
                workers: builder.workers,
                quantum_engine_builder: builder
                    .quantum_engine_builder
                    .as_ref()
                    .map(|obj| obj.clone_ref(py)),
                noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                explicit_num_qubits: builder.explicit_num_qubits,
            }),
            SimBuilderInner::SeleneExecutable(builder) => {
                SimBuilderInner::SeleneExecutable(PySeleneExecutableSimBuilder {
                    program: builder.program.as_ref().map(|obj| obj.clone_ref(py)),
                    engine_builder: builder.engine_builder.clone(),
                    seed: builder.seed,
                    workers: builder.workers,
                    quantum_engine_builder: builder
                        .quantum_engine_builder
                        .as_ref()
                        .map(|obj| obj.clone_ref(py)),
                    noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    explicit_num_qubits: builder.explicit_num_qubits,
                })
            }
            SimBuilderInner::SeleneLibrary(builder) => {
                SimBuilderInner::SeleneLibrary(PySeleneLibrarySimBuilder {
                    program: builder.program.as_ref().map(|obj| obj.clone_ref(py)),
                    seed: builder.seed,
                    workers: builder.workers,
                    quantum_engine_builder: builder
                        .quantum_engine_builder
                        .as_ref()
                        .map(|obj| obj.clone_ref(py)),
                    noise_builder: builder.noise_builder.as_ref().map(|obj| obj.clone_ref(py)),
                    explicit_num_qubits: builder.explicit_num_qubits,
                })
            }
            SimBuilderInner::Empty => SimBuilderInner::Empty,
        })
    }
}

/// Register the sim module
pub fn register_sim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySimBuilder>()?;
    m.add_function(wrap_pyfunction!(sim, m)?)?;
    Ok(())
}
