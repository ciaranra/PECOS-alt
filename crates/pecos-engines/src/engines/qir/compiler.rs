use crate::engines::qir::common::get_thread_id;
use crate::engines::qir::error::QirError;
use crate::errors::QueueError;
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compiles a QIR program to a dynamically loadable library
///
/// This struct provides methods for compiling QIR (Quantum Intermediate Representation)
/// programs into dynamically loadable libraries that can be executed by the QIR engine.
///
/// # Compilation Process
///
/// 1. The QIR file is compiled to an object file using LLVM's `llc` tool
/// 2. A Rust static library with the QIR runtime implementation is built
/// 3. The object file and runtime library are linked into a shared library using `clang`
///
/// # Thread Safety
///
/// The compiler is designed to be thread-safe, with each compilation creating
/// unique output files to avoid conflicts between threads.
pub struct QirCompiler;

impl QirCompiler {
    /// Helper function to log an error and convert it to `QueueError`
    fn log_error<E: Into<QirError>>(error: E, thread_id: &str) -> QueueError {
        let error = error.into();
        warn!("QIR Compiler: [Thread {}] {}", thread_id, error);
        error.into()
    }

    /// Helper function to handle command execution errors
    fn handle_command_error<T>(
        result: std::io::Result<T>,
        error_msg: &str,
        thread_id: &str,
    ) -> Result<T, QueueError> {
        result.map_err(|e| {
            let error_msg = format!("{error_msg}: {e}");
            Self::log_error(QirError::CompilationFailed(error_msg), thread_id)
        })
    }

