//! Runtime Builder Module
//!
//! This module handles building and managing the static pecos-qir runtime library
//! that LLVM IR programs link against.
//!
//! # Runtime Library Location
//!
//! The static library is stored at:
//! - Linux/macOS: `~/.cargo/pecos-qir/libpecos_qir.a`
//! - Windows: `~/.cargo/pecos-qir/pecos_qir.lib`
//!
//! # Rebuild Strategy
//!
//! The runtime library is rebuilt only when necessary:
//!
//! 1. **Missing Library**: If the library doesn't exist at the expected location
//! 2. **Marker File**: If `~/.cargo/pecos-qir/.needs_rebuild` exists
//!
//! The marker file is created by the build.rs script when it detects:
//! - Source files in pecos-qir have changed
//! - Dependencies have been updated
//! - The library is missing
//!
//! After a successful build, the marker file is removed to prevent unnecessary rebuilds.
//!
//! # Build Process
//!
//! The build process:
//! 1. Creates a minimal wrapper crate that depends on pecos-qir
//! 2. Builds it as a static library using cargo
//! 3. Copies the result to the expected location
//! 4. Removes the marker file
//!
//! This approach ensures the runtime library includes all necessary symbols
//! while avoiding circular dependencies during the build.

use log::{debug, info};
use pecos_core::errors::PecosError;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Handles building the static pecos-qir runtime library
pub struct RuntimeBuilder;

// Simple global mutex to prevent concurrent builds
static BUILD_MUTEX: Mutex<()> = Mutex::new(());

impl RuntimeBuilder {
    /// Build the Rust LLVM runtime as a static library
    ///
    /// This method ensures we have an up-to-date static library by checking
    /// for the existence of the library and a marker file that indicates
    /// a rebuild is needed.
    ///
    /// # Returns
    /// - `Ok(PathBuf)`: Path to the runtime library
    /// - `Err(PecosError)`: If building fails
    ///
    /// # Rebuild Conditions
    /// The library is rebuilt if:
    /// - The library file doesn't exist at `~/.cargo/pecos-qir/libpecos_qir.a`
    /// - The marker file `~/.cargo/pecos-qir/.needs_rebuild` exists
    ///
    /// The marker file is created by build.rs when source changes are detected.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The build lock cannot be acquired
    /// - Cargo commands fail
    /// - Library file is not found after build
    ///
    /// # Panics
    ///
    /// Panics if the build mutex is poisoned
    pub fn build_runtime() -> Result<PathBuf, PecosError> {
        // Prevent concurrent builds
        let _lock = BUILD_MUTEX.lock().unwrap();

        let lib_path = Self::get_lib_path();
        let marker_path = Self::get_marker_path();

        // Check if we need to build (library missing or marker exists)
        let needs_build = !lib_path.exists() || marker_path.exists();

        if needs_build {
            info!("Building runtime library...");
            Self::build_static_library(&lib_path)?;

            // Remove the marker file after successful build
            let _ = fs::remove_file(&marker_path);

            info!("Runtime library built: {:?}", lib_path);
        } else {
            debug!("Using existing runtime library: {:?}", lib_path);
        }

        Ok(lib_path)
    }

