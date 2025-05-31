use log::{debug, info};
use pecos_core::errors::PecosError;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Handles building the static pecos-qir runtime library
pub struct RuntimeBuilder;

impl RuntimeBuilder {
    /// Get the path to the persistent library location
    pub fn get_persistent_lib_path() -> PathBuf {
        let cargo_home = env::var("CARGO_HOME")
            .ok()
            .or_else(|| env::var("HOME").ok())
            .or_else(|| env::var("USERPROFILE").ok())
            .map_or_else(|| PathBuf::from(".cargo"), PathBuf::from);

        let lib_name = if cfg!(target_os = "windows") {
            "pecos_qir.lib"
        } else {
            "libpecos_qir.a"
        };
        cargo_home.join("pecos-qir").join(lib_name)
    }

    /// Build the Rust QIR runtime as a static library
    ///
    /// This method ensures we have an up-to-date static library by:
    /// 1. Checking if the library exists in the persistent location
    /// 2. If not, building it using Cargo (which uses fingerprinting for efficiency)
    pub fn build_runtime() -> Result<PathBuf, PecosError> {
        let persistent_lib_path = Self::get_persistent_lib_path();

        // If library exists, Cargo's fingerprinting will handle updates
        if persistent_lib_path.exists() {
            debug!("Found existing runtime library: {:?}", persistent_lib_path);
            return Ok(persistent_lib_path);
        }

        // Build the library
        info!("Building static runtime library");
        Self::build_static_library()
    }

    /// Build the static library on demand
    fn build_static_library() -> Result<PathBuf, PecosError> {
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
        let Ok(_lock) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_file)
        else {
            // Wait for other build to complete
            for _ in 0..300 {
                if persistent_lib_path.exists() {
                    info!("Library built by another thread");
                    return Ok(persistent_lib_path);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            return Err(PecosError::Processing(
                "Timeout waiting for library build".to_string(),
            ));
        };

        let build_start = Instant::now();
        info!("Building to persistent location");

        // We need a minimal wrapper to build the static library
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
        let built_lib_path = target_dir
            .join("release")
            .join(persistent_lib_path.file_name().unwrap());

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

        let build_duration = build_start.elapsed();
        info!(
            "Built runtime library: {:?} ({:.2}s)",
            persistent_lib_path,
            build_duration.as_secs_f64()
        );

        Ok(persistent_lib_path)
    }

    /// Create the minimal wrapper crate for building the static library
    fn create_wrapper_crate(build_dir: &Path) -> Result<(), PecosError> {
        Self::ensure_dir(build_dir)?;

        // Get version and edition from workspace
        let (version, edition) = Self::get_workspace_metadata()?;

        // Cargo.toml
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
