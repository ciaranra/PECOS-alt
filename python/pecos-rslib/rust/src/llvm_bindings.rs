// Copyright 2025 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! Python bindings for LLVM execution


use pecos::setup_llvm_engine;
use pecos_core::rng::RngManageable;
use pecos_engines::NoiseModel;
use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_engines::shot_results;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::fs;
use std::path::PathBuf;


/// Python wrapper for LLVM execution
#[pyclass(name = "QisEngine")]
pub struct PyQisEngine {
    llvm_path: PathBuf,
}

#[pymethods]
impl PyQisEngine {
    /// Create a new LLVM engine from an LLVM file path
    #[new]
    pub fn new(llvm_path: &str) -> PyResult<Self> {
        let path = PathBuf::from(llvm_path);
        if !path.exists() {
            return Err(PyRuntimeError::new_err(format!(
                "LLVM file not found: {llvm_path}"
            )));
        }
        Ok(Self { llvm_path: path })
    }

    /// Execute the LLVM program with the given parameters
    pub fn execute(
        &self,
        py: Python<'_>,
        shots: usize,
        seed: Option<u64>,
        noise_probability: Option<f64>,
        workers: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        // Execute LLVM with proper serialization (LLVM best practice)
        let results =
            execute_llvm_safe(&self.llvm_path, shots, seed, noise_probability, workers, None)
                .map_err(|e| PyRuntimeError::new_err(format!("LLVM execution failed: {e:?}")))?;

        // Convert results to Python format
        convert_results_to_python(py, results, shots)
    }
}

/// Convert shot results to Python format
fn convert_results_to_python(
    py: Python<'_>,
    results: shot_results::ShotVec,
    shots: usize,
) -> PyResult<Py<PyAny>> {
    let result_list = PyList::empty(py);
    for shot in results.shots {
        // Handle different result formats
        match shot.data.len() {
            1 => {
                // Single register - return as single value
                if let Some((_, data)) = shot.data.iter().next() {
                    match data {
                        shot_results::Data::U32(v) => {
                            result_list.append(*v)?;
                        }
                        shot_results::Data::I64(v) => {
                            result_list.append(*v)?;
                        }
                        _ => {}
                    }
                }
            }
            0 => {
                // No data - skip
            }
            _ => {
                // Multiple registers - return as tuple
                let tuple_vals = PyList::empty(py);
                for data in shot.data.values() {
                    match data {
                        shot_results::Data::U32(v) => {
                            tuple_vals.append(*v)?;
                        }
                        shot_results::Data::I64(v) => {
                            tuple_vals.append(*v)?;
                        }
                        _ => {}
                    }
                }
                result_list.append(tuple_vals.to_tuple())?;
            }
        }
    }

    // Return a dictionary with results and metadata
    let result_dict = PyDict::new(py);
    result_dict.set_item("results", result_list)?;
    result_dict.set_item("shots", shots)?;
    result_dict.set_item("execution_successful", true)?;

    Ok(result_dict.into())
}

