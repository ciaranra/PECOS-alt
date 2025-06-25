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

//! Python bindings for QIR execution

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::{PyDict, PyList};
use pecos_qir::{setup_qir_engine};
use pecos_qir::error_handling::{init_qir_context, clear_qir_context, get_qir_diagnostic_report};
use pecos_qir::panic_handler::{init_qir_panic_handler, with_qir_error_context};
use pecos_qir::qir_utils::validate_qir_format;
use pecos_engines::NoiseModel;
use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_engines::shot_results;
use pecos_core::rng::RngManageable;
use std::path::PathBuf;
use std::fs;

/// Python wrapper for QIR execution
#[pyclass(name = "QirEngine")]
pub struct PyQirEngine {
    qir_path: PathBuf,
}

#[pymethods]
impl PyQirEngine {
    /// Create a new QIR engine from a QIR file path
    #[new]
    pub fn new(qir_path: &str) -> PyResult<Self> {
        let path = PathBuf::from(qir_path);
        if !path.exists() {
            return Err(PyRuntimeError::new_err(format!(
                "QIR file not found: {}",
                qir_path
            )));
        }
        Ok(Self { qir_path: path })
    }

    /// Execute the QIR program with the given parameters
    pub fn execute(
        &self,
        py: Python<'_>,
        shots: usize,
        seed: Option<u64>,
        noise_probability: Option<f64>,
        workers: Option<usize>,
    ) -> PyResult<PyObject> {
        // Execute QIR with proper serialization (LLVM best practice)
        let results = execute_qir_safe(&self.qir_path, shots, seed, noise_probability, workers)
            .map_err(|e| PyRuntimeError::new_err(format!("QIR execution failed: {:?}", e)))?;

        // Convert results to Python format
        convert_results_to_python(py, results, shots)
    }
}

/// Convert shot results to Python format
fn convert_results_to_python(
    py: Python<'_>,
    results: shot_results::ShotVec,
    shots: usize,
) -> PyResult<PyObject> {
    let result_list = PyList::empty(py);
    for shot in results.shots {
        // Handle different result formats
        if shot.data.len() == 1 {
            // Single register - return as single value
            if let Some((_, data)) = shot.data.iter().next() {
                match data {
                    shot_results::Data::U32(v) => {
                        result_list.append(*v != 0)?;
                    }
                    shot_results::Data::I64(v) => {
                        result_list.append(*v != 0)?;
                    }
                    _ => {}
                }
            }
        } else if shot.data.len() > 1 {
            // Multiple registers - return as tuple
            let tuple_vals = PyList::empty(py);
            for (_, data) in &shot.data {
                match data {
                    shot_results::Data::U32(v) => {
                        tuple_vals.append(*v != 0)?;
                    }
                    shot_results::Data::I64(v) => {
                        tuple_vals.append(*v != 0)?;
                    }
                    _ => {}
                }
            }
            result_list.append(tuple_vals.to_tuple())?;
        }
    }

    // Return a dictionary with results and metadata
    let result_dict = PyDict::new(py);
    result_dict.set_item("results", result_list)?;
    result_dict.set_item("shots", shots)?;
    result_dict.set_item("execution_successful", true)?;
    
    Ok(result_dict.into())
}


