/*!
HUGR-enabled LLVM Engine

This module provides functions to compile HUGR files to LLVM IR and create engines from them.
This is a simplified interface that doesn't try to implement complex traits.
*/

use super::compiler::{HugrCompiler, HugrCompilerConfig};
use crate::LlvmEngine;
use log::info;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::path::Path;
use tempfile::TempDir;

/// Compile a HUGR file to LLVM IR and create an LLVM engine from it
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
pub fn create_hugr_llvm_engine<P: AsRef<Path>>(
    hugr_path: P,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    let hugr_path = hugr_path.as_ref();
    info!("Creating LLVM engine from HUGR: {}", hugr_path.display());

    // Create temporary directory for compilation output
    let temp_dir =
        TempDir::new().map_err(|e| PecosError::with_context(e, "Failed to create temp dir"))?;

    // Set up compiler configuration - use native HUGR convention for direct execution
    let output_path = temp_dir.path().join("compiled.ll");
    let config = HugrCompilerConfig {
        output_path: Some(output_path),
        debug_info: false,
    };

    // Compile HUGR to LLVM-IR
    let compiler = HugrCompiler::with_config(config);
    let llvm_path = compiler.compile_hugr(hugr_path)?;

    info!("Compiled HUGR to native LLVM-IR: {}", llvm_path.display());

    // Create LLVM engine from compiled output
    let mut llvm_engine = LlvmEngine::new(llvm_path);

    // Set shots if specified
    if let Some(num_shots) = shots {
        llvm_engine.set_assigned_shots(num_shots);
    }

    // Pre-compile for efficiency
    llvm_engine.pre_compile()?;

    Ok(Box::new(llvm_engine))
}

/// Compile a HUGR file to native HUGR LLVM-IR using default settings
///
/// # Arguments
/// * `hugr_path` - Path to the HUGR file
/// * `output_path` - Path where the LLVM-IR file should be written
///
/// # Returns
/// Path to the generated LLVM-IR file
///
/// # Errors
/// Returns `PecosError` if compilation fails
pub fn compile_hugr_to_llvm<P: AsRef<Path>, Q: AsRef<Path>>(
    hugr_path: P,
    output_path: Q,
) -> Result<std::path::PathBuf, PecosError> {
    let config = HugrCompilerConfig {
        output_path: Some(output_path.as_ref().to_path_buf()),
        debug_info: false,
    };

    let compiler = HugrCompiler::with_config(config);
    compiler.compile_hugr(hugr_path)
}

/// Setup function for creating a HUGR-enabled LLVM engine (alias for backwards compatibility)
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
pub fn setup_hugr_llvm_engine<P: AsRef<Path>>(
    hugr_path: P,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    create_hugr_llvm_engine(hugr_path, shots)
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    use super::compile_hugr_to_llvm;
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

        let result = compile_hugr_to_llvm(temp_file.path(), output_file.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HUGR support not compiled")
        );
    }
}
