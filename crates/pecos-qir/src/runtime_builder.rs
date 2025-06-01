use log::info;
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
    /// Build the Rust QIR runtime as a static library
    ///
    /// This method ensures we have an up-to-date static library by leveraging
    /// Cargo's built-in incremental compilation and dependency tracking.
    pub fn build_runtime() -> Result<PathBuf, PecosError> {
        // Prevent concurrent builds
        let _lock = BUILD_MUTEX.lock().unwrap();

        let lib_path = Self::get_lib_path();
        let lib_dir = lib_path.parent().unwrap();
        Self::ensure_dir(lib_dir)?;

        // Build the wrapper crate
        let build_dir = lib_dir.join("build");
        if !build_dir.join("Cargo.toml").exists() {
            Self::create_wrapper_crate(&build_dir)?;
        }

        // Always run cargo build - it will use its own incremental compilation
        // and dependency tracking to decide if a rebuild is needed
        info!("Checking runtime library...");

        // Use a separate target directory to avoid conflicts
        let target_dir = lib_dir.join("target");

        let output = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--quiet",
                "--target-dir",
                target_dir.to_str().unwrap(),
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
        let built_lib = target_dir
            .join("release")
            .join(lib_path.file_name().unwrap());

        if !built_lib.exists() {
            return Err(PecosError::Processing(
                "Built library not found at expected location".to_string(),
            ));
        }

        // Copy to final location if different
        if built_lib != lib_path {
            // Check if we need to copy (compare contents or just copy if dest doesn't exist)
            let should_copy = if lib_path.exists() {
                // Compare file sizes as a quick check
                match (fs::metadata(&built_lib), fs::metadata(&lib_path)) {
                    (Ok(built_meta), Ok(lib_meta)) => built_meta.len() != lib_meta.len(),
                    _ => true, // If we can't compare, copy to be safe
                }
            } else {
                true // Destination doesn't exist, so copy
            };

            if should_copy {
                fs::copy(&built_lib, &lib_path)
                    .map_err(|e| PecosError::Processing(format!("Failed to copy library: {e}")))?;
            }
        }

        // Check if cargo actually rebuilt (by comparing timestamps)
        if let (Ok(built_meta), Ok(lib_meta)) = (fs::metadata(&built_lib), fs::metadata(&lib_path))
        {
            if let (Ok(built_time), Ok(lib_time)) = (built_meta.modified(), lib_meta.modified()) {
                if built_time == lib_time {
                    info!("Runtime library is up to date: {:?}", lib_path);
                } else {
                    info!("Runtime library rebuilt: {:?}", lib_path);
                }
            }
        } else {
            info!("Runtime library ready: {:?}", lib_path);
        }

        Ok(lib_path)
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

        fs::write(src_dir.join("lib.rs"), "pub use pecos_qir::*;\n")
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

    /// Ensure a directory exists, creating it if necessary
    fn ensure_dir(path: &Path) -> Result<(), PecosError> {
        if !path.exists() {
            fs::create_dir_all(path)
                .map_err(|e| PecosError::Processing(format!("Failed to create directory: {e}")))?;
        }
        Ok(())
    }
}
