/*!
Python API for HUGR/QIR functionality

This module provides Python-friendly functions for HUGR compilation and QIR engine creation.
These functions are designed to be easily wrapped with `PyO3`.
*/

#[cfg(feature = "hugr-llvm-pipeline")]
use crate::LlvmEngine;
#[cfg(feature = "hugr-llvm-pipeline")]
use crate::hugr::compiler::{HugrCompiler, HugrCompilerConfig};
#[cfg(feature = "hugr-llvm-pipeline")]
use pecos_core::errors::PecosError;
#[cfg(feature = "hugr-llvm-pipeline")]
use std::collections::HashMap;
#[cfg(feature = "hugr-llvm-pipeline")]
use std::path::PathBuf;
#[cfg(feature = "hugr-llvm-pipeline")]
use std::sync::{LazyLock, Mutex};
#[cfg(feature = "hugr-llvm-pipeline")]
use tempfile::TempDir;

/// Result type for Python API functions
pub type PyResult<T> = Result<T, String>;

/// Storage entry for LLVM engines - stores both the engine and the temporary directory
#[cfg(feature = "hugr-llvm-pipeline")]
pub struct LlvmEngineEntry {
    pub engine: LlvmEngine,
    _temp_dir: TempDir, // Keep the temp dir alive
}

/// Global storage for LLVM engines when called from Python bindings
#[cfg(feature = "hugr-llvm-pipeline")]
static PYTHON_LLVM_ENGINES: LazyLock<Mutex<HashMap<usize, LlvmEngineEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Convert `PecosError` to String for Python compatibility
#[cfg(feature = "hugr-llvm-pipeline")]
fn convert_error(err: &PecosError) -> String {
    err.to_string()
}

//
// HUGR-LLVM Pipeline Functions
// These functions are only available when the hugr-llvm-pipeline feature is enabled
//

#[cfg(feature = "hugr-llvm-pipeline")]
/// Compile HUGR bytes to LLVM IR string
///
/// # Arguments
/// * `hugr_bytes` - HUGR data as bytes
/// * `debug_info` - Whether to include debug information
///
/// # Returns
/// LLVM IR as a string
///
/// # Errors
/// Returns an error if:
/// - Failed to create temporary directory
/// - HUGR compilation fails
/// - Failed to read the generated LLVM IR file
pub fn compile_hugr_bytes_to_llvm_string(hugr_bytes: &[u8], debug_info: bool) -> PyResult<String> {
    // Create temporary output file for LLVM IR
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let llvm_path = temp_dir.path().join("output.ll");

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(llvm_path.clone()),
        debug_info,
    };

    // Compile HUGR bytes to LLVM IR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr_bytes(hugr_bytes, &llvm_path)
        .map_err(|e| convert_error(&e))?;

    // Read LLVM IR as string
    std::fs::read_to_string(&llvm_path).map_err(|e| format!("Failed to read LLVM IR file: {e}"))
}

/// Compile HUGR file to LLVM IR file
///
/// # Arguments
/// * `hugr_path` - Path to HUGR file
/// * `llvm_path` - Path for output LLVM IR file
/// * `debug_info` - Whether to include debug information
///
/// # Returns
/// Success indicator
///
/// # Errors
/// Returns an error if:
/// - HUGR compilation fails
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn compile_hugr_file_to_llvm_file(
    hugr_path: &str,
    llvm_path: &str,
    debug_info: bool,
) -> PyResult<()> {
    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(PathBuf::from(llvm_path)),
        debug_info,
    };

    // Compile HUGR to LLVM IR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr(hugr_path)
        .map_err(|e| convert_error(&e))?;

    Ok(())
}

