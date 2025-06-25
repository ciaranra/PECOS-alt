/*!
`PyO3` bindings for HUGR/QIR functionality

This module exposes HUGR compilation and QIR engine functionality to Python.
*/

use pecos_qir::python_api;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};

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
    llvm_convention: String,
}

#[pymethods]
impl PyHugrCompiler {
    /// Create a new HUGR compiler
    ///
    /// # Arguments
    /// * `debug_info` - Whether to include debug information
    /// * `llvm_convention` - LLVM-IR convention ("hugr" or "qir")
    #[new]
    fn new(debug_info: Option<bool>, llvm_convention: Option<String>) -> Self {
        Self {
            debug_info: debug_info.unwrap_or(false),
            llvm_convention: llvm_convention.unwrap_or_else(|| "hugr".to_string()),
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
            &self.llvm_convention,
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
            &self.llvm_convention,
        )
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
    }

    /// Set debug information flag
    fn set_debug_info(&mut self, debug_info: bool) {
        self.debug_info = debug_info;
    }

    /// Set quantum operation naming convention
    fn set_llvm_convention(&mut self, llvm_convention: String) -> PyResult<()> {
        let supported = python_api::get_supported_llvm_conventions();
        if !supported.contains(&llvm_convention) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unsupported naming convention: {llvm_convention}. Supported: {supported:?}"
            )));
        }
        self.llvm_convention = llvm_convention;
        Ok(())
    }

    /// Get current naming convention
    fn get_llvm_convention(&self) -> String {
        self.llvm_convention.clone()
    }

    /// Get supported naming conventions
    #[staticmethod]
    fn get_supported_llvm_conventions() -> Vec<String> {
        python_api::get_supported_llvm_conventions()
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
    /// * `llvm_convention` - Quantum operation naming convention
    #[new]
    fn new(
        hugr_bytes: &Bound<'_, PyBytes>,
        shots: Option<usize>,
        debug_info: Option<bool>,
        llvm_convention: Option<String>,
    ) -> PyResult<Self> {
        let bytes = hugr_bytes.as_bytes();
        let shots = shots.unwrap_or(1000);
        let debug_info = debug_info.unwrap_or(false);
        let llvm_convention = llvm_convention.unwrap_or_else(|| "hugr".to_string());

        // Create the QIR engine and store it
        let engine_id = get_next_engine_id();
        
        let engine_result = python_api::create_qir_engine_from_hugr_bytes_with_storage(
            bytes, 
            shots, 
            debug_info, 
            &llvm_convention,
            engine_id
        )
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;

        Ok(Self { engine_id, shots })
    }

    /// Create QIR engine from HUGR file
    ///
    /// # Arguments
    /// * `hugr_path` - Path to HUGR file
    /// * `shots` - Number of shots to assign to the engine
    /// * `debug_info` - Whether to include debug information
    /// * `llvm_convention` - Quantum operation naming convention
    #[classmethod]
    fn from_file(
        _cls: &Bound<'_, PyType>,
        hugr_path: &str,
        shots: Option<usize>,
        debug_info: Option<bool>,
        llvm_convention: Option<String>,
    ) -> PyResult<Self> {
        let shots = shots.unwrap_or(1000);
        let debug_info = debug_info.unwrap_or(false);
        let llvm_convention = llvm_convention.unwrap_or_else(|| "hugr".to_string());

        // Create the QIR engine and store it
        let engine_id = get_next_engine_id();
        
        let engine_result = python_api::create_qir_engine_from_hugr_file_with_storage(
            hugr_path,
            shots,
            debug_info,
            &llvm_convention,
            engine_id
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

    /// Run the quantum program
    fn run(&self) -> PyResult<Vec<u8>> {
        // Get the engine from global storage and execute it
        let mut engines = python_api::get_stored_engine_mut(self.engine_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        
        let entry = engines.get_mut(&self.engine_id).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Engine {} not found", self.engine_id)
            )
        })?;
        
        let engine = &mut entry.engine;
        
        // Update shots if they've changed
        engine.set_assigned_shots(self.shots);
        
        // Execute the quantum program
        let results = engine.run().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Quantum execution failed: {e}")
            )
        })?;
        
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
fn get_supported_llvm_conventions() -> Vec<String> {
    python_api::get_supported_llvm_conventions()
}

/// Compile HUGR bytes to QIR string (standalone function)
#[pyfunction]
fn compile_hugr_bytes_to_qir(
    hugr_bytes: &Bound<'_, PyBytes>,
    debug_info: Option<bool>,
    llvm_convention: Option<String>,
) -> PyResult<String> {
    let bytes = hugr_bytes.as_bytes();
    let debug_info = debug_info.unwrap_or(false);
    let llvm_convention = llvm_convention.unwrap_or_else(|| "hugr".to_string());

    python_api::compile_hugr_bytes_to_qir_string(bytes, debug_info, &llvm_convention)
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
}

/// Compile HUGR file to QIR file (standalone function)
#[pyfunction]
fn compile_hugr_file_to_qir(
    hugr_path: &str,
    qir_path: &str,
    debug_info: Option<bool>,
    llvm_convention: Option<String>,
) -> PyResult<()> {
    let debug_info = debug_info.unwrap_or(false);
    let llvm_convention = llvm_convention.unwrap_or_else(|| "hugr".to_string());

    python_api::compile_hugr_file_to_qir_file(hugr_path, qir_path, debug_info, &llvm_convention)
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
}

/// Register HUGR-related functions and classes with the Python module
pub fn register_hugr_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes
    m.add_class::<PyHugrCompiler>()?;
    m.add_class::<PyHugrQirEngine>()?;

    // Add standalone functions
    m.add_function(wrap_pyfunction!(is_hugr_support_available, m)?)?;
    m.add_function(wrap_pyfunction!(get_supported_llvm_conventions, m)?)?;
    m.add_function(wrap_pyfunction!(compile_hugr_bytes_to_qir, m)?)?;
    m.add_function(wrap_pyfunction!(compile_hugr_file_to_qir, m)?)?;

    Ok(())
}
