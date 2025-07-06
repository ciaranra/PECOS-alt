/*!
`PyO3` bindings for HUGR/LLVM functionality

This module exposes HUGR compilation and LLVM engine functionality to Python.
*/

use pecos_hugr_llvm::{HugrCompiler, HugrCompilerConfig};
use pecos_llvm_runtime::LlvmEngine;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use tempfile::TempDir;

static mut NEXT_ENGINE_ID: usize = 1;

/// Storage entry for LLVM engines - stores both the engine and the temporary directory
pub struct LlvmEngineEntry {
    pub engine: LlvmEngine,
    _temp_dir: Option<TempDir>, // Keep the temp dir alive
}

/// Global storage for LLVM engines when called from Python bindings
pub static PYTHON_LLVM_ENGINES: LazyLock<Mutex<HashMap<usize, LlvmEngineEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
}

#[pymethods]
impl PyHugrCompiler {
    /// Create a new HUGR compiler
    ///
    /// # Arguments
    /// * `debug_info` - Whether to include debug information
    #[new]
    fn new(debug_info: Option<bool>) -> Self {
        Self {
            debug_info: debug_info.unwrap_or(false),
        }
    }

    /// Compile HUGR bytes to LLVM IR string
    ///
    /// # Arguments
    /// * `hugr_bytes` - HUGR data as bytes
    ///
    /// # Returns
    /// LLVM IR as a string
    fn compile_bytes_to_llvm(&self, hugr_bytes: &Bound<'_, PyBytes>) -> PyResult<String> {
        let bytes = hugr_bytes.as_bytes();

        // Use the pure compilation crate
        let compiler = if self.debug_info {
            HugrCompiler::new().with_debug_info(true)
        } else {
            HugrCompiler::new()
        };

        compiler
            .compile_hugr_bytes_to_string(bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Compile HUGR bytes to LLVM IR file
    ///
    /// # Arguments
    /// * `hugr_bytes` - HUGR data as bytes
    /// * `llvm_path` - Path for output LLVM IR file
    fn compile_bytes_to_llvm_file(
        &self,
        hugr_bytes: &Bound<'_, PyBytes>,
        llvm_path: &str,
    ) -> PyResult<()> {
        let config = HugrCompilerConfig {
            output_path: Some(PathBuf::from(llvm_path)),
            debug_info: self.debug_info,
        };

        let compiler = HugrCompiler::with_config(config.clone());
        let bytes = hugr_bytes.as_bytes();

        // Compile directly to the output path
        compiler
            .compile_hugr_bytes(bytes, config.output_path.as_ref().unwrap())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(())
    }

    /// Compile HUGR file to LLVM IR file
    ///
    /// # Arguments
    /// * `hugr_path` - Path to HUGR file
    /// * `llvm_path` - Path for output LLVM IR file
    fn compile_file_to_llvm(&self, hugr_path: &str, llvm_path: &str) -> PyResult<()> {
        let config = HugrCompilerConfig {
            output_path: Some(PathBuf::from(llvm_path)),
            debug_info: self.debug_info,
        };

        let compiler = HugrCompiler::with_config(config);
        compiler
            .compile_hugr(hugr_path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(())
    }

    /// Set debug information flag
    fn set_debug_info(&mut self, debug_info: bool) {
        self.debug_info = debug_info;
    }

    /// Compile HUGR bytes to QIR string (deprecated, use compile_bytes_to_llvm)
    fn compile_bytes_to_qir(&self, hugr_bytes: &Bound<'_, PyBytes>) -> PyResult<String> {
        self.compile_bytes_to_llvm(hugr_bytes)
    }

    /// Compile HUGR file to QIR file (deprecated, use compile_file_to_llvm)
    fn compile_file_to_qir(&self, hugr_path: &str, qir_path: &str) -> PyResult<()> {
        self.compile_file_to_llvm(hugr_path, qir_path)
    }
}

/// Python wrapper for HUGR LLVM engine
#[pyclass(name = "HugrLlvmEngine")]
pub struct PyHugrLlvmEngine {
    engine_id: usize,
    shots: usize,
}

#[pymethods]
impl PyHugrLlvmEngine {
    /// Create LLVM engine from HUGR bytes
    ///
    /// # Arguments
    /// * `hugr_bytes` - HUGR data as bytes
    /// * `shots` - Number of shots to assign to the engine
    /// * `debug_info` - Whether to include debug information
    #[new]
    fn new(
        hugr_bytes: &Bound<'_, PyBytes>,
        shots: Option<usize>,
        debug_info: Option<bool>,
    ) -> PyResult<Self> {
        let bytes = hugr_bytes.as_bytes();
        let shots = shots.unwrap_or(1000);
        let debug_info = debug_info.unwrap_or(false);

        // Step 1: Compile HUGR to LLVM IR
        let compiler = if debug_info {
            HugrCompiler::new().with_debug_info(true)
        } else {
            HugrCompiler::new()
        };

        let llvm_ir = compiler
            .compile_hugr_bytes_to_string(bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Step 2: Create temporary file for LLVM IR
        let temp_dir = TempDir::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        let llvm_path = temp_dir.path().join("output.ll");

        std::fs::write(&llvm_path, llvm_ir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Step 3: Create LLVM engine
        let mut engine = LlvmEngine::new(llvm_path);
        engine.set_assigned_shots(shots);

        // Pre-compile the engine
        engine
            .pre_compile()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Store the engine
        let engine_id = get_next_engine_id();
        let entry = LlvmEngineEntry {
            engine,
            _temp_dir: Some(temp_dir),
        };

        PYTHON_LLVM_ENGINES.lock().unwrap().insert(engine_id, entry);

        Ok(Self { engine_id, shots })
    }

    /// Create LLVM engine from HUGR file
    ///
    /// # Arguments
    /// * `hugr_path` - Path to HUGR file
    /// * `shots` - Number of shots to assign to the engine
    /// * `debug_info` - Whether to include debug information
    #[classmethod]
    fn from_file(
        _cls: &Bound<'_, PyType>,
        hugr_path: &str,
        shots: Option<usize>,
        debug_info: Option<bool>,
    ) -> PyResult<Self> {
        let shots = shots.unwrap_or(1000);
        let debug_info = debug_info.unwrap_or(false);

        // Step 1: Compile HUGR to LLVM IR
        let temp_dir = TempDir::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        let llvm_path = temp_dir.path().join("output.ll");

        let config = HugrCompilerConfig {
            output_path: Some(llvm_path.clone()),
            debug_info,
        };

        let compiler = HugrCompiler::with_config(config);
        compiler
            .compile_hugr(hugr_path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Step 2: Create LLVM engine
        let mut engine = LlvmEngine::new(llvm_path);
        engine.set_assigned_shots(shots);

        // Pre-compile the engine
        engine
            .pre_compile()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Store the engine
        let engine_id = get_next_engine_id();
        let entry = LlvmEngineEntry {
            engine,
            _temp_dir: Some(temp_dir),
        };

        PYTHON_LLVM_ENGINES.lock().unwrap().insert(engine_id, entry);

        Ok(Self { engine_id, shots })
    }

    /// Run the LLVM engine and return measurement results
    ///
    /// # Returns
    /// List of measurement results (0 or 1)
    fn run(&self) -> PyResult<Vec<u8>> {
        use pecos_engines::run_sim;

        let mut engines = PYTHON_LLVM_ENGINES.lock().unwrap();
        let entry = engines.get_mut(&self.engine_id).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Engine {} not found",
                self.engine_id
            ))
        })?;

        // Clone the engine to use as a ClassicalEngine
        let engine_clone = entry.engine.clone();

        // Use run_sim with the proper architecture
        let results = run_sim(
            Box::new(engine_clone),
            self.shots,
            None, // seed
            None, // workers
            None, // noise_model
            None, // quantum_engine
        )
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Extract measurement results - take the first measurement from each shot
        let mut measurements = Vec::with_capacity(self.shots);
        for shot in results.shots {
            // Find the first measurement value
            let measurement = shot
                .data
                .values()
                .find_map(|data| match data {
                    pecos_engines::shot_results::Data::U32(v) => Some(*v != 0),
                    pecos_engines::shot_results::Data::I64(v) => Some(*v != 0),
                    pecos_engines::shot_results::Data::U8(v) => Some(*v != 0),
                    _ => None,
                })
                .unwrap_or(false);
            measurements.push(u8::from(measurement));
        }

        Ok(measurements)
    }

    /// Reset the engine state
    fn reset(&mut self) -> PyResult<()> {
        let mut engines = PYTHON_LLVM_ENGINES.lock().unwrap();
        let entry = engines.get_mut(&self.engine_id).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Engine {} not found",
                self.engine_id
            ))
        })?;

        // Reset by creating a new engine with the same configuration
        let llvm_path = entry.engine.get_llvm_file().to_path_buf();
        let mut new_engine = LlvmEngine::new(llvm_path);
        new_engine.set_assigned_shots(self.shots);
        new_engine
            .pre_compile()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        entry.engine = new_engine;
        Ok(())
    }

    /// Get the number of shots assigned to this engine
    fn get_shots(&self) -> usize {
        self.shots
    }

    /// Get the engine ID
    fn get_engine_id(&self) -> usize {
        self.engine_id
    }

    /// Create QIR engine from HUGR bytes (deprecated, use new)
    #[classmethod]
    fn from_hugr_bytes(
        _cls: &Bound<'_, PyType>,
        hugr_bytes: &Bound<'_, PyBytes>,
        shots: Option<usize>,
        debug_info: Option<bool>,
    ) -> PyResult<Self> {
        Self::new(hugr_bytes, shots, debug_info)
    }

    /// Create QIR engine from HUGR file (deprecated, use from_file)
    #[classmethod]
    fn from_hugr_file(
        cls: &Bound<'_, PyType>,
        hugr_path: &str,
        shots: Option<usize>,
        debug_info: Option<bool>,
    ) -> PyResult<Self> {
        Self::from_file(cls, hugr_path, shots, debug_info)
    }
}

