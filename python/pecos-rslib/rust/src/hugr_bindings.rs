/*!
`PyO3` bindings for HUGR/QIR functionality

This module exposes HUGR compilation and QIR engine functionality to Python.
*/

use pecos_qir::python_api;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Global storage for QIR engines (in a real implementation, this would be more sophisticated)
#[allow(dead_code)]
static QIR_ENGINES: LazyLock<Mutex<HashMap<usize, Box<dyn pecos_engines::ClassicalEngine>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static mut NEXT_ENGINE_ID: usize = 1;

/// Get the next available engine ID
fn get_next_engine_id() -> usize {
    unsafe {
        let id = NEXT_ENGINE_ID;
        NEXT_ENGINE_ID += 1;
        id
    }
}

/// Python wrapper for HUGR compiler
#[pyclass(name = "HugrCompiler")]
pub struct PyHugrCompiler {
    debug_info: bool,
    naming_convention: String,
}

#[pymethods]
impl PyHugrCompiler {
    /// Create a new HUGR compiler
    ///
    /// # Arguments
    /// * `debug_info` - Whether to include debug information
    /// * `naming_convention` - Quantum operation naming convention ("standard", "hugr", "pecos")
    #[new]
    fn new(debug_info: Option<bool>, naming_convention: Option<String>) -> Self {
        Self {
            debug_info: debug_info.unwrap_or(false),
            naming_convention: naming_convention.unwrap_or_else(|| "standard".to_string()),
        }
    }

    /// Compile HUGR bytes to QIR string
    ///
    /// # Arguments
    /// * `hugr_bytes` - HUGR data as bytes
    ///
    /// # Returns
    /// QIR as a string
    fn compile_bytes_to_qir(&self, hugr_bytes: &Bound<'_, PyBytes>) -> PyResult<String> {
        let bytes = hugr_bytes.as_bytes();
        python_api::compile_hugr_bytes_to_qir_string(
            bytes,
            self.debug_info,
            &self.naming_convention,
        )
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
    }

    /// Compile HUGR file to QIR file
    ///
    /// # Arguments
    /// * `hugr_path` - Path to HUGR file
    /// * `qir_path` - Path for output QIR file
    fn compile_file_to_qir(&self, hugr_path: &str, qir_path: &str) -> PyResult<()> {
        python_api::compile_hugr_file_to_qir_file(
            hugr_path,
            qir_path,
            self.debug_info,
            &self.naming_convention,
        )
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
    }

    /// Set debug information flag
    fn set_debug_info(&mut self, debug_info: bool) {
        self.debug_info = debug_info;
    }

    /// Set quantum operation naming convention
    fn set_naming_convention(&mut self, naming_convention: String) -> PyResult<()> {
        let supported = python_api::get_supported_naming_conventions();
        if !supported.contains(&naming_convention) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unsupported naming convention: {naming_convention}. Supported: {supported:?}"
            )));
        }
        self.naming_convention = naming_convention;
        Ok(())
    }

    /// Get current naming convention
    fn get_naming_convention(&self) -> String {
        self.naming_convention.clone()
    }

    /// Get supported naming conventions
    #[staticmethod]
    fn get_supported_naming_conventions() -> Vec<String> {
        python_api::get_supported_naming_conventions()
    }
}

/// Python wrapper for HUGR QIR engine
#[pyclass(name = "HugrQirEngine")]
pub struct PyHugrQirEngine {
    engine_id: usize,
    shots: usize,
}

#[pymethods]
impl PyHugrQirEngine {
    /// Create QIR engine from HUGR bytes
    ///
    /// # Arguments
    /// * `hugr_bytes` - HUGR data as bytes
    /// * `shots` - Number of shots to assign to the engine
    /// * `debug_info` - Whether to include debug information
    /// * `naming_convention` - Quantum operation naming convention
    #[new]
    fn new(
        hugr_bytes: &Bound<'_, PyBytes>,
        shots: Option<usize>,
        debug_info: Option<bool>,
        naming_convention: Option<String>,
    ) -> PyResult<Self> {
        let bytes = hugr_bytes.as_bytes();
        let shots = shots.unwrap_or(1000);
        let debug_info = debug_info.unwrap_or(false);
        let naming_convention = naming_convention.unwrap_or_else(|| "standard".to_string());

        // For now, just return a dummy engine ID
        // In a full implementation, we'd actually create the engine and store it
        let engine_id = get_next_engine_id();

        // Validate by attempting compilation
        python_api::create_qir_engine_from_hugr_bytes(bytes, shots, debug_info, &naming_convention)
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        Ok(Self { engine_id, shots })
    }