/// Simplified LLVM execution
fn execute_llvm_safe(
    llvm_path: &std::path::Path,
    shots: usize,
    seed: Option<u64>,
    noise_probability: Option<f64>,
    workers: Option<usize>,
    max_qubits: Option<usize>,
) -> Result<shot_results::ShotVec, pecos_core::errors::PecosError> {
    use crate::llvm_execution_guard::LlvmExecutionGuard;

    // Create execution guard to prevent cleanup issues
    let _guard = LlvmExecutionGuard::new()
        .map_err(|e| pecos_core::errors::PecosError::Input(e.to_string()))?;

    // Simple reset - no complex context system
    unsafe {
        pecos_qis_runtime::runtime::llvm_runtime_reset();
    }

    // Set up LLVM engine with max_qubits if specified
    let classical_engine = if max_qubits.is_some() {
        pecos::setup_llvm_engine_with_config(llvm_path, None, max_qubits)?
    } else {
        setup_llvm_engine(llvm_path, None)?
    };

    // Create noise model
    let noise_model: Box<dyn NoiseModel> = if let Some(prob) = noise_probability {
        let mut model = DepolarizingNoiseModel::new_uniform(prob);
        if let Some(s) = seed {
            model.set_seed(s)?;
        }
        Box::new(model)
    } else {
        Box::new(pecos_engines::noise::PassThroughNoiseModel::new())
    };

    // Execute simulation with MonteCarloEngine directly to support max_qubits
    let workers = workers.unwrap_or(1);

    // Use MonteCarloEngine directly to have control over max_qubits
    let results = if let Some(max_q) = max_qubits {
        // When max_qubits is specified, use the new method
        pecos_engines::monte_carlo::MonteCarloEngine::run_with_noise_model_and_max_qubits(
            classical_engine,
            noise_model,
            max_q,
            shots,
            workers,
            seed,
        )?
    } else {
        // When max_qubits is not specified, use a reasonable default
        // For programs with loops, we need extra headroom
        let static_qubits = classical_engine.num_qubits();
        // Use 3x the static count or 10, whichever is larger, to handle dynamic allocation
        let default_max_qubits = std::cmp::max(static_qubits * 3, 10);

        pecos_engines::monte_carlo::MonteCarloEngine::run_with_noise_model_and_max_qubits(
            classical_engine,
            noise_model,
            default_max_qubits,
            shots,
            workers,
            seed,
        )?
    };

    // Force another reset after execution
    unsafe {
        pecos_qis_runtime::runtime::llvm_runtime_reset();
    }

    // Clear any stored engines from HUGR bindings
    #[cfg(feature = "hugr-llvm-pipeline")]
    {
        use crate::hugr_bindings::PYTHON_LLVM_ENGINES;
        PYTHON_LLVM_ENGINES.lock().unwrap().clear();
    }

    // Clean up runtime registry
    pecos_qis_runtime::runtime::registry::cleanup_all_runtimes();

    // Give the runtime a moment to clean up thread-local storage
    // This prevents segfaults when running in pytest environments
    std::thread::sleep(std::time::Duration::from_millis(1));

    Ok(results)
}

/// Direct function to execute LLVM file
#[pyfunction]
#[pyo3(name = "execute_llvm")]
#[pyo3(signature = (llvm_path, shots, seed, noise_probability, workers, max_qubits=None))]
pub fn py_execute_llvm(
    py: Python<'_>,
    llvm_path: &str,
    shots: usize,
    seed: Option<u64>,
    noise_probability: Option<f64>,
    workers: Option<usize>,
    max_qubits: Option<usize>,
) -> PyResult<Py<PyAny>> {
    // Enhanced error handling removed - not needed for simplification

    // Validate LLVM file path
    let path = std::path::PathBuf::from(llvm_path);
    if !path.exists() {
        return Err(PyRuntimeError::new_err(format!(
            "LLVM file not found: {llvm_path}"
        )));
    }

    // Check for pytest environment and warn about potential segfaults
    if std::env::var("PYTEST_CURRENT_TEST").is_ok() {
        // We're running in pytest - execution works but may segfault during cleanup
        log::warn!(
            "Warning: LLVM execution in pytest may segfault during cleanup (output will be produced first)"
        );

        // Force clear any lingering runtime state from previous tests
        unsafe {
            pecos_qis_runtime::runtime::llvm_runtime_reset();
        }
        // Clear any interactive callbacks
        pecos_qis_runtime::runtime::core_runtime::clear_interactive_callback();
    }

    // LLVM execution context initialization removed (was stub)

    // Execute LLVM directly without error context wrapper
    let results = execute_llvm_safe(&path, shots, seed, noise_probability, workers, max_qubits)
        .map_err(|e| PyRuntimeError::new_err(format!("LLVM execution failed: {e}")))?;

    // Convert results to Python format
    convert_results_to_python(py, results, shots)
}