impl Drop for PyHugrLlvmEngine {
    fn drop(&mut self) {
        // Remove from storage when dropped
        let _ = PYTHON_LLVM_ENGINES.lock().unwrap().remove(&self.engine_id);
    }
}

/// Python function to check if HUGR support is available
#[pyfunction]
fn is_hugr_supported() -> bool {
    // Always true now that we have the separate pecos-hugr-llvm crate
    true
}

/// Python module for HUGR bindings
pub fn register_hugr_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes directly to parent module with expected names
    parent_module.add_class::<PyHugrCompiler>()?;
    parent_module.add_class::<PyHugrLlvmEngine>()?;
    parent_module.add_function(wrap_pyfunction!(is_hugr_supported, parent_module)?)?;

    // Also create convenience functions with expected names
    parent_module.add_function(wrap_pyfunction!(compile_hugr_bytes_to_llvm, parent_module)?)?;
    parent_module.add_function(wrap_pyfunction!(compile_hugr_file_to_llvm, parent_module)?)?;
    parent_module.add_function(wrap_pyfunction!(is_hugr_support_available, parent_module)?)?;

    Ok(())
}

/// Compile HUGR bytes to LLVM IR
#[pyfunction]
fn compile_hugr_bytes_to_llvm(
    hugr_bytes: &Bound<'_, PyBytes>,
    output_path: Option<String>,
    debug_info: Option<bool>,
) -> PyResult<Option<String>> {
    let compiler = PyHugrCompiler::new(debug_info);
    if let Some(path) = output_path {
        compiler.compile_bytes_to_llvm_file(hugr_bytes, &path)?;
        Ok(None)
    } else {
        Ok(Some(compiler.compile_bytes_to_llvm(hugr_bytes)?))
    }
}

/// Compile HUGR file to LLVM IR file
#[pyfunction]
fn compile_hugr_file_to_llvm(
    hugr_path: &str,
    llvm_path: &str,
    debug_info: Option<bool>,
) -> PyResult<()> {
    let compiler = PyHugrCompiler::new(debug_info);
    compiler.compile_file_to_llvm(hugr_path, llvm_path)
}

/// Check if HUGR support is available
#[pyfunction]
fn is_hugr_support_available() -> bool {
    is_hugr_supported()
}
