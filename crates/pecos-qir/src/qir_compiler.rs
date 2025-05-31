use crate::common::get_thread_id;
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

/// Compiles QIR programs to dynamically loadable libraries
pub struct QirCompiler;

impl QirCompiler {
    /// Compile a QIR program to a dynamically loadable library
    ///
    /// This method compiles a QIR file into a shared library that can be loaded and executed.
    /// It uses caching to avoid recompiling unchanged files.
    pub fn compile<P: AsRef<Path>>(
        qir_file: P,
        output_dir: Option<P>,
    ) -> Result<PathBuf, PecosError> {
        let qir_file = qir_file.as_ref();
        let thread_id = get_thread_id();

        // Validate the QIR file
        Self::validate_qir_file(qir_file)?;

        // Determine output directory
        let output_dir = Self::prepare_output_directory(qir_file, output_dir)?;

        // Check for cached compilation
        if let Some(cached_lib) = Self::find_cached_library(qir_file, &output_dir)? {
            info!(
                "[Thread {}] Using cached library: {:?}",
                thread_id, cached_lib
            );
            return Ok(cached_lib);
        }

        info!(
            "[Thread {}] Starting compilation: {:?}",
            thread_id, qir_file
        );

        // Generate file paths
        let (object_file, library_file) = Self::generate_file_paths(qir_file, &output_dir);

        // Compile QIR to object file
        Self::compile_to_object_file(qir_file, &object_file, &thread_id)?;

        // Get the runtime library
        let rust_runtime_lib = RuntimeBuilder::build_runtime()?;

        // Link into a shared library
        Self::link_shared_library(&object_file, &rust_runtime_lib, &library_file, &thread_id)?;

        info!(
            "[Thread {}] Compilation successful: {:?}",
            thread_id, library_file
        );

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
        let cached_lib = fs::read_dir(output_dir).ok().and_then(|entries| {
            entries.filter_map(Result::ok).find_map(|entry| {
                let path = entry.path();
                let filename = path.file_name()?.to_str()?;

                if filename.starts_with(&lib_prefix) && filename.ends_with(&lib_suffix) {
                    // Check if library is newer than QIR file
                    let lib_metadata = fs::metadata(&path).ok()?;
                    let lib_modified = lib_metadata.modified().ok()?;

                    (lib_modified >= qir_modified).then_some(path)
                } else {
                    None
                }
            })
        });

        Ok(cached_lib)
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
    fn compile_to_object_file(
        qir_file: &Path,
        object_file: &Path,
        thread_id: &str,
    ) -> Result<(), PecosError> {
        debug!(
            "[Thread {}] Compiling: {:?} -> {:?}",
            thread_id, qir_file, object_file
        );

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

            debug!("[Thread {}] Using clang: {:?}", thread_id, clang);

            WindowsCompiler::compile_to_object_file(
                qir_file,
                object_file,
                &clang,
                thread_id,
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

            let output = Self::handle_command_error(result, "Failed to run llc", thread_id)?;
            Self::handle_command_status(&output, "llc", thread_id)?;

            debug!(
                "[Thread {}] Successfully compiled QIR to object file",
                thread_id
            );
            Ok(())
        }
    }

    /// Link object file and runtime library into a shared library
    fn link_shared_library(
        object_file: &Path,
        rust_runtime_lib: &Path,
        library_file: &Path,
        thread_id: &str,
    ) -> Result<(), PecosError> {
        debug!(
            "[Thread {}] Linking object file and runtime library...",
            thread_id
        );

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
                thread_id,
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
                thread_id,
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

            let output = Self::handle_command_error(result, "Failed to execute gcc", thread_id)?;
            Self::handle_command_status(&output, "gcc", thread_id)?;

            debug!("[Thread {}] Linked: {:?}", thread_id, library_file);
            Ok(())
        }
    }

    /// Helper function to handle command execution errors
    fn handle_command_error<T>(
        result: std::io::Result<T>,
        error_msg: &str,
        thread_id: &str,
    ) -> Result<T, PecosError> {
        result.map_err(|e| {
            warn!("[Thread {}] {}: {}", thread_id, error_msg, e);
            PecosError::Processing(format!("QIR compilation failed: {error_msg}: {e}"))
        })
    }

    /// Helper function to handle command execution status
    fn handle_command_status(
        output: &std::process::Output,
        command_name: &str,
        thread_id: &str,
    ) -> Result<(), PecosError> {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error = PecosError::Processing(format!(
                "QIR compilation failed: {command_name} failed with status: {} and error: {stderr}",
                output.status
            ));
            warn!("[Thread {}] {}", thread_id, error);
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