    /// Build the static library
    fn build_static_library(lib_path: &Path) -> Result<(), PecosError> {
        let lib_dir = lib_path.parent().unwrap();
        Self::ensure_dir(lib_dir)?;

        // Build the wrapper crate
        let build_dir = lib_dir.join("build");
        if !build_dir.join("Cargo.toml").exists() {
            Self::create_wrapper_crate(&build_dir)?;
        }

        // Use a separate target directory to avoid conflicts
        let target_dir = lib_dir.join("target");

        // Get cargo from environment or use default
        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let output = Command::new(&cargo)
            .args([
                "build",
                "--release",
                "--quiet",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
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
        let built_lib = target_dir
            .join("release")
            .join(lib_path.file_name().unwrap());

        if !built_lib.exists() {
            return Err(PecosError::Processing(
                "Built library not found at expected location".to_string(),
            ));
        }

        // Copy to final location
        fs::copy(&built_lib, lib_path)
            .map_err(|e| PecosError::Processing(format!("Failed to copy library: {e}")))?;

        // Touch the file to update its modification time
        // This is important because cargo's build cache might result in an older timestamp
        // We do this by appending and truncating to force a metadata update
        Self::touch_library_file(lib_path);

        Ok(())
    }

    /// Get the path to the library location
    fn get_lib_path() -> PathBuf {
        let base_dir = if let Ok(cargo_home) = env::var("CARGO_HOME") {
            PathBuf::from(cargo_home)
        } else if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(".cargo")
        } else if let Ok(userprofile) = env::var("USERPROFILE") {
            PathBuf::from(userprofile).join(".cargo")
        } else {
            PathBuf::from(".cargo")
        };

        let lib_name = if cfg!(target_os = "windows") {
            "pecos_qir.lib"
        } else {
            "libpecos_qir.a"
        };
        base_dir.join("pecos-qir").join(lib_name)
    }

    /// Get the path to the marker file
    fn get_marker_path() -> PathBuf {
        let base_dir = if let Ok(cargo_home) = env::var("CARGO_HOME") {
            PathBuf::from(cargo_home)
        } else if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(".cargo")
        } else if let Ok(userprofile) = env::var("USERPROFILE") {
            PathBuf::from(userprofile).join(".cargo")
        } else {
            PathBuf::from(".cargo")
        };

        base_dir.join("pecos-qir").join(".needs_rebuild")
    }

    /// Create the minimal wrapper crate for building the static library
    fn create_wrapper_crate(build_dir: &Path) -> Result<(), PecosError> {
        Self::ensure_dir(build_dir)?;

        // Get version and edition from workspace
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

        // src/lib.rs
        let src_dir = build_dir.join("src");
        Self::ensure_dir(&src_dir)?;

        // Create a minimal lib.rs that exports the instance-based runtime
        let lib_rs_content = r"// Re-export the instance-based runtime implementation
pub use pecos_qir::runtime::*;
pub use pecos_qir::runtime::state::*;
pub use pecos_qir::runtime::registry::*;
";

        fs::write(src_dir.join("lib.rs"), lib_rs_content)
            .map_err(|e| PecosError::Processing(format!("Failed to write lib.rs: {e}")))?;

        Ok(())
    }

    /// Get workspace version and edition with a simple approach
    fn get_workspace_metadata() -> Result<(String, String), PecosError> {
        // First, try to get from the workspace root Cargo.toml
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .ancestors()
            .find(|p| {
                p.join("Cargo.toml").exists() && {
                    // Check if this is the workspace root by looking for [workspace]
                    fs::read_to_string(p.join("Cargo.toml"))
                        .map(|content| content.contains("[workspace]"))
                        .unwrap_or(false)
                }
            })
            .ok_or_else(|| PecosError::Processing("Failed to find workspace root".to_string()))?;

        let toml_content = fs::read_to_string(workspace_root.join("Cargo.toml")).map_err(|e| {
            PecosError::Processing(format!("Failed to read workspace Cargo.toml: {e}"))
        })?;

        let mut version = "0.1.0".to_string();
        let mut edition = "2024".to_string();

        // Simple line-by-line parsing for [workspace.package] section
        let mut in_workspace_package = false;
        for line in toml_content.lines() {
            let line = line.trim();
            if line == "[workspace.package]" {
                in_workspace_package = true;
            } else if line.starts_with('[') {
                in_workspace_package = false;
            } else if in_workspace_package {
                if let Some(v) = line
                    .strip_prefix("version = \"")
                    .and_then(|s| s.strip_suffix('"'))
                {
                    version = v.to_string();
                } else if let Some(e) = line
                    .strip_prefix("edition = \"")
                    .and_then(|s| s.strip_suffix('"'))
                {
                    edition = e.to_string();
                }
            }
        }

        Ok((version, edition))
    }

    /// Touch a file to update its modification time
    fn touch_library_file(path: &Path) {
        use std::fs::OpenOptions;
        use std::io::Write;

        if let Ok(mut file) = OpenOptions::new().append(true).open(path) {
            // Get current size
            if let Ok(metadata) = file.metadata() {
                let original_size = metadata.len();
                // Write a byte to force timestamp update
                let _ = file.write_all(b"\0");
                let _ = file.sync_all();
                // Truncate back to original size
                let _ = file.set_len(original_size);
                debug!("Touched library file to update modification time");
            }
        }
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
