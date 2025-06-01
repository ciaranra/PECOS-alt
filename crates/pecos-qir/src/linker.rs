//! QIR Linker Module
//!
//! This module is responsible for compiling QIR programs (.ll files) and linking them
//! with the pre-built runtime library to create dynamically loadable libraries.
//!
//! The process involves:
//! 1. Compiling the QIR file to an object file using LLVM tools
//! 2. Getting the pre-built runtime library from `RuntimeBuilder`
//! 3. Linking them together to create a shared library (.so/.dll/.dylib)
//!
//! # Rebuild Strategy
//!
//! The `QirLinker` serves as the central coordination point for determining when
//! components need to be rebuilt. The strategy is:
//!
//! 1. **Runtime Library Check** - Always call `RuntimeBuilder::build_runtime()` first.
//!    This leverages Cargo's incremental compilation to efficiently detect if the
//!    runtime library needs rebuilding due to source or dependency changes.
//!
//! 2. **Cached QIR Library Check** - Check if a previously compiled QIR library exists
//!    and is still valid by comparing timestamps against:
//!    - The source QIR file (existing check)
//!    - The runtime library (to catch runtime updates)
//!
//! 3. **Rebuild Decision** - A rebuild occurs if:
//!    - The QIR library doesn't exist
//!    - The QIR source file is newer than the cached library
//!    - The runtime library is newer than the cached library
//!    - Any runtime dependencies changed (handled by Cargo)
//!
//! This approach ensures correctness while maintaining good performance through
//! caching and Cargo's incremental compilation.

#[cfg(target_os = "macos")]
use crate::platform::macos::MacOSCompiler;
#[cfg(target_os = "windows")]
use crate::platform::windows::WindowsCompiler;
use crate::platform::{executable_name, standard_llvm_paths};
use crate::runtime_builder::RuntimeBuilder;
use log::{debug, info, warn};
use pecos_core::errors::PecosError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Links QIR programs with the runtime library to create dynamically loadable libraries
pub struct QirLinker;

impl QirLinker {
    /// Compile and link a QIR program with the runtime to create a dynamically loadable library
    ///
    /// This method compiles a QIR file to an object file, then links it with the
    /// pre-built runtime library to create a shared library that can be loaded and executed.
    /// It uses caching to avoid recompiling unchanged files.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The QIR file does not exist or is empty
    /// - LLVM tools are not installed or are the wrong version
    /// - Compilation of the QIR file fails
    /// - Linking the object file with the runtime library fails
    /// - File system operations fail (creating directories, reading/writing files)
    pub fn compile<P: AsRef<Path>>(
        qir_file: P,
        output_dir: Option<P>,
    ) -> Result<PathBuf, PecosError> {
        let qir_file = qir_file.as_ref();
        // Validate the QIR file
        Self::validate_qir_file(qir_file)?;

        // Determine output directory
        let output_dir = Self::prepare_output_directory(qir_file, output_dir)?;

        // First check for cached compilation before building runtime
        // This avoids unnecessary runtime timestamp updates
        if let Some(cached_lib) = Self::find_cached_library(qir_file, &output_dir)? {
            // Now ensure the runtime library is up-to-date
            // This will use cargo's incremental compilation to detect if rebuild is needed
            let rust_runtime_lib = RuntimeBuilder::build_runtime()?;

            // Check if the cached library is still valid after runtime check
            let cached_metadata = fs::metadata(&cached_lib)?;
            let cached_mtime = cached_metadata.modified().map_err(PecosError::IO)?;

            let runtime_metadata = fs::metadata(&rust_runtime_lib)?;
            let runtime_mtime = runtime_metadata.modified().map_err(PecosError::IO)?;

            if cached_mtime >= runtime_mtime {
                info!("Using cached library: {:?}", cached_lib);
                return Ok(cached_lib);
            }
            info!("Cached library is older than runtime library, rebuilding...");
            // Fall through to rebuild
        } else {
            // No cached library, so ensure runtime is built
            RuntimeBuilder::build_runtime()?;
        }

        info!("Starting compilation: {:?}", qir_file);

        // Get the runtime library path again (it's already built)
        let rust_runtime_lib = RuntimeBuilder::build_runtime()?;

        // Generate file paths
        let (object_file, library_file) = Self::generate_file_paths(qir_file, &output_dir);

        // Compile QIR to object file
        Self::compile_to_object_file(qir_file, &object_file)?;

        // Link into a shared library
        Self::link_shared_library(&object_file, &rust_runtime_lib, &library_file)?;

        info!("Compilation successful: {:?}", library_file);

        Ok(library_file)
    }