/// Simplified QIR execution
fn execute_qir_safe(
    qir_path: &std::path::Path,
    shots: usize,
    seed: Option<u64>,
    noise_probability: Option<f64>,
    workers: Option<usize>,
) -> Result<shot_results::ShotVec, pecos_core::errors::PecosError> {
    use crate::qir_execution_guard::QirExecutionGuard;
    
    // Create execution guard to prevent cleanup issues
    let _guard = QirExecutionGuard::new()
        .map_err(|e| pecos_core::errors::PecosError::Input(e.to_string()))?;
    
    // Simple reset - no complex context system
    unsafe {
        pecos_qir::runtime::qir_runtime_reset();
    }
    
    // Set up QIR engine
    let classical_engine = setup_qir_engine(qir_path, None)?;
    
    // Create noise model
    let noise_model: Box<dyn NoiseModel> = if let Some(prob) = noise_probability {
        let mut model = DepolarizingNoiseModel::new_uniform(prob);
        if let Some(s) = seed {
            model.set_seed(s)?;
        }
        Box::new(model)
    } else {
        Box::new(pecos_engines::noise::PassThroughNoiseModel)
    };
    
    // Execute simulation with validated parameters
    let mut params = crate::safe_calls::SimParams::new(classical_engine, shots);
    
    if let Some(s) = seed {
        params = params.with_seed(s);
    }
    if let Some(w) = workers {
        params = params.with_workers(w);
    }
    params = params.with_noise_model(noise_model);
    
    let results = params.run()?;
    
    // Force another reset after execution
    unsafe {
        pecos_qir::runtime::qir_runtime_reset();
    }
    
    // Clear any stored engines
    #[cfg(feature = "hugr-llvm-pipeline")]
    {
        if let Ok(mut engines) = pecos_qir::python_api::get_stored_engine_mut(0) {
            engines.clear();
        }
    }
    
    // Clean up runtime registry
    pecos_qir::runtime_registry::cleanup_all_runtimes();
    
    // Give the runtime a moment to clean up thread-local storage
    // This prevents segfaults when running in pytest environments
    std::thread::sleep(std::time::Duration::from_millis(1));
    
    Ok(results)
}

/// Direct function to execute QIR file
#[pyfunction]
#[pyo3(name = "execute_qir")]
pub fn py_execute_qir(
    py: Python<'_>,
    qir_path: &str,
    shots: usize,
    seed: Option<u64>,
    noise_probability: Option<f64>,
    workers: Option<usize>,
    llvm_convention: Option<&str>,
) -> PyResult<PyObject> {
    // Initialize enhanced error handling
    init_qir_panic_handler();
    
    // Validate QIR file path
    let path = std::path::PathBuf::from(qir_path);
    if !path.exists() {
        return Err(PyRuntimeError::new_err(format!(
            "QIR file not found: {}",
            qir_path
        )));
    }
    
    // Validate QIR format before execution (skip for HUGR convention)
    let convention = llvm_convention.unwrap_or("qir");
    if convention != "hugr" {
        match fs::read_to_string(&path) {
            Ok(qir_content) => {
                if let Err(validation_error) = validate_qir_format(&qir_content) {
                    return Err(PyRuntimeError::new_err(format!(
                        "QIR format validation failed: {}",
                        validation_error
                    )));
                }
            }
            Err(e) => {
                return Err(PyRuntimeError::new_err(format!(
                    "Failed to read QIR file: {}",
                    e
                )));
            }
        }
    }
    
    // Check for pytest environment and warn about potential segfaults
    if std::env::var("PYTEST_CURRENT_TEST").is_ok() {
        // We're running in pytest - execution works but may segfault during cleanup
        eprintln!("Warning: QIR execution in pytest may segfault during cleanup (output will be produced first)");
        
        // Force clear any lingering runtime state from previous tests
        unsafe {
            pecos_qir::runtime::qir_runtime_reset();
        }
        // Clear any interactive callbacks
        pecos_qir::runtime::core_runtime::clear_interactive_callback();
    }
    
    // Initialize QIR execution context
    init_qir_context(Some(path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()));
    
    // Execute using enhanced error context
    let results = with_qir_error_context("execute_qir", || {
        execute_qir_safe(&path, shots, seed, noise_probability, workers)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    })
    .map_err(|e| {
        let diagnostic = get_qir_diagnostic_report();
        clear_qir_context();
        
        let detailed_error = format!(
            "QIR execution failed: {}\n\nDiagnostic Information:\n{}",
            e, diagnostic
        );
        PyRuntimeError::new_err(detailed_error)
    })?;
    
    // Clear context after successful execution
    clear_qir_context();
    
    // Convert results to Python format
    convert_results_to_python(py, results, shots)
}