/// Validate LLVM format and get detailed diagnostics
#[pyfunction]
#[pyo3(name = "validate_llvm_format_detailed")]
pub fn py_validate_llvm_format(llvm_path: &str) -> PyResult<Py<PyAny>> {
    use pyo3::types::PyDict;

    let path = std::path::PathBuf::from(llvm_path);
    if !path.exists() {
        return Err(PyRuntimeError::new_err(format!(
            "LLVM file not found: {llvm_path}"
        )));
    }

    let llvm_content = fs::read_to_string(&path)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to read LLVM file: {e}")))?;

    Python::attach(|py| {
        let result = PyDict::new(py);

        // Basic format validation
        if llvm_content.contains("@__quantum__") {
            result.set_item("format_valid", true)?;
            result.set_item("format_errors", Vec::<String>::new())?;
        } else {
            result.set_item("format_valid", false)?;
            result.set_item(
                "format_errors",
                vec!["No quantum operations found".to_string()],
            )?;
        }

        // Runtime issue detection (simplified - no actual validation needed)
        result.set_item("runtime_warnings", Vec::<String>::new())?;

        // LLVM statistics
        let stats = PyDict::new(py);
        stats.set_item("total_lines", llvm_content.lines().count())?;
        stats.set_item(
            "quantum_operations",
            llvm_content.matches("__quantum__qis__").count(),
        )?;
        stats.set_item("has_entry_point", llvm_content.contains("EntryPoint"))?;
        stats.set_item("has_opaque_types", llvm_content.contains("type opaque"))?;
        stats.set_item(
            "uses_integer_qubits",
            llvm_content.contains("__quantum__qis__h__body(i64"),
        )?;
        stats.set_item(
            "uses_pointer_qubits",
            llvm_content.contains("__quantum__qis__h__body(i8*")
                || llvm_content.contains("__quantum__qis__h__body(%Qubit*"),
        )?;
        result.set_item("statistics", stats)?;

        Ok(result.into())
    })
}

/// Get LLVM execution diagnostic report
///
/// Note: This function is deprecated and always returns an empty string.
/// It is kept for backward compatibility only.
#[pyfunction]
#[pyo3(name = "get_llvm_diagnostic_report")]
pub fn py_get_llvm_diagnostic_report() -> String {
    String::new()
}

/// Reset LLVM runtime state (simplified)
#[pyfunction]
#[pyo3(name = "reset_llvm_runtime")]
pub fn py_reset_llvm_runtime() {
    use std::thread;
    use std::time::Duration;

    // Clear all stored engines first
    #[cfg(feature = "hugr-llvm-pipeline")]
    {
        use crate::hugr_bindings::PYTHON_LLVM_ENGINES;
        PYTHON_LLVM_ENGINES.lock().unwrap().clear();
    }

    // Simple reset - no aggressive cleanup
    unsafe {
        pecos_qis_runtime::runtime::llvm_runtime_reset();
    }

    // Clean up all runtime registry states
    pecos_qis_runtime::runtime::registry::cleanup_all_runtimes();

    // Give the runtime a moment to clean up
    // This helps prevent segfaults in pytest environments
    thread::sleep(Duration::from_millis(10));
}

/// Register LLVM Python module
pub fn register_llvm_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQisEngine>()?;
    m.add_function(wrap_pyfunction!(py_execute_llvm, m)?)?;
    m.add_function(wrap_pyfunction!(py_validate_llvm_format, m)?)?;
    m.add_function(wrap_pyfunction!(py_get_llvm_diagnostic_report, m)?)?;
    m.add_function(wrap_pyfunction!(py_reset_llvm_runtime, m)?)?;

    // Add cleanup handlers to prevent abort on exit
    m.add_function(wrap_pyfunction!(
        crate::llvm_execution_guard::_mark_llvm_shutting_down,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        crate::llvm_execution_guard::_wait_for_llvm_completion,
        m
    )?)?;

    // Register cleanup handler on module load
    crate::llvm_execution_guard::register_cleanup_handler();

    Ok(())
}
