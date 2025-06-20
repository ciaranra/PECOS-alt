/*!
Python API for HUGR/QIR functionality

This module provides Python-friendly functions for HUGR compilation and QIR engine creation.
These functions are designed to be easily wrapped with `PyO3`.
*/

#[cfg(feature = "hugr-llvm-pipeline")]
use crate::hugr::compiler::{HugrCompiler, HugrCompilerConfig, QuantumNamingConvention};
#[cfg(feature = "hugr-llvm-pipeline")]
use crate::QirEngine;
#[cfg(feature = "hugr-llvm-pipeline")]
use pecos_core::errors::PecosError;
#[cfg(feature = "hugr-llvm-pipeline")]
use tempfile::TempDir;
#[cfg(feature = "hugr-llvm-pipeline")]
use std::path::PathBuf;

/// Result type for Python API functions
pub type PyResult<T> = Result<T, String>;

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
/// Compile HUGR bytes to QIR string
///
/// # Arguments
/// * `hugr_bytes` - HUGR data as bytes
/// * `debug_info` - Whether to include debug information
/// * `naming_convention` - Quantum operation naming convention ("standard", "hugr", "pecos")
///
/// # Returns
/// QIR as a string
///
/// # Errors
/// Returns an error if:
/// - Unknown naming convention is provided
/// - Failed to create temporary directory
/// - HUGR compilation fails
/// - Failed to read the generated QIR file
pub fn compile_hugr_bytes_to_qir_string(
    hugr_bytes: &[u8],
    debug_info: bool,
    naming_convention: &str,
) -> PyResult<String> {
    // Parse naming convention
    let naming = match naming_convention {
        "standard" | "qir" => QuantumNamingConvention::StandardQir,
        "hugr" => QuantumNamingConvention::Hugr,
        "pecos" => QuantumNamingConvention::Pecos,
        _ => return Err(format!("Unknown naming convention: {naming_convention}")),
    };

    // Create temporary output file for QIR
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let qir_path = temp_dir.path().join("output.ll");

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(qir_path.clone()),
        debug_info,
        quantum_naming: naming,
    };

    // Compile HUGR bytes to QIR (this will use our transformation)
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr_bytes(hugr_bytes, &qir_path)
        .map_err(|e| convert_error(&e))?;

    // Read QIR as string
    std::fs::read_to_string(&qir_path).map_err(|e| format!("Failed to read QIR file: {e}"))
}

/// Compile HUGR file to QIR file
///
/// # Arguments
/// * `hugr_path` - Path to HUGR file
/// * `qir_path` - Path for output QIR file
/// * `debug_info` - Whether to include debug information
/// * `naming_convention` - Quantum operation naming convention
///
/// # Returns
/// Success indicator
///
/// # Errors
/// Returns an error if:
/// - Unknown naming convention is provided
/// - HUGR compilation fails
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn compile_hugr_file_to_qir_file(
    hugr_path: &str,
    qir_path: &str,
    debug_info: bool,
    naming_convention: &str,
) -> PyResult<()> {
    // Parse naming convention
    let naming = match naming_convention {
        "standard" | "qir" => QuantumNamingConvention::StandardQir,
        "hugr" => QuantumNamingConvention::Hugr,
        "pecos" => QuantumNamingConvention::Pecos,
        _ => return Err(format!("Unknown naming convention: {naming_convention}")),
    };

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(PathBuf::from(qir_path)),
        debug_info,
        quantum_naming: naming,
    };

    // Compile HUGR to QIR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr(hugr_path)
        .map_err(|e| convert_error(&e))?;

    Ok(())
}

/// Create a QIR engine from HUGR bytes
///
/// # Arguments
/// * `hugr_bytes` - HUGR data as bytes
/// * `shots` - Number of shots to assign to the engine
/// * `debug_info` - Whether to include debug information
/// * `naming_convention` - Quantum operation naming convention
///
/// # Returns
/// Opaque handle to the QIR engine
///
/// # Errors
/// Returns an error if:
/// - Unknown naming convention is provided
/// - Failed to create temporary directory
/// - Failed to write HUGR file
/// - HUGR compilation fails
/// - QIR engine pre-compilation fails
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn create_qir_engine_from_hugr_bytes(
    hugr_bytes: &[u8],
    shots: usize,
    debug_info: bool,
    naming_convention: &str,
) -> PyResult<usize> {
    // Parse naming convention
    let naming = match naming_convention {
        "standard" | "qir" => QuantumNamingConvention::StandardQir,
        "hugr" => QuantumNamingConvention::Hugr,
        "pecos" => QuantumNamingConvention::Pecos,
        _ => return Err(format!("Unknown naming convention: {naming_convention}")),
    };

    // Create temporary file for HUGR
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let hugr_path = temp_dir.path().join("input.hugr");
    let qir_path = temp_dir.path().join("output.ll");

    // Write HUGR bytes to file
    std::fs::write(&hugr_path, hugr_bytes)
        .map_err(|e| format!("Failed to write HUGR file: {e}"))?;

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(qir_path.clone()),
        debug_info,
        quantum_naming: naming,
    };

    // Compile HUGR bytes to QIR (this will use our transformation)
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr_bytes(hugr_bytes, &qir_path)
        .map_err(|e| convert_error(&e))?;

    // Create QIR engine
    let mut qir_engine = QirEngine::new(qir_path);
    qir_engine.set_assigned_shots(shots);
    qir_engine.pre_compile().map_err(|e| convert_error(&e))?;

    // For now, return a dummy handle - in a full implementation,
    // we'd store the engine in a global map with a unique ID
    Ok(0)
}

