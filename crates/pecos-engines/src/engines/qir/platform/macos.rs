//! macOS-specific implementations for QIR compilation

use crate::engines::qir::error::QirError;
use crate::errors::QueueError;
use log::{debug, warn};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Handle macOS-specific QIR compilation
pub struct MacOSCompiler;

impl MacOSCompiler {
    /// Log an error with thread ID
    pub fn log_error<E: Into<QirError>>(error: E, thread_id: &str) -> QueueError {
        let error = error.into();
        warn!("QIR Compiler: [Thread {}] {}", thread_id, error);
        error.into()
    }

    /// Get standard LLVM installation paths for macOS
    pub fn standard_llvm_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/opt/llvm/bin"),
        ]
    }

    /// Get executable name for macOS
    pub fn executable_name(tool_name: &str) -> String {
        tool_name.to_string()
    }

    /// Link object file and runtime library into a shared library on macOS
    ///
    /// This method uses `-dynamiclib` instead of `-shared` as required by macOS linker
    pub fn link_shared_library(
        object_file: &Path,
        rust_runtime_lib: &Path,
        library_file: &Path,
        thread_id: &str,
        handle_command_error: impl Fn(
            std::io::Result<std::process::Output>,
            &str,
            &str,
        ) -> Result<std::process::Output, QueueError>,
        handle_command_status: impl Fn(&std::process::Output, &str, &str) -> Result<(), QueueError>,
    ) -> Result<(), QueueError> {
        debug!(
            "QIR Compiler: [Thread {}] Linking with macOS-specific logic",
            thread_id
        );

        // Use clang instead of ld directly on macOS as it handles the linking better
        let clang = Command::new("clang")
            .args(["-dynamiclib", "-o"]) // Use -dynamiclib instead of -shared
            .arg(library_file)
            .arg(object_file)
            .arg(rust_runtime_lib)
            .output();

        let output = handle_command_error(clang, "Failed to execute clang for linking", thread_id)?;
        handle_command_status(&output, "clang", thread_id)?;

        debug!(
            "QIR Compiler: [Thread {}] Successfully linked shared library on macOS: {:?}",
            thread_id, library_file
        );

        Ok(())
    }
}
