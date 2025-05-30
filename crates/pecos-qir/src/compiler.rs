use crate::common::get_thread_id;
#[cfg(target_os = "macos")]
use crate::platform::macos::MacOSCompiler;
#[cfg(target_os = "windows")]
use crate::platform::windows::WindowsCompiler;
use crate::platform::{executable_name, standard_llvm_paths};
use log::{debug, info, warn};
use pecos_core::errors::PecosError;
use std::env;
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
    /// Helper function to log an error and return it
    fn log_error(error: PecosError, thread_id: &str) -> PecosError {
        warn!("[Thread {}] {}", thread_id, error);
        error
    }

    /// Helper function to handle command execution errors
    fn handle_command_error<T>(
        result: std::io::Result<T>,
        error_msg: &str,
        thread_id: &str,
    ) -> Result<T, PecosError> {
        result.map_err(|e| {
            Self::log_error(
                PecosError::Processing(format!("{error_msg}: {e}")),
                thread_id,
            )
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
            return Err(Self::log_error(
                PecosError::Processing(format!(
                    "{command_name} failed: {}", 
                    stderr.trim()
                )),
                thread_id,
            ));
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
    /// * `Result<PathBuf, PecosError>` - Path to the compiled library if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::ResourceError` - If the QIR file does not exist or is empty
    /// * `PecosError::IO` - If the QIR file cannot be read
    /// * `PecosError::CompilationError` - If the compilation process fails
    /// * `PecosError::IO` - If the temporary directory cannot be created
    ///
    /// # Compilation Process
    ///
    /// 1. The QIR file is validated (exists, not empty)
    /// 2. The output directory is created if it doesn't exist
    /// 3. The QIR file is compiled to an object file using LLVM's `llc` tool
    /// 4. A Rust static library with the QIR runtime implementation is built
    /// 5. The object file and runtime library are linked into a shared library using `clang`
    pub fn compile<P: AsRef<Path>>(
        qir_file: P,
        output_dir: Option<P>,
    ) -> Result<PathBuf, PecosError> {
        let qir_file = qir_file.as_ref();
        let thread_id = get_thread_id();

        info!("[Thread {}] Starting compilation: {:?}", thread_id, qir_file);

        // Validate the QIR file
        Self::validate_qir_file(qir_file, &thread_id)?;

        // Determine and create output directory
        let output_dir = Self::prepare_output_directory(qir_file, output_dir, &thread_id)?;

        // Generate file paths
        let (object_file, library_file) = Self::generate_file_paths(qir_file, &output_dir);

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

        info!("[Thread {}] Compilation successful: {:?}", thread_id, library_file);

        Ok(library_file)
    }

    /// Validate that the QIR file exists and is not empty
    fn validate_qir_file(qir_file: &Path, thread_id: &str) -> Result<(), PecosError> {
        let metadata = fs::metadata(qir_file).map_err(|_| {
            Self::log_error(
                PecosError::Resource(format!("QIR file not found: {}", qir_file.display())),
                thread_id,
            )
        })?;

        if metadata.len() == 0 {
            return Err(Self::log_error(
                PecosError::Resource(format!("QIR file is empty: {}", qir_file.display())),
                thread_id,
            ));
        }

        debug!("[Thread {}] Validated: {:?} ({} bytes)", thread_id, qir_file, metadata.len());

        Ok(())
    }

    /// Prepare the output directory
    fn prepare_output_directory<P: AsRef<Path>>(
        qir_file: &Path,
        output_dir: Option<P>,
        thread_id: &str,
    ) -> Result<PathBuf, PecosError> {
        let output_dir = output_dir
            .map(|d| d.as_ref().to_path_buf())
            .unwrap_or_else(|| {
                qir_file.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("build")
            });

        if !output_dir.exists() {
            debug!("[Thread {}] Creating directory: {:?}", thread_id, output_dir);
            Self::ensure_dir(&output_dir)?;
        }

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
        
        #[cfg(target_os = "linux")]
        let lib_ext = "so";
        #[cfg(target_os = "macos")]
        let lib_ext = "dylib";
        #[cfg(target_os = "windows")]
        let lib_ext = "dll";

        let library_file = output_dir.join(format!("lib{file_stem}_{timestamp}.{lib_ext}"));

        (object_file, library_file)
    }

    /// Find an LLVM tool in the system
    fn find_llvm_tool(tool_name: &str) -> Option<PathBuf> {
        let exec_name = executable_name(tool_name);

        // Check environment variables
        for env_var in ["PECOS_LLVM_PATH", "LLVM_HOME"] {
            if let Ok(path) = env::var(env_var) {
                let tool_path = PathBuf::from(path).join("bin").join(&exec_name);
                if tool_path.exists() {
                    debug!("Found {} from {}: {:?}", tool_name, env_var, tool_path);
                    return Some(tool_path);
                }
            }
        }

        // Check PATH
        const WHICH_CMD: &str = if cfg!(target_os = "windows") { "where" } else { "which" };
        
        if let Ok(output) = Command::new(WHICH_CMD).arg(tool_name).output() {
            if output.status.success() {
                if let Some(first_line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                    let path = PathBuf::from(first_line.trim());
                    if path.exists() {
                        debug!("Found {} from PATH: {:?}", tool_name, path);
                        return Some(path);
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
    fn check_llvm_version(tool_path: &Path) -> Result<String, String> {
        let output = Command::new(tool_path)
            .arg("--version")
            .output()
            .map_err(|e| format!("Version check failed: {e}"))?;

        if !output.status.success() {
            return Err("Version check failed".to_string());
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version_line = version_output
            .lines()
            .next()
            .ok_or("Empty version output")?;

        // Extract version number (e.g., "14.0.0")
        let version = version_line
            .split_whitespace()
            .find(|s| s.chars().any(|c| c.is_ascii_digit()) && (s.contains('.') || s.parse::<u32>().is_ok()))
            .ok_or("No version found")?;

        // Check major version
        let major: u32 = version.split('.').next()
            .and_then(|v| v.parse().ok())
            .ok_or("Invalid version format")?;

        if major != 14 {
            return Err(format!("LLVM {version} not supported. Requires LLVM 14.x"));
        }

        Ok(version.to_string())
    }

    /// Compile QIR file to object file using LLVM tools
    ///
    /// On Windows, this uses clang directly with the dllexport attribute added to the main function.
    /// On other platforms, it uses llc to compile the QIR to an object file.
    fn compile_to_object_file(
        qir_file: &Path,
        object_file: &Path,
        thread_id: &str,
    ) -> Result<(), PecosError> {
        debug!("[Thread {}] Compiling: {:?} -> {:?}", thread_id, qir_file, object_file);

        // Ensure the output directory exists
        if let Some(parent) = object_file.parent() {
            Self::ensure_dir(parent)?;
        }

        #[cfg(target_os = "windows")]
        {
            // Try to find clang first - always needed for linking on Windows
            let clang = Self::find_llvm_tool("clang").ok_or_else(|| {
                Self::log_error(
                    PecosError::Processing(
                        "clang not found. Install LLVM 14 and add to PATH.".to_string()
                    ),
                    thread_id,
                )
            })?;

            // Verify LLVM version
            let version_result = Self::check_llvm_version(&clang);
            if let Err(version_err) = version_result {
                return Err(Self::log_error(
                    PecosError::Processing(version_err),
                    thread_id,
                ));
            }

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
                Self::log_error(
                    PecosError::Processing(
                        "llc not found. Install LLVM 14 (e.g., 'apt install llvm-14' or 'brew install llvm@14').".to_string()
                    ),
                    thread_id,
                )
            })?;

            // Verify LLVM version
            let version_result = Self::check_llvm_version(&llc_path);
            if let Err(version_err) = version_result {
                return Err(Self::log_error(
                    PecosError::Processing(version_err),
                    thread_id,
                ));
            }

            let result = Command::new(llc_path)
                .args(["-filetype=obj", "-o"])
                .arg(object_file)
                .arg(qir_file)
                .output();

            let output = Self::handle_command_error(result, "Failed to run llc", thread_id)?;
            Self::handle_command_status(&output, "llc", thread_id)?;

            debug!("[Thread {}] Object file created", thread_id);

            Ok(())
        }
    }

    /// Link object file and runtime library into a shared library
    ///
    /// On Windows, this creates a DEF file to explicitly export all QIR runtime functions,
    /// then uses clang with the LLD linker to create a DLL.
    /// On Linux, it uses gcc to create a shared object.
    /// On macOS, it uses clang with -dynamiclib to create a dynamic library.
    fn link_shared_library(
        object_file: &Path,
        rust_runtime_lib: &Path,
        library_file: &Path,
        thread_id: &str,
    ) -> Result<(), PecosError> {
        debug!("[Thread {}] Linking libraries...", thread_id);

        // Ensure the output directory exists
        if let Some(parent) = library_file.parent() {
            Self::ensure_dir(parent)?;
        }

        // Verify input files exist
        if !object_file.exists() {
            return Err(Self::log_error(
                PecosError::Processing(format!("Object file not found: {}", object_file.display())),
                thread_id,
            ));
        }
        if !rust_runtime_lib.exists() {
            return Err(Self::log_error(
                PecosError::Processing(format!("Runtime library not found: {}", rust_runtime_lib.display())),
                thread_id,
            ));
        }

        #[cfg(target_os = "windows")]
        {
            let clang = Self::find_llvm_tool("clang").ok_or_else(|| {
                Self::log_error(
                    PecosError::Processing("clang not found. Install LLVM tools.".to_string()),
                    thread_id,
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
    
    /// Get the path to the persistent library location
    fn get_persistent_lib_path() -> PathBuf {
        let cargo_home = env::var("CARGO_HOME")
            .ok()
            .or_else(|| env::var("HOME").ok())
            .or_else(|| env::var("USERPROFILE").ok())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".cargo"));
        
        let lib_name = if cfg!(target_os = "windows") { "pecos_qir.lib" } else { "libpecos_qir.a" };
        cargo_home.join("pecos-qir").join(lib_name)
    }

    /// Build the Rust QIR runtime as a static library
    ///
    /// This method ensures we have an up-to-date static library by:
    /// 1. Checking if the library exists in the persistent location
    /// 2. If not, building it using Cargo (which uses fingerprinting for efficiency)
    ///
    /// # Arguments
    ///
    /// * `output_dir` - Directory where the runtime library should be built (unused)
    ///
    /// # Returns
    ///
    /// * `Result<PathBuf, PecosError>` - Path to the static library if successful
    fn build_rust_runtime(_output_dir: &Path) -> Result<PathBuf, PecosError> {
        let thread_id = get_thread_id();
        let persistent_lib_path = Self::get_persistent_lib_path();
        
        // If library exists, Cargo's fingerprinting will handle updates
        if persistent_lib_path.exists() {
            debug!("[Thread {}] Found existing library: {:?}", thread_id, persistent_lib_path);
            return Ok(persistent_lib_path);
        }
        
        // Build the library
        info!("[Thread {}] Building static library", thread_id);
        
        Self::build_static_library(&thread_id)
    }
    
    /// Build the static library on demand
    fn build_static_library(thread_id: &str) -> Result<PathBuf, PecosError> {
        use std::fs::OpenOptions;
        use std::time::Instant;
        
        let persistent_lib_path = Self::get_persistent_lib_path();
        let persistent_dir = persistent_lib_path.parent().unwrap();
        
        // Early return if library exists
        if persistent_lib_path.exists() {
            return Ok(persistent_lib_path);
        }
        
        // Create persistent directory
        Self::ensure_dir(persistent_dir)?;
        
        // Atomic lock file creation
        let lock_file = persistent_dir.join(".building.lock");
        let _lock = match OpenOptions::new().write(true).create_new(true).open(&lock_file) {
            Ok(file) => file,
            Err(_) => {
                // Wait for other build to complete
                for _ in 0..300 {
                    if persistent_lib_path.exists() {
                        info!("[Thread {}] Library built by another thread", thread_id);
                        return Ok(persistent_lib_path);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                return Err(PecosError::Processing("Timeout waiting for library build".to_string()));
            }
        };
        
        let build_start = Instant::now();
        info!("[Thread {}] Building to persistent location", thread_id);
        
        // We need a minimal wrapper to build the static library
        // This is because cargo rustc doesn't work well during tests
        let build_dir = persistent_dir.join("build");
        
        // Create wrapper crate if it doesn't exist
        if !build_dir.join("Cargo.toml").exists() {
            Self::create_wrapper_crate(&build_dir)?;
        }
        
        // Use a separate target directory to avoid conflicts
        let target_dir = persistent_dir.join("target");
        
        let output = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--quiet",
                "--target-dir", target_dir.to_str().unwrap(),
            ])
            .env("CARGO_INCREMENTAL", "1")
            .current_dir(&build_dir)
            .output()
            .map_err(|e| PecosError::Processing(format!("Failed to run cargo: {e}")))?;
        
        if !output.status.success() {
            return Err(PecosError::Processing(format!(
                "Failed to build static library: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        
        // The library will be in target/release/libpecos_qir.a (or .lib on Windows)
        let built_lib_path = target_dir.join("release").join(persistent_lib_path.file_name().unwrap());
        
        if !built_lib_path.exists() {
            return Err(PecosError::Processing(format!(
                "Built static library not found at expected location: {}",
                built_lib_path.display()
            )));
        }
        
        // Hard link if possible, otherwise copy
        if built_lib_path != persistent_lib_path {
            // Try hard link first (most efficient)
            if fs::hard_link(&built_lib_path, &persistent_lib_path).is_err() {
                // Fall back to copy
                fs::copy(&built_lib_path, &persistent_lib_path)
                    .map_err(|e| PecosError::Processing(format!("Failed to copy library: {e}")))?;
            }
        }
        
        // Clean up the lock file
        let _ = fs::remove_file(&lock_file);
        
        // No need for timestamp file - Cargo's fingerprinting handles this
        
        let build_duration = build_start.elapsed();
        info!("[Thread {}] Built: {:?} ({:.2}s)", thread_id, persistent_lib_path, build_duration.as_secs_f64());
        
        Ok(persistent_lib_path)
    }
    
    /// Create the minimal wrapper crate for building the static library
    fn create_wrapper_crate(build_dir: &Path) -> Result<(), PecosError> {
        Self::ensure_dir(build_dir)?;
        
        let (version, edition) = Self::get_workspace_metadata()?;
        
        let cargo_toml = format!(
            r#"[package]
name = "pecos-qir-static"
version = "{version}"
edition = "{edition}"

[lib]
name = "pecos_qir"
crate-type = ["staticlib"]

[dependencies]
pecos-qir = {{ path = {:?} }}
"#,
            env!("CARGO_MANIFEST_DIR")
        );
        
        fs::write(build_dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| PecosError::Processing(format!("Failed to write Cargo.toml: {e}")))?;
        
        let src_dir = build_dir.join("src");
        Self::ensure_dir(&src_dir)?;
        fs::write(src_dir.join("lib.rs"), "pub use pecos_qir::*;\n")
            .map_err(|e| PecosError::Processing(format!("Failed to write lib.rs: {e}")))?;
        
        Ok(())
    }
    
    /// Extract version and edition from workspace Cargo.toml
    fn get_workspace_metadata() -> Result<(String, String), PecosError> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .ancestors()
            .nth(2)
            .ok_or_else(|| PecosError::Processing("Failed to find workspace root".to_string()))?;
        
        let toml = fs::read_to_string(workspace_root.join("Cargo.toml"))
            .map_err(|e| PecosError::Processing(format!("Failed to read Cargo.toml: {e}")))?;
        
        let mut version = "0.1.0".to_string();
        let mut edition = "2021".to_string();
        
        // Simple parser for [workspace.package] section
        let mut in_workspace = false;
        for line in toml.lines() {
            let line = line.trim();
            if line == "[workspace.package]" {
                in_workspace = true;
            } else if line.starts_with('[') {
                in_workspace = false;
            } else if in_workspace {
                if let Some(v) = line.strip_prefix("version = \"").and_then(|s| s.strip_suffix('"')) {
                    version = v.to_string();
                } else if let Some(e) = line.strip_prefix("edition = \"").and_then(|s| s.strip_suffix('"')) {
                    edition = e.to_string();
                }
            }
        }
        
        Ok((version, edition))
    }
}