/// Create a QIR engine from HUGR file
///
/// # Arguments
/// * `hugr_path` - Path to HUGR file
/// * `shots` - Number of shots to assign to the engine
/// * `debug_info` - Whether to include debug information
/// * `naming_convention` - Quantum operation naming convention
///
/// # Returns
/// Opaque handle to the QIR engine
///
/// # Errors
/// Returns an error if:
/// - Unknown naming convention is provided
/// - Failed to create temporary directory
/// - HUGR compilation fails
/// - QIR engine pre-compilation fails
#[cfg(feature = "hugr-llvm-pipeline")]
pub fn create_qir_engine_from_hugr_file(
    hugr_path: &str,
    shots: usize,
    debug_info: bool,
    naming_convention: &str,
) -> PyResult<usize> {
    // Parse naming convention
    let naming = match naming_convention {
        "standard" | "qir" => QuantumNamingConvention::StandardQir,
        "hugr" => QuantumNamingConvention::Hugr,
        "pecos" => QuantumNamingConvention::Pecos,
        _ => return Err(format!("Unknown naming convention: {naming_convention}")),
    };

    // Create temporary directory for compilation
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let qir_path = temp_dir.path().join("output.ll");

    // Set up compiler configuration
    let config = HugrCompilerConfig {
        output_path: Some(qir_path.clone()),
        debug_info,
        quantum_naming: naming,
    };

    // Compile HUGR to QIR
    let compiler = HugrCompiler::with_config(config);
    compiler
        .compile_hugr(hugr_path)
        .map_err(|e| convert_error(&e))?;

    // Create QIR engine
    let mut qir_engine = QirEngine::new(qir_path);
    qir_engine.set_assigned_shots(shots);
    qir_engine.pre_compile().map_err(|e| convert_error(&e))?;

    // For now, return a dummy handle - in a full implementation,
    // we'd store the engine in a global map with a unique ID
    Ok(0)
}

/// Get the supported quantum operation naming conventions
#[cfg(feature = "hugr-llvm-pipeline")]
#[must_use]
pub fn get_supported_naming_conventions() -> Vec<String> {
    vec![
        "standard".to_string(),
        "qir".to_string(),
        "hugr".to_string(),
        "pecos".to_string(),
    ]
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
pub fn compile_hugr_bytes_to_qir_string(
    _hugr_bytes: &[u8],
    _debug_info: bool,
    _naming_convention: &str,
) -> PyResult<String> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn compile_hugr_file_to_qir_file(
    _hugr_path: &str,
    _qir_path: &str,
    _debug_info: bool,
    _naming_convention: &str,
) -> PyResult<()> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn create_qir_engine_from_hugr_bytes(
    _hugr_bytes: &[u8],
    _shots: usize,
    _debug_info: bool,
    _naming_convention: &str,
) -> PyResult<usize> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn create_qir_engine_from_hugr_file(
    _hugr_path: &str,
    _shots: usize,
    _debug_info: bool,
    _naming_convention: &str,
) -> PyResult<usize> {
    Err("HUGR-LLVM pipeline not available".to_string())
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub fn get_supported_naming_conventions() -> Vec<String> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_conventions() {
        let conventions = get_supported_naming_conventions();
        #[cfg(feature = "hugr-llvm-pipeline")]
        {
            assert!(conventions.contains(&"standard".to_string()));
            assert!(conventions.contains(&"hugr".to_string()));
            assert!(conventions.contains(&"pecos".to_string()));
        }
        #[cfg(not(feature = "hugr-llvm-pipeline"))]
        {
            assert!(conventions.is_empty());
        }
    }

    #[test]
    fn test_hugr_support_check() {
        // This test just ensures the function exists and returns a boolean
        let _available = is_hugr_support_available();
    }

    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    #[test]
    fn test_hugr_compilation_fails_without_feature() {
        let result = compile_hugr_bytes_to_qir_string(&[0, 1, 2, 3], false, "standard");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HUGR-LLVM pipeline not available"));
    }
}