    /// Find a cached library if it exists and is up-to-date
    fn find_cached_library(
        qir_file: &Path,
        output_dir: &Path,
    ) -> Result<Option<PathBuf>, PecosError> {
        let qir_metadata = fs::metadata(qir_file)?;
        let qir_modified = qir_metadata.modified().map_err(PecosError::IO)?;

        let file_stem = qir_file
            .file_stem()
            .unwrap_or_else(|| "qir_program".as_ref())
            .to_string_lossy();

        let lib_extension = Self::get_library_extension();
        let lib_prefix = format!("lib{file_stem}_");
        let lib_suffix = format!(".{lib_extension}");

        // Look for existing libraries matching the pattern
        let Ok(entries) = fs::read_dir(output_dir) else {
            return Ok(None);
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            if filename.starts_with(&lib_prefix) && filename.ends_with(&lib_suffix) {
                // Check if library is newer than QIR file
                if let Ok(lib_metadata) = fs::metadata(&path) {
                    if let Ok(lib_modified) = lib_metadata.modified() {
                        if lib_modified >= qir_modified {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Validate that the QIR file exists and is not empty
    fn validate_qir_file(qir_file: &Path) -> Result<(), PecosError> {
        let metadata = fs::metadata(qir_file).map_err(|_| {
            PecosError::Resource(format!("QIR file not found: {}", qir_file.display()))
        })?;

        if metadata.len() == 0 {
            return Err(PecosError::Resource(format!(
                "QIR file is empty: {}",
                qir_file.display()
            )));
        }

        Ok(())
    }

    /// Prepare the output directory
    fn prepare_output_directory<P: AsRef<Path>>(
        qir_file: &Path,
        output_dir: Option<P>,
    ) -> Result<PathBuf, PecosError> {
        let output_dir = output_dir.map_or_else(
            || {
                qir_file
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("build")
            },
            |d| d.as_ref().to_path_buf(),
        );

        Self::ensure_dir(&output_dir)?;
        Ok(output_dir)
    }

    /// Generate file paths for object file and library file
    fn generate_file_paths(qir_file: &Path, output_dir: &Path) -> (PathBuf, PathBuf) {
        let file_stem = qir_file
            .file_stem()
            .unwrap_or_else(|| "qir_program".as_ref())
            .to_string_lossy();

        // Generate unique library name with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let object_file = output_dir.join(format!("{file_stem}.o"));
        let library_file = output_dir.join(format!(
            "lib{file_stem}_{timestamp}.{}",
            Self::get_library_extension()
        ));

        (object_file, library_file)
    }

    /// Get the platform-specific library extension
    fn get_library_extension() -> &'static str {
        if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "dll"
        }
    }

    /// Find an LLVM tool in the system
    pub(crate) fn find_llvm_tool(tool_name: &str) -> Option<PathBuf> {
        let exec_name = executable_name(tool_name);

        // Check environment variables first
        for env_var in ["PECOS_LLVM_PATH", "LLVM_HOME"] {
            if let Ok(path) = std::env::var(env_var) {
                let tool_path = PathBuf::from(path).join("bin").join(&exec_name);
                if tool_path.exists() {
                    debug!("Found {} from {}: {:?}", tool_name, env_var, tool_path);
                    return Some(tool_path);
                }
            }
        }

        // Check PATH using which/where
        let command = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };

        if let Ok(output) = Command::new(command).arg(tool_name).output() {
            if output.status.success() {
                if let Ok(path_str) = String::from_utf8(output.stdout) {
                    if let Some(first_line) = path_str.lines().next() {
                        let path = PathBuf::from(first_line.trim());
                        if path.exists() {
                            debug!("Found {} from PATH: {:?}", tool_name, path);
                            return Some(path);
                        }
                    }
                }
            }
        }

        // Check standard locations
        for base_path in standard_llvm_paths() {
            let tool_path = base_path.join(&exec_name);
            if tool_path.exists() {
                debug!("Found {} at: {:?}", tool_name, tool_path);
                return Some(tool_path);
            }
        }

        None
    }

    /// Check LLVM version (requires LLVM 14.x)
    pub(crate) fn check_llvm_version(tool_path: &Path) -> Result<String, String> {
        let output = Command::new(tool_path)
            .arg("--version")
            .output()
            .map_err(|e| format!("Version check failed: {e}"))?;

        if !output.status.success() {
            return Err("Version check failed".to_string());
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version = Self::extract_version(&version_output)?;

        // Check major version
        let major = version
            .split('.')
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or("Invalid version format")?;

        if major != 14 {
            return Err(format!("LLVM {version} not supported. Requires LLVM 14.x"));
        }

        Ok(version.to_string())
    }

    /// Extract version number from version output
    fn extract_version(output: &str) -> Result<&str, &'static str> {
        output
            .lines()
            .next()
            .ok_or("Empty version output")?
            .split_whitespace()
            .find(|s| {
                s.chars().any(|c| c.is_ascii_digit())
                    && (s.contains('.') || s.parse::<u32>().is_ok())
            })
            .ok_or("No version found")
    }

    /// Compile QIR file to object file using LLVM tools
    fn compile_to_object_file(qir_file: &Path, object_file: &Path) -> Result<(), PecosError> {
        debug!("Compiling: {:?} -> {:?}", qir_file, object_file);

        // Ensure the output directory exists
        if let Some(parent) = object_file.parent() {
            Self::ensure_dir(parent)?;
        }

        #[cfg(target_os = "windows")]
        {
            let clang = Self::find_llvm_tool("clang").ok_or_else(|| {
                PecosError::Processing(
                    "clang not found. Install LLVM 14 and add to PATH.".to_string(),
                )
            })?;

            // Verify LLVM version
            Self::check_llvm_version(&clang).map_err(PecosError::Processing)?;

            debug!("Using clang: {:?}", clang);

            WindowsCompiler::compile_to_object_file(
                qir_file,
                object_file,
                &clang,
                Self::handle_command_error,
                Self::handle_command_status,
            )
        }
        #[cfg(not(target_os = "windows"))]
        {
            let llc_path = Self::find_llvm_tool("llc").ok_or_else(|| {
                PecosError::Processing("llc not found. Install LLVM 14 (e.g., 'apt install llvm-14' or 'brew install llvm@14').".to_string())
            })?;

            // Verify LLVM version
            Self::check_llvm_version(&llc_path).map_err(PecosError::Processing)?;

            let result = Command::new(llc_path)
                .args(["-filetype=obj", "-o"])
                .arg(object_file)
                .arg(qir_file)
                .output();

            let output = Self::handle_command_error(result, "Failed to run llc")?;
            Self::handle_command_status(&output, "llc")?;

            debug!("Successfully compiled QIR to object file");
            Ok(())
        }
    }

    /// Link object file and runtime library into a shared library
    fn link_shared_library(
        object_file: &Path,
        rust_runtime_lib: &Path,
        library_file: &Path,
    ) -> Result<(), PecosError> {
        debug!("Linking object file and runtime library...");

        // Ensure the output directory exists
        if let Some(parent) = library_file.parent() {
            Self::ensure_dir(parent)?;
        }

        // Verify input files exist
        for (file, desc) in [
            (object_file, "Object file"),
            (rust_runtime_lib, "Runtime library"),
        ] {
            if !file.exists() {
                return Err(PecosError::Processing(format!(
                    "{desc} not found: {}",
                    file.display()
                )));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let clang = Self::find_llvm_tool("clang").ok_or_else(|| {
                PecosError::Processing(
                    "clang not found in system. Please install LLVM tools.".to_string(),
                )
            })?;

            WindowsCompiler::link_shared_library(
                object_file,
                rust_runtime_lib,
                library_file,
                &clang,
                Self::handle_command_error,
                Self::handle_command_status,
            )
        }
        #[cfg(target_os = "macos")]
        {
            MacOSCompiler::link_shared_library(
                object_file,
                rust_runtime_lib,
                library_file,
                Self::handle_command_error,
                Self::handle_command_status,
            )
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let result = Command::new("gcc")
                .args(["-shared", "-o"])
                .arg(library_file)
                .arg(object_file)
                .arg(rust_runtime_lib)
                .output();

            let output = Self::handle_command_error(result, "Failed to execute gcc")?;
            Self::handle_command_status(&output, "gcc")?;

            debug!("Linked: {:?}", library_file);
            Ok(())
        }
    }

    /// Helper function to handle command execution errors
    fn handle_command_error<T>(
        result: std::io::Result<T>,
        error_msg: &str,
    ) -> Result<T, PecosError> {
        result.map_err(|e| {
            warn!("{}: {}", error_msg, e);
            PecosError::Processing(format!("QIR compilation failed: {error_msg}: {e}"))
        })
    }

    /// Helper function to handle command execution status
    fn handle_command_status(
        output: &std::process::Output,
        command_name: &str,
    ) -> Result<(), PecosError> {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error = PecosError::Processing(format!(
                "QIR compilation failed: {command_name} failed with status: {} and error: {stderr}",
                output.status
            ));
            warn!("{}", error);
            return Err(error);
        }
        Ok(())
    }

    /// Ensure a directory exists, creating it if necessary
    fn ensure_dir(path: &Path) -> Result<(), PecosError> {
        if !path.exists() {
            fs::create_dir_all(path)
                .map_err(|e| PecosError::Processing(format!("Failed to create directory: {e}")))?;
        }
        Ok(())
    }
}