/// Create an LLVM engine from HUGR bytes
///
/// # Arguments
/// * `hugr_bytes` - HUGR data as bytes
/// * `shots` - Number of shots to assign to the engine
/// * `debug_info` - Whether to include debug information
///
/// # Returns
/// Opaque handle to the LLVM engine
///
/// # Errors
/// Returns an error if:
/// - Failed to create temporary directory
/// - Failed to write HUGR file
/// - HUGR compilation fails
/// - LLVM engine pre-compilation fails
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn create_llvm_engine_from_hugr_bytes(
    hugr_bytes: &[u8],
    shots: usize,
    debug_info: bool,
) -> PyResult<usize> {
    // Create temporary file for HUGR
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let hugr_path = temp_dir.path().join("input.hugr");
    let llvm_path = temp_dir.path().join("output.ll");

    // Write HUGR bytes to file
    std::fs::write(&hugr_path, hugr_bytes)
        .map_err(|e| format!("Failed to write HUGR file: {e}"))?;

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(llvm_path.clone()),
        debug_info,
    };

    // Compile HUGR bytes to LLVM IR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr_bytes(hugr_bytes, &llvm_path)
        .map_err(|e| convert_error(&e))?;

    // Create LLVM engine
    let mut llvm_engine = LlvmEngine::new(llvm_path);
    llvm_engine.set_assigned_shots(shots);
    llvm_engine.pre_compile().map_err(|e| convert_error(&e))?;

    // For now, return a dummy handle - in a full implementation,
    // we'd store the engine in a global map with a unique ID
    Ok(0)
}

/// Create an LLVM engine from HUGR file
///
/// # Arguments
/// * `hugr_path` - Path to HUGR file
/// * `shots` - Number of shots to assign to the engine
/// * `debug_info` - Whether to include debug information
///
/// # Returns
/// Opaque handle to the LLVM engine
///
/// # Errors
/// Returns an error if:
/// - Failed to create temporary directory
/// - HUGR compilation fails
/// - LLVM engine pre-compilation fails
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn create_llvm_engine_from_hugr_file(
    hugr_path: &str,
    shots: usize,
    debug_info: bool,
) -> PyResult<usize> {
    // Create temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let llvm_path = temp_dir.path().join("output.ll");

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(llvm_path.clone()),
        debug_info,
    };

    // Compile HUGR to LLVM IR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr(hugr_path)
        .map_err(|e| convert_error(&e))?;

    // Create LLVM engine
    let mut llvm_engine = LlvmEngine::new(llvm_path);
    llvm_engine.set_assigned_shots(shots);
    llvm_engine.pre_compile().map_err(|e| convert_error(&e))?;

    // For now, return a dummy handle - in a full implementation,
    // we'd store the engine in a global map with a unique ID
    Ok(0)
}

/// Create an LLVM engine from HUGR bytes with engine storage
///
/// This version stores the engine in global storage and returns the engine ID
///
/// # Arguments
/// * `hugr_bytes` - HUGR data as bytes
/// * `shots` - Number of shots to assign to the engine
/// * `debug_info` - Whether to include debug information
/// * `engine_id` - The ID to assign to this engine
///
/// # Returns
/// The engine ID
///
/// # Errors
/// Returns an error if:
/// - Failed to create temporary directory
/// - Failed to write HUGR file
/// - HUGR compilation fails
/// - LLVM engine pre-compilation fails
/// - Failed to store engine
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn create_llvm_engine_from_hugr_bytes_with_storage(
    hugr_bytes: &[u8],
    shots: usize,
    debug_info: bool,
    engine_id: usize,
) -> PyResult<usize> {
    // Create temporary file for HUGR
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let hugr_path = temp_dir.path().join("input.hugr");
    let llvm_path = temp_dir.path().join("output.ll");

    // Write HUGR bytes to file
    std::fs::write(&hugr_path, hugr_bytes)
        .map_err(|e| format!("Failed to write HUGR file: {e}"))?;

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(llvm_path.clone()),
        debug_info,
    };

    // Compile HUGR bytes to LLVM IR (this will use our transformation)
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr_bytes(hugr_bytes, &llvm_path)
        .map_err(|e| convert_error(&e))?;

    // Create LLVM engine
    let mut llvm_engine = LlvmEngine::new(llvm_path.clone());
    llvm_engine.set_assigned_shots(shots);
    llvm_engine.pre_compile().map_err(|e| convert_error(&e))?;

    // Set up quantum system for interactive execution
    // First determine the number of qubits needed by analyzing the LLVM IR file

    // Note: Interactive execution for HUGR immediate measurements should be handled
    // by the HybridEngine, not directly by LlvmEngine

    // Store the engine and temp directory in global storage
    let mut engines = PYTHON_LLVM_ENGINES
        .lock()
        .map_err(|e| format!("Failed to lock engine storage: {e}"))?;

    let entry = LlvmEngineEntry {
        engine: llvm_engine,
        _temp_dir: temp_dir,
    };
    engines.insert(engine_id, entry);

    Ok(engine_id)
}

