/*!
Python API for PMIR compilation pipeline

This module provides Python-accessible functions for the PMIR pipeline.
*/

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::{PmirConfig, compile_hugr_via_pmir};

/// Python-accessible PMIR configuration
#[pyclass]
#[derive(Clone)]
pub struct PyPmirConfig {
    #[pyo3(get, set)]
    pub debug_output: bool,
    #[pyo3(get, set)]
    pub optimization_level: u8,
    #[pyo3(get, set)]
    pub target_triple: Option<String>,
}

#[pymethods]
impl PyPmirConfig {
    #[new]
    fn new() -> Self {
        Self {
            debug_output: false,
            optimization_level: 2,
            target_triple: None,
        }
    }
}

impl From<PyPmirConfig> for PmirConfig {
    fn from(py_config: PyPmirConfig) -> Self {
        PmirConfig {
            debug_output: py_config.debug_output,
            optimization_level: py_config.optimization_level,
            target_triple: py_config.target_triple,
        }
    }
}

/// Compile HUGR JSON to LLVM IR using the PMIR pipeline
#[pyfunction]
#[pyo3(name = "compile_hugr_via_pmir")]
pub fn py_compile_hugr_via_pmir(hugr_json: &str, config: Option<PyPmirConfig>) -> PyResult<String> {
    let config = config.map(Into::into).unwrap_or_default();

    compile_hugr_via_pmir(hugr_json, &config)
        .map_err(|e| PyRuntimeError::new_err(format!("PMIR compilation failed: {:?}", e)))
}

/// Get the intermediate PAST representation as RON
#[pyfunction]
#[pyo3(name = "hugr_to_past_ron")]
pub fn py_hugr_to_past_ron(hugr_json: &str) -> PyResult<String> {
    use super::hugr_parser::parse_hugr_to_past;

    let past = parse_hugr_to_past(hugr_json)
        .map_err(|e| PyRuntimeError::new_err(format!("HUGR parsing failed: {:?}", e)))?;

    past.to_ron_string()
        .map_err(|e| PyRuntimeError::new_err(format!("RON serialization failed: {:?}", e)))
}

/// Register PMIR Python module
/// This would be used if pecos-pmir was exposed as a standalone Python module
/// Currently PMIR is exposed through pecos-rslib instead
#[allow(dead_code)]
pub fn register_pmir_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let pmir_module = PyModule::new(parent.py(), "pmir")?;

    pmir_module.add_class::<PyPmirConfig>()?;
    pmir_module.add_function(wrap_pyfunction!(py_compile_hugr_via_pmir, &pmir_module)?)?;
    pmir_module.add_function(wrap_pyfunction!(py_hugr_to_past_ron, &pmir_module)?)?;

    parent.add_submodule(&pmir_module)?;
    Ok(())
}
