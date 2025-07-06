/*!
Python API for PHIR compilation pipeline

This module provides Python-accessible functions for the PHIR pipeline.
*/

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::{PhirConfig, compile_hugr_via_phir};

/// Python-accessible PHIR configuration
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

impl From<PyPmirConfig> for PhirConfig {
    fn from(py_config: PyPmirConfig) -> Self {
        PhirConfig {
            debug: py_config.debug_output,
            optimization_level: py_config.optimization_level,
            target_triple: py_config.target_triple,
            generate_llvm_ir: true, // Default to generating LLVM IR
        }
    }
}

/// Compile HUGR JSON to LLVM IR using the PHIR pipeline
#[pyfunction]
#[pyo3(name = "compile_hugr_via_phir")]
pub fn py_compile_hugr_via_phir(hugr_json: &str, config: Option<PyPmirConfig>) -> PyResult<String> {
    let config = config.map(Into::into).unwrap_or_default();

    compile_hugr_via_phir(hugr_json, &config)
        .map_err(|e| PyRuntimeError::new_err(format!("PHIR compilation failed: {:?}", e)))
}

/// Register PHIR Python module
/// This would be used if pecos-pmir was exposed as a standalone Python module
/// Currently PHIR is exposed through pecos-rslib instead
#[allow(dead_code)]
pub fn register_pmir_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let pmir_module = PyModule::new(parent.py(), "pmir")?;

    pmir_module.add_class::<PyPmirConfig>()?;
    pmir_module.add_function(wrap_pyfunction!(py_compile_hugr_via_phir, &pmir_module)?)?;

    parent.add_submodule(&pmir_module)?;
    Ok(())
}
