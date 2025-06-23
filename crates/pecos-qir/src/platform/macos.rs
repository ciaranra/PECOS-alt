//! macOS-specific implementations for QIR compilation

use log::debug;
use pecos_core::errors::PecosError;
use std::path::Path;
use std::process::Command;

/// Handle macOS-specific QIR compilation
pub struct MacOSCompiler;

impl MacOSCompiler {
    /// Link object file and runtime library into a shared library on macOS
    ///
    /// This method uses `-dynamiclib` instead of `-shared` as required by macOS linker
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `clang` command cannot be executed (e.g., clang is not installed or not in PATH)
    /// - The `clang` command fails to link the object file and runtime library
    /// - The provided `handle_command_error` closure returns an error
    /// - The provided `handle_command_status` closure returns an error (e.g., non-zero exit status)
    pub fn link_shared_library(
        object_file: &Path,
        rust_runtime_lib: &Path,
        library_file: &Path,
        handle_command_error: impl Fn(
            std::io::Result<std::process::Output>,
            &str,
        ) -> Result<std::process::Output, PecosError>,
        handle_command_status: impl Fn(&std::process::Output, &str) -> Result<(), PecosError>,
    ) -> Result<(), PecosError> {
        debug!("QIR Compiler: Linking with macOS-specific logic");

        // Use clang instead of ld directly on macOS as it handles the linking better
        let clang = Command::new("clang")
            .args(["-dynamiclib", "-o"]) // Use -dynamiclib instead of -shared
            .arg(library_file)
            .arg(object_file)
            .arg(rust_runtime_lib)
            .output();

        let output = handle_command_error(clang, "Failed to execute clang for linking")?;
        handle_command_status(&output, "clang")?;

        debug!(
            "QIR Compiler: Successfully linked shared library on macOS: {:?}",
            library_file
        );

        Ok(())
    }
}