    /// Helper function to handle command execution status
    fn handle_command_status(
        output: &std::process::Output,
        command_name: &str,
        thread_id: &str,
    ) -> Result<(), QueueError> {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_msg = format!(
                "{command_name} failed with status: {} and error: {stderr}",
                output.status
            );
            return Err(Self::log_error(
                QirError::CompilationFailed(error_msg),
                thread_id,
            ));
        }
        Ok(())
    }

    /// Compile a QIR program to a dynamically loadable library
    ///
    /// This method compiles a QIR (Quantum Intermediate Representation) file into a
    /// dynamically loadable library that can be executed by the QIR engine.
    ///
    /// # Arguments
    ///
    /// * `qir_file` - Path to the QIR file to compile
    /// * `output_dir` - Optional output directory for the compiled library
    ///
    /// # Returns
    ///
    /// * `Result<PathBuf, QueueError>` - Path to the compiled library if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `QirError::FileNotFound` - If the QIR file does not exist
    /// * `QirError::EmptyFile` - If the QIR file is empty
    /// * `QirError::FileReadError` - If the QIR file cannot be read
    /// * `QirError::CompilationFailed` - If the compilation process fails
    /// * `QirError::TempDirCreationFailed` - If the temporary directory cannot be created
    ///
    /// # Compilation Process
    ///
    /// 1. The QIR file is validated (exists, not empty)
    /// 2. The output directory is created if it doesn't exist
    /// 3. The QIR file is compiled to an object file using LLVM's `llc` tool
    /// 4. A Rust static library with the QIR runtime implementation is built
    /// 5. The object file and runtime library are linked into a shared library using `clang`
    #[allow(clippy::too_many_lines)]
    pub fn compile<P: AsRef<Path>>(
        qir_file: P,
        output_dir: Option<P>,
    ) -> Result<PathBuf, QueueError> {
        let qir_file = qir_file.as_ref();
        let thread_id = get_thread_id();

        info!(
            "QIR Compiler: [Thread {}] Starting compilation of QIR file: {:?}",
            thread_id, qir_file
        );

        // Validate the QIR file
        Self::validate_qir_file(qir_file, &thread_id)?;

        // Determine and create output directory
        let output_dir = Self::prepare_output_directory(qir_file, output_dir, &thread_id)?;

        // Generate file paths
        let (object_file, library_file) =
            Self::generate_file_paths(qir_file, &output_dir, &thread_id);

        // Compile QIR to object file
        Self::compile_to_object_file(qir_file, &object_file, &thread_id)?;

        // Get the QIR runtime library
        let rust_runtime_lib = Self::build_rust_runtime(&output_dir).map_err(|e| {
            warn!(
                "QIR Compiler: [Thread {}] Failed to build Rust runtime: {}",
                thread_id, e
            );
            e
        })?;

        // Link object file and runtime library into a shared library
        Self::link_shared_library(&object_file, &rust_runtime_lib, &library_file, &thread_id)?;

        info!(
            "QIR Compiler: [Thread {}] Successfully compiled QIR file to library: {:?}",
            thread_id, library_file
        );

        Ok(library_file)
    }

    /// Validate that the QIR file exists and is not empty
    fn validate_qir_file(qir_file: &Path, thread_id: &str) -> Result<(), QueueError> {
        // Check if the file exists
        if !qir_file.exists() {
            return Err(Self::log_error(
                QirError::FileNotFound(qir_file.to_path_buf()),
                thread_id,
            ));
        }

        // Check if the file is empty
        let metadata = fs::metadata(qir_file).map_err(|e| {
            Self::log_error(
                QirError::FileReadError {
                    path: qir_file.to_path_buf(),
                    error: e,
                },
                thread_id,
            )
        })?;

        if metadata.len() == 0 {
            return Err(Self::log_error(
                QirError::EmptyFile(qir_file.to_path_buf()),
                thread_id,
            ));
        }

        debug!(
            "QIR Compiler: [Thread {}] QIR file validation successful: {:?} ({} bytes)",
            thread_id,
            qir_file,
            metadata.len()
        );

        Ok(())
    }

    /// Prepare the output directory
    fn prepare_output_directory<P: AsRef<Path>>(
        qir_file: &Path,
        output_dir: Option<P>,
        thread_id: &str,
    ) -> Result<PathBuf, QueueError> {
        // Determine output directory
        let output_dir = if let Some(dir) = output_dir {
            dir.as_ref().to_path_buf()
        } else {
            let parent_dir = qir_file.parent().unwrap_or_else(|| Path::new("."));
            parent_dir.join("build")
        };

        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            debug!(
                "QIR Compiler: [Thread {}] Creating output directory: {:?}",
                thread_id, output_dir
            );
            fs::create_dir_all(&output_dir)
                .map_err(|e| Self::log_error(QirError::TempDirCreationFailed(e), thread_id))?;
        }

        Ok(output_dir)
    }

    /// Generate file paths for object file and library file
    fn generate_file_paths(
        qir_file: &Path,
        output_dir: &Path,
        thread_id: &str,
    ) -> (PathBuf, PathBuf) {
        // Get file name without extension
        let file_stem = qir_file
            .file_stem()
            .unwrap_or_else(|| "qir_program".as_ref());
        let file_stem_str = file_stem.to_string_lossy();

        // Generate unique library name with timestamp to avoid conflicts
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let lib_name = format!("{file_stem_str}_{timestamp}");

        // Determine file paths
        let object_file = output_dir.join(format!("{file_stem_str}.o"));

        // Determine library extension based on platform
        #[cfg(target_os = "linux")]
        let lib_extension = "so";
        #[cfg(target_os = "macos")]
        let lib_extension = "dylib";
        #[cfg(target_os = "windows")]
        let lib_extension = "dll";

        let library_file = output_dir.join(format!("lib{lib_name}.{lib_extension}"));

        debug!("QIR Compiler: [Thread {}] Compilation paths:", thread_id);
        debug!(
            "QIR Compiler: [Thread {}]   Input file: {:?}",
            thread_id, qir_file
        );
        debug!(
            "QIR Compiler: [Thread {}]   Object file: {:?}",
            thread_id, object_file
        );
        debug!(
            "QIR Compiler: [Thread {}]   Library file: {:?}",
            thread_id, library_file
        );

        (object_file, library_file)
    }

    /// Compile QIR file to object file using LLVM's llc tool
    fn compile_to_object_file(
        qir_file: &Path,
        object_file: &Path,
        thread_id: &str,
    ) -> Result<(), QueueError> {
        debug!(
            "QIR Compiler: [Thread {}] Compiling QIR to object file using llc...",
            thread_id
        );

        let llc_output = Self::handle_command_error(
            Command::new("llc")
                .arg("-filetype=obj")
                .arg("-o")
                .arg(object_file)
                .arg(qir_file)
                .output(),
            "Failed to execute llc",
            thread_id,
        )?;

        Self::handle_command_status(&llc_output, "llc", thread_id)?;

        debug!(
            "QIR Compiler: [Thread {}] Successfully compiled QIR to object file",
            thread_id
        );

        Ok(())
    }

    /// Link object file and runtime library into a shared library using clang
    fn link_shared_library(
        object_file: &Path,
        rust_runtime_lib: &Path,
        library_file: &Path,
        thread_id: &str,
    ) -> Result<(), QueueError> {
        debug!(
            "QIR Compiler: [Thread {}] Linking object file and runtime library using clang...",
            thread_id
        );

        let clang_output = Self::handle_command_error(
            Command::new("clang")
                .arg("-shared")
                .arg("-o")
                .arg(library_file)
                .arg(object_file)
                .arg(rust_runtime_lib)
                .output(),
            "Failed to execute clang",
            thread_id,
        )?;

        Self::handle_command_status(&clang_output, "clang", thread_id)?;

        debug!(
            "QIR Compiler: [Thread {}] Successfully linked shared library",
            thread_id
        );

        Ok(())
    }

    /// Find the pre-built QIR runtime library
    ///
    /// This function looks for the pre-built QIR runtime library in the target directory.
    /// It checks both debug and release directories.
    ///
    /// # Returns
    ///
    /// * `Option<(PathBuf, usize)>` - Path to the library and its size if found, None otherwise
    fn find_prebuilt_library(thread_id: &str) -> Option<(PathBuf, u64)> {
        // Check for pre-built runtime library in target directory
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();

        // Determine library name based on platform
        #[cfg(target_os = "windows")]
        let lib_filename = "qir_runtime.lib";
        #[cfg(not(target_os = "windows"))]
        let lib_filename = "libqir_runtime.a";

        // Check both debug and release directories
        let debug_lib_path = workspace_dir.join(format!("target/debug/{lib_filename}"));
        let release_lib_path = workspace_dir.join(format!("target/release/{lib_filename}"));

        // Check debug directory first
        if debug_lib_path.exists() {
            let size = fs::metadata(&debug_lib_path).map(|m| m.len()).unwrap_or(0);
            debug!(
                "QIR Compiler: [Thread {}] Found pre-built library in debug directory: {:?} ({} bytes)",
                thread_id, debug_lib_path, size
            );
            return Some((debug_lib_path, size));
        }

        // Then check release directory
        if release_lib_path.exists() {
            let size = fs::metadata(&release_lib_path)
                .map(|m| m.len())
                .unwrap_or(0);
            debug!(
                "QIR Compiler: [Thread {}] Found pre-built library in release directory: {:?} ({} bytes)",
                thread_id, release_lib_path, size
            );
            return Some((release_lib_path, size));
        }

        None
    }

    /// Build the Rust QIR runtime as a static library
    ///
    /// This method finds the pre-built QIR runtime library in the target directory:
    /// - `target/debug/libqir_runtime.a` (or `qir_runtime.lib` on Windows)
    /// - `target/release/libqir_runtime.a` (or `qir_runtime.lib` on Windows)
    ///
    /// The pre-built library is automatically generated by the `build.rs` script
    /// in the pecos-engines crate.
    ///
    /// If the pre-built library is not found, this method will attempt to build it
    /// by running `cargo build -p pecos-engines` before raising an error.
    ///
    /// See `QIR_RUNTIME.md` for more details on the QIR runtime library build process.
    ///
    /// # Arguments
    ///
    /// * `output_dir` - Directory where the runtime library should be built (unused)
    ///
    /// # Returns
    ///
    /// * `Result<PathBuf, QueueError>` - Path to the pre-built static library if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `QirError::CompilationFailed` - If the pre-built library cannot be found or built
    fn build_rust_runtime(_output_dir: &Path) -> Result<PathBuf, QueueError> {
        let thread_id = get_thread_id();
        debug!(
            "QIR Compiler: [Thread {}] Looking for pre-built QIR runtime library",
            thread_id
        );

        // Try to find the pre-built library
        if let Some((lib_path, size)) = Self::find_prebuilt_library(&thread_id) {
            info!(
                "QIR Compiler: [Thread {}] Using pre-built QIR runtime library from: {:?} ({} bytes)",
                thread_id, lib_path, size
            );
            return Ok(lib_path);
        }

        // If no pre-built library is found, attempt to build it
        warn!(
            "QIR Compiler: [Thread {}] No pre-built QIR runtime library found. Attempting to build it...",
            thread_id
        );

        // Get workspace directory for running cargo
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();

        // Run cargo build to trigger the build.rs script
        debug!(
            "QIR Compiler: [Thread {}] Running 'cargo build -p pecos-engines'...",
            thread_id
        );

        let cargo_output = Self::handle_command_error(
            Command::new("cargo")
                .arg("build")
                .arg("-p")
                .arg("pecos-engines")
                .current_dir(workspace_dir)
                .output(),
            "Failed to execute cargo",
            &thread_id,
        )?;

        Self::handle_command_status(&cargo_output, "cargo", &thread_id)?;

        // Check if the library was built
        if let Some((lib_path, size)) = Self::find_prebuilt_library(&thread_id) {
            info!(
                "QIR Compiler: [Thread {}] Successfully built QIR runtime library: {:?} ({} bytes)",
                thread_id, lib_path, size
            );
            return Ok(lib_path);
        }

        // If still not found, return an error
        let error_msg = "Failed to find or build QIR runtime library. The library should be automatically built by the build.rs script. See QIR_RUNTIME.md for more details.".to_string();
        Err(Self::log_error(
            QirError::CompilationFailed(error_msg.clone()),
            &error_msg,
        ))
    }
}