    /// Create QIR engine from HUGR file
    ///
    /// # Arguments
    /// * `hugr_path` - Path to HUGR file
    /// * `shots` - Number of shots to assign to the engine
    /// * `debug_info` - Whether to include debug information
    /// * `naming_convention` - Quantum operation naming convention
    #[classmethod]
    fn from_file(
        _cls: &Bound<'_, PyType>,
        hugr_path: &str,
        shots: Option<usize>,
        debug_info: Option<bool>,
        naming_convention: Option<String>,
    ) -> PyResult<Self> {
        let shots = shots.unwrap_or(1000);
        let debug_info = debug_info.unwrap_or(false);
        let naming_convention = naming_convention.unwrap_or_else(|| "standard".to_string());

        // For now, just return a dummy engine ID
        let engine_id = get_next_engine_id();

        // Validate by attempting compilation
        python_api::create_qir_engine_from_hugr_file(
            hugr_path,
            shots,
            debug_info,
            &naming_convention,
        )
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        Ok(Self { engine_id, shots })
    }

    /// Get the number of shots assigned to this engine
    fn get_shots(&self) -> usize {
        self.shots
    }

    /// Set the number of shots for this engine
    fn set_shots(&mut self, shots: usize) {
        self.shots = shots;
    }

    /// Get the engine ID
    fn get_engine_id(&self) -> usize {
        self.engine_id
    }

    /// Run the quantum program (placeholder implementation)
    #[allow(clippy::unnecessary_wraps)] // PyO3 requires PyResult even for infallible methods
    fn run(&self) -> PyResult<Vec<u8>> {
        // This is a placeholder - in a full implementation, we'd:
        // 1. Get the engine from the global storage
        // 2. Execute it for the specified number of shots
        // 3. Return the results

        // Use self.shots to generate dummy results
        let mut results = Vec::with_capacity(self.shots);
        for i in 0..self.shots {
            results.push(u8::try_from(i % 2).unwrap_or(0)); // Alternate 0 and 1
        }
        Ok(results)
    }

    /// Get string representation
    fn __repr__(&self) -> String {
        format!("HugrQirEngine(id={}, shots={})", self.engine_id, self.shots)
    }
}

/// Check if HUGR support is available
#[pyfunction]
fn is_hugr_support_available() -> bool {
    python_api::is_hugr_support_available()
}

/// Get supported quantum operation naming conventions
#[pyfunction]
fn get_supported_naming_conventions() -> Vec<String> {
    python_api::get_supported_naming_conventions()
}

/// Compile HUGR bytes to QIR string (standalone function)
#[pyfunction]
fn compile_hugr_bytes_to_qir(
    hugr_bytes: &Bound<'_, PyBytes>,
    debug_info: Option<bool>,
    naming_convention: Option<String>,
) -> PyResult<String> {
    let bytes = hugr_bytes.as_bytes();
    let debug_info = debug_info.unwrap_or(false);
    let naming_convention = naming_convention.unwrap_or_else(|| "standard".to_string());

    python_api::compile_hugr_bytes_to_qir_string(bytes, debug_info, &naming_convention)
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
}

/// Compile HUGR file to QIR file (standalone function)
#[pyfunction]
fn compile_hugr_file_to_qir(
    hugr_path: &str,
    qir_path: &str,
    debug_info: Option<bool>,
    naming_convention: Option<String>,
) -> PyResult<()> {
    let debug_info = debug_info.unwrap_or(false);
    let naming_convention = naming_convention.unwrap_or_else(|| "standard".to_string());

    python_api::compile_hugr_file_to_qir_file(hugr_path, qir_path, debug_info, &naming_convention)
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
}

/// Register HUGR-related functions and classes with the Python module
pub fn register_hugr_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes
    m.add_class::<PyHugrCompiler>()?;
    m.add_class::<PyHugrQirEngine>()?;

    // Add standalone functions
    m.add_function(wrap_pyfunction!(is_hugr_support_available, m)?)?;
    m.add_function(wrap_pyfunction!(get_supported_naming_conventions, m)?)?;
    m.add_function(wrap_pyfunction!(compile_hugr_bytes_to_qir, m)?)?;
    m.add_function(wrap_pyfunction!(compile_hugr_file_to_qir, m)?)?;

    Ok(())
}