/// Validate QIR format and get detailed diagnostics
#[pyfunction]
#[pyo3(name = "validate_qir_format_detailed")]
pub fn py_validate_qir_format(qir_path: &str) -> PyResult<PyObject> {
    use pyo3::types::PyDict;
    use pecos_qir::error_handling::validate_qir_for_runtime_issues;
    
    let path = std::path::PathBuf::from(qir_path);
    if !path.exists() {
        return Err(PyRuntimeError::new_err(format!(
            "QIR file not found: {}",
            qir_path
        )));
    }
    
    let qir_content = fs::read_to_string(&path)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to read QIR file: {}", e)))?;
    
    Python::with_gil(|py| {
        let result = PyDict::new(py);
        
        // Basic format validation
        match validate_qir_format(&qir_content) {
            Ok(()) => {
                result.set_item("format_valid", true)?;
                result.set_item("format_errors", Vec::<String>::new())?;
            }
            Err(e) => {
                result.set_item("format_valid", false)?;
                result.set_item("format_errors", vec![e.to_string()])?;
            }
        }
        
        // Runtime issue detection
        match validate_qir_for_runtime_issues(&qir_content) {
            Ok(warnings) => {
                result.set_item("runtime_warnings", warnings)?;
            }
            Err(e) => {
                result.set_item("runtime_warnings", vec![format!("Validation failed: {}", e)])?;
            }
        }
        
        // QIR statistics
        let stats = PyDict::new(py);
        stats.set_item("total_lines", qir_content.lines().count())?;
        stats.set_item("quantum_operations", qir_content.matches("__quantum__qis__").count())?;
        stats.set_item("has_entry_point", qir_content.contains("EntryPoint"))?;
        stats.set_item("has_opaque_types", qir_content.contains("type opaque"))?;
        stats.set_item("uses_integer_qubits", qir_content.contains("__quantum__qis__h__body(i64"))?;
        stats.set_item("uses_pointer_qubits", qir_content.contains("__quantum__qis__h__body(i8*") || qir_content.contains("__quantum__qis__h__body(%Qubit*"))?;
        result.set_item("statistics", stats)?;
        
        Ok(result.into())
    })
}


/// Get QIR execution diagnostic report
#[pyfunction]
#[pyo3(name = "get_qir_diagnostic_report")]
pub fn py_get_qir_diagnostic_report() -> PyResult<String> {
    Ok(get_qir_diagnostic_report())
}

/// Reset QIR runtime state (simplified)
#[pyfunction]
#[pyo3(name = "reset_qir_runtime")]
pub fn py_reset_qir_runtime() -> PyResult<()> {
    use std::thread;
    use std::time::Duration;
    
    // Clear all stored engines first
    #[cfg(feature = "hugr-llvm-pipeline")]
    {
        if let Ok(mut engines) = pecos_qir::python_api::get_stored_engine_mut(0) {
            engines.clear();
        }
    }
    
    // Simple reset - no aggressive cleanup
    unsafe {
        pecos_qir::runtime::qir_runtime_reset();
    }
    
    // Clean up all runtime registry states
    pecos_qir::runtime_registry::cleanup_all_runtimes();
    
    // Give the runtime a moment to clean up
    // This helps prevent segfaults in pytest environments
    thread::sleep(Duration::from_millis(10));
    
    Ok(())
}

/// Register QIR Python module  
pub fn register_qir_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQirEngine>()?;
    m.add_function(wrap_pyfunction!(py_execute_qir, m)?)?;
    m.add_function(wrap_pyfunction!(py_validate_qir_format, m)?)?;
    m.add_function(wrap_pyfunction!(py_get_qir_diagnostic_report, m)?)?;
    m.add_function(wrap_pyfunction!(py_reset_qir_runtime, m)?)?;
    
    // Add cleanup handlers to prevent abort on exit
    m.add_function(wrap_pyfunction!(crate::qir_execution_guard::_mark_qir_shutting_down, m)?)?;
    m.add_function(wrap_pyfunction!(crate::qir_execution_guard::_wait_for_qir_completion, m)?)?;
    
    // Register cleanup handler on module load
    crate::qir_execution_guard::register_cleanup_handler();
    
    Ok(())
}