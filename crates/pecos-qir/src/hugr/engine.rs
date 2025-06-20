/*!
HUGR-enabled QIR Engine

This module provides functions to compile HUGR files to QIR and create engines from them.
This is a simplified interface that doesn't try to implement complex traits.
*/

use super::compiler::{HugrCompiler, HugrCompilerConfig};
use crate::QirEngine;
use log::info;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::path::Path;
use tempfile::TempDir;

/// Compile a HUGR file to QIR and create a QIR engine from it
///
/// # Arguments
/// * `hugr_path` - Path to the HUGR file to compile and load
/// * `shots` - Optional number of shots to assign to the engine
///
/// # Returns
/// A boxed `ClassicalEngine` instance ready for execution
///
/// # Errors
/// Returns `PecosError` if compilation or engine creation fails
pub fn create_hugr_qir_engine<P: AsRef<Path>>(
    hugr_path: P,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    let hugr_path = hugr_path.as_ref();
    info!("Creating QIR engine from HUGR: {}", hugr_path.display());

    // Create temporary directory for compilation output
    let temp_dir =
        TempDir::new().map_err(|e| PecosError::with_context(e, "Failed to create temp dir"))?;

    // Set up compiler configuration
    let output_path = temp_dir.path().join("compiled.ll");
    let config = HugrCompilerConfig {
        output_path: Some(output_path),
        debug_info: false,
        quantum_naming: super::compiler::QuantumNamingConvention::StandardQir,
    };

    // Compile HUGR to QIR
    let compiler = HugrCompiler::with_config(config);
    let qir_path = compiler.compile_hugr(hugr_path)?;

    info!("Compiled HUGR to QIR: {}", qir_path.display());

    // Create QIR engine from compiled output
    let mut qir_engine = QirEngine::new(qir_path);

    // Set shots if specified
    if let Some(num_shots) = shots {
        qir_engine.set_assigned_shots(num_shots);
    }

    // Pre-compile for efficiency
    qir_engine.pre_compile()?;

    Ok(Box::new(qir_engine))
}

/// Compile a HUGR file to QIR using default settings
///
/// # Arguments
/// * `hugr_path` - Path to the HUGR file
/// * `output_path` - Path where the QIR file should be written
///
/// # Returns
/// Path to the generated QIR file
///
/// # Errors
/// Returns `PecosError` if compilation fails
pub fn compile_hugr_to_qir<P: AsRef<Path>, Q: AsRef<Path>>(
    hugr_path: P,
    output_path: Q,
) -> Result<std::path::PathBuf, PecosError> {
    let config = HugrCompilerConfig {
        output_path: Some(output_path.as_ref().to_path_buf()),
        debug_info: false,
        quantum_naming: super::compiler::QuantumNamingConvention::StandardQir,
    };

    let compiler = HugrCompiler::with_config(config);
    compiler.compile_hugr(hugr_path)
}

/// Setup function for creating a HUGR-enabled QIR engine (alias for backwards compatibility)
///
/// # Arguments
/// * `hugr_path` - Path to the HUGR file
/// * `shots` - Optional number of shots
///
/// # Returns
/// A boxed `ClassicalEngine` instance
///
/// # Errors
/// Returns `PecosError` if:
/// - The HUGR file cannot be read
/// - Compilation fails
/// - Engine creation fails
pub fn setup_hugr_qir_engine<P: AsRef<Path>>(
    hugr_path: P,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    create_hugr_qir_engine(hugr_path, shots)
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    use super::compile_hugr_to_qir;
    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    use tempfile::NamedTempFile;

    #[test]
    fn test_hugr_engine_interface() {
        // This test ensures the module compiles and the functions exist
        // Actual testing would require valid HUGR files
    }

    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    #[test]
    fn test_hugr_compilation_without_feature() {
        let temp_file = NamedTempFile::new().unwrap();
        let output_file = NamedTempFile::new().unwrap();

        let result = compile_hugr_to_qir(temp_file.path(), output_file.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HUGR support not compiled")
        );
    }
}
