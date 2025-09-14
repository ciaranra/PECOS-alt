use pecos_engines::ClassicalEngine;
use pecos_selene::selene_library_engine::SeleneLibraryEngine;
/// Python bindings for `SeleneLibraryEngine`
use pyo3::prelude::*;
use std::path::PathBuf;

/// Python wrapper for `SeleneLibraryEngine`
#[pyclass(name = "SeleneLibraryEngine")]
pub struct PySeleneLibraryEngine {
    engine: SeleneLibraryEngine,
}

#[pymethods]
impl PySeleneLibraryEngine {
    /// Create a new `SeleneLibraryEngine`
    #[new]
    pub fn new(library_path: String, num_qubits: usize) -> PyResult<Self> {
        let path = PathBuf::from(library_path);
        if !path.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(
                format!("Library file not found: {}", path.display()),
            ));
        }

        Ok(Self {
            engine: SeleneLibraryEngine::new(path, num_qubits),
        })
    }

    /// Get the number of qubits
    pub fn num_qubits(&self) -> usize {
        self.engine.num_qubits()
    }

    /// Reset the engine for a new shot
    pub fn reset(&mut self) -> PyResult<()> {
        self.engine.reset().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Reset failed: {e}"))
        })
    }

    /// Get the library path as a string
    #[allow(clippy::unused_self)] // TODO: Implement properly when library_path field is accessible
    pub fn library_path(&self) -> String {
        // Access the library_path field (need to make it pub in the Rust side)
        // For now, return a placeholder
        "library_path".to_string()
    }
}