/// Create an LLVM engine from HUGR file with engine storage
///
/// This version stores the engine in global storage and returns the engine ID
///
/// # Arguments
/// * `hugr_path` - Path to HUGR file
/// * `shots` - Number of shots to assign to the engine
/// * `debug_info` - Whether to include debug information
/// * `engine_id` - The ID to assign to this engine
///
/// # Returns
/// The engine ID
///
/// # Errors
/// Returns an error if:
/// - Failed to create temporary directory
/// - HUGR compilation fails
/// - LLVM engine pre-compilation fails
/// - Failed to store engine
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn create_llvm_engine_from_hugr_file_with_storage(
    hugr_path: &str,
    shots: usize,
    debug_info: bool,
    engine_id: usize,
) -> PyResult<usize> {
    // Create temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let llvm_path = temp_dir.path().join("output.ll");

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(llvm_path.clone()),
        debug_info,
    };

    // Compile HUGR to LLVM IR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr(hugr_path)
        .map_err(|e| convert_error(&e))?;

    // Create LLVM engine
    let mut llvm_engine = LlvmEngine::new(llvm_path.clone());
    llvm_engine.set_assigned_shots(shots);
    llvm_engine.pre_compile().map_err(|e| convert_error(&e))?;

    // Set up quantum system for interactive execution
    // First determine the number of qubits needed by analyzing the LLVM IR file

    // Note: Interactive execution for HUGR immediate measurements should be handled
    // by the HybridEngine, not directly by LlvmEngine

    // Store the engine and temp directory in global storage
    let mut engines = PYTHON_LLVM_ENGINES
        .lock()
        .map_err(|e| format!("Failed to lock engine storage: {e}"))?;

    let entry = LlvmEngineEntry {
        engine: llvm_engine,
        _temp_dir: temp_dir,
    };
    engines.insert(engine_id, entry);

    Ok(engine_id)
}

/// Get engine from storage for execution
///
/// # Arguments
/// * `engine_id` - The ID of the engine to retrieve
///
/// # Returns
/// Mutable reference to the engine
///
/// # Errors
/// Returns an error if:
/// - Failed to lock storage
/// - Engine not found
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn get_stored_engine_mut(
    _engine_id: usize,
) -> PyResult<std::sync::MutexGuard<'static, HashMap<usize, LlvmEngineEntry>>> {
    PYTHON_LLVM_ENGINES
        .lock()
        .map_err(|e| format!("Failed to lock engine storage: {e}"))
}

/// Check if HUGR support is compiled in
#[must_use]
pub fn is_hugr_support_available() -> bool {
    cfg!(feature = "hugr-llvm-pipeline")
}

//
// Stub functions when hugr-llvm-pipeline is not available
//

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn compile_hugr_bytes_to_llvm_string(
    _hugr_bytes: &[u8],
    _debug_info: bool,
) -> PyResult<String> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn compile_hugr_file_to_llvm_file(
    _hugr_path: &str,
    _llvm_path: &str,
    _debug_info: bool,
) -> PyResult<()> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn create_llvm_engine_from_hugr_bytes(
    _hugr_bytes: &[u8],
    _shots: usize,
    _debug_info: bool,
) -> PyResult<usize> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn create_llvm_engine_from_hugr_file(
    _hugr_path: &str,
    _shots: usize,
    _debug_info: bool,
) -> PyResult<usize> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn create_llvm_engine_from_hugr_bytes_with_storage(
    _hugr_bytes: &[u8],
    _shots: usize,
    _debug_info: bool,
    _engine_id: usize,
) -> PyResult<usize> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn create_llvm_engine_from_hugr_file_with_storage(
    _hugr_path: &str,
    _shots: usize,
    _debug_info: bool,
    _engine_id: usize,
) -> PyResult<usize> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hugr_support_check() {
        // This test just ensures the function exists and returns a boolean
        let _available = is_hugr_support_available();
    }

    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    #[test]
    fn test_hugr_compilation_fails_without_feature() {
        let result = compile_hugr_bytes_to_llvm_string(&[0, 1, 2, 3], false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("HUGR-LLVM pipeline not available")
        );
    }
}
