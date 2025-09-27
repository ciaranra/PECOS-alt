/// Common platform compilation utilities
use pecos_core::errors::PecosError;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Common LLVM compilation flags
pub const COMMON_LLVM_FLAGS: &[&str] = &[
    "-O3",
    "-ffast-math",
    "-march=native",
];

/// Validate input and output paths for compilation
pub fn validate_compilation_paths(
    llvm_file: &Path,
    output_path: &Path,
) -> Result<(), PecosError> {
    // Validate LLVM file exists and is readable
    if !llvm_file.exists() {
        return Err(PecosError::Resource(format!(
            "LLVM IR file does not exist: {}",
            llvm_file.display()
        )));
    }

    let metadata = fs::metadata(llvm_file).map_err(PecosError::IO)?;
    if metadata.len() == 0 {
        return Err(PecosError::Resource(format!(
            "LLVM IR file is empty: {}",
            llvm_file.display()
        )));
    }

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(PecosError::IO)?;
        }
    }

    Ok(())
}

/// Execute a command with proper error handling
pub fn execute_command(
    command: &mut Command,
    operation: &str,
) -> Result<String, PecosError> {
    let output = command.output().map_err(|e| {
        PecosError::Compilation(format!("Failed to execute {}: {}", operation, e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        return Err(PecosError::Compilation(format!(
            "{} failed with exit code: {:?}\nstderr: {}\nstdout: {}",
            operation, output.status.code(), stderr, stdout
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Create a temporary file with proper cleanup on error
pub fn create_temp_file(
    dir: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<tempfile::NamedTempFile, PecosError> {
    tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile_in(dir)
        .map_err(|e| PecosError::Resource(format!("Failed to create temporary file: {}", e)))
}