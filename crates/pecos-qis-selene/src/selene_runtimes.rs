//! Utility functions for Selene runtime plugins
//!
//! This module provides convenient access to Selene runtime implementations.
//! The runtimes are automatically built when you build this crate if the
//! Selene repository is found at ../selene (relative to PECOS).

use crate::SeleneRuntime;
use std::path::PathBuf;

/// Error type for runtime fetching
#[derive(Debug)]
pub enum RuntimeFetchError {
    IoError(std::io::Error),
    DownloadError(String),
    InvalidPath(String),
}

impl std::fmt::Display for RuntimeFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::DownloadError(msg) => write!(f, "Download error: {msg}"),
            Self::InvalidPath(msg) => write!(f, "Invalid path: {msg}"),
        }
    }
}

impl std::error::Error for RuntimeFetchError {}

impl From<std::io::Error> for RuntimeFetchError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// Create a Selene Simple Runtime
///
/// This loads the Selene Simple runtime plugin that was built by the build script.
/// The runtime is expected to be at `../selene/target/release/libselene_simple_runtime.so`
/// (relative to the PECOS workspace).
///
/// # Example
/// ```rust
/// use pecos_qis_selene::{selene_simple_runtime};
/// use pecos_qis_core::{qis_engine, QisEngine};
/// use pecos_engines::ClassicalControlEngineBuilder;
/// use pecos_qis_ffi_types::OperationCollector;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load the simple runtime (built during compilation)
/// match selene_simple_runtime() {
///     Ok(runtime) => {
///         let interface = OperationCollector::new();
///         let engine = qis_engine().runtime(runtime).program(interface).build()?;
///         // Engine is ready to use
///     }
///     Err(e) => {
///         // Runtime not built - Selene repository not found
///         eprintln!("Simple runtime not available: {}", e);
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the Selene simple runtime library cannot be found.
pub fn selene_simple_runtime() -> Result<SeleneRuntime, RuntimeFetchError> {
    eprintln!("[selene_simple_runtime] Called");
    let runtime_path = find_built_selene_runtime("selene_simple_runtime")?;
    eprintln!("[selene_simple_runtime] Found runtime at: {:?}", runtime_path);
    let runtime = SeleneRuntime::new(runtime_path);
    eprintln!("[selene_simple_runtime] Created SeleneRuntime, returning");
    Ok(runtime)
}

/// Create a Selene Soft RZ Runtime
///
/// This runtime implements soft RZ gates for more accurate gate modeling.
/// The runtime is expected to be at `../selene/target/release/libselene_soft_rz_runtime.so`
/// (relative to the PECOS workspace).
///
/// # Example
/// ```rust
/// use pecos_qis_selene::{selene_soft_rz_runtime};
/// use pecos_qis_core::{qis_engine, QisEngine};
/// use pecos_engines::ClassicalControlEngineBuilder;
/// use pecos_qis_ffi_types::OperationCollector;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load the soft RZ runtime (built during compilation)
/// match selene_soft_rz_runtime() {
///     Ok(runtime) => {
///         let interface = OperationCollector::new();
///         let engine = qis_engine().runtime(runtime).program(interface).build()?;
///         // Engine is ready with soft RZ gate support
///     }
///     Err(e) => {
///         // Runtime not built - Selene repository not found
///         eprintln!("Soft RZ runtime not available: {}", e);
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the Selene soft RZ runtime library cannot be found.
pub fn selene_soft_rz_runtime() -> Result<SeleneRuntime, RuntimeFetchError> {
    let runtime_path = find_built_selene_runtime("selene_soft_rz_runtime")?;
    Ok(SeleneRuntime::new(runtime_path))
}

// Note: We only expose convenience functions for actual Selene runtime plugins.
// Other Selene plugins (error models, simulators, compilers) can still be loaded
// using find_selene_runtime() or selene_runtime() with an explicit path.

/// Find a Selene runtime that was built by the build script
///
/// This uses the same logic as the build script to locate the Selene repository
/// and find the built runtime .so file.
fn find_built_selene_runtime(lib_name: &str) -> Result<PathBuf, RuntimeFetchError> {
    // Check the same paths as the build script
    let possible_selene_paths = [
        PathBuf::from("../../../selene"), // From crate directory
        PathBuf::from("../selene"),       // From workspace root
    ];

    let selene_path = possible_selene_paths
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            RuntimeFetchError::InvalidPath(
                "Selene repository not found. Expected at ../selene or ../../../selene".to_string(),
            )
        })?;

    // The runtime should be in selene/target/release/lib{name}.so
    let runtime_path = selene_path
        .join("target/release")
        .join(format!("lib{lib_name}.so"));

    if !runtime_path.exists() {
        return Err(RuntimeFetchError::InvalidPath(format!(
            "Selene runtime {} not found at {}. Run 'cargo build --release' in Selene repository to build it.",
            lib_name,
            runtime_path.display()
        )));
    }

    log::info!("Found built Selene runtime: {}", runtime_path.display());
    Ok(runtime_path)
}

/// Try to find a Selene runtime in common locations
///
/// Searches in order:
/// 1. `PECOS_SELENE_DIR` environment variable
/// 2. Selene repository target directory (../selene/target/release)
/// 3. Current target/release or target/debug
/// 4. Workspace target directory
/// 5. System library paths
#[must_use]
pub fn find_selene_runtime(name: &str) -> Option<PathBuf> {
    let filename = format!("libselene_{name}.so");

    // Check environment variable
    if let Ok(selene_dir) = std::env::var("PECOS_SELENE_DIR") {
        let path = PathBuf::from(selene_dir).join(&filename);
        if path.exists() {
            return Some(path);
        }
    }

    // Check Selene repository (adjacent to PECOS)
    for profile in &["release", "debug"] {
        // Try from workspace root: ../selene/target/{profile}
        let selene_target = PathBuf::from("../selene/target")
            .join(profile)
            .join(&filename);
        if selene_target.exists() {
            return Some(selene_target);
        }

        // Also try from crate directory: ../../../selene/target/{profile}
        let selene_from_crate = PathBuf::from("../../../selene/target")
            .join(profile)
            .join(&filename);
        if selene_from_crate.exists() {
            return Some(selene_from_crate);
        }

        // Also check for compiler in selene-compilers
        if name == "hugr_qis_compiler" {
            let compiler_path = PathBuf::from("../selene/selene-compilers/hugr_qis/target")
                .join(profile)
                .join(&filename);
            if compiler_path.exists() {
                return Some(compiler_path);
            }

            // From crate directory
            let compiler_from_crate =
                PathBuf::from("../../../selene/selene-compilers/hugr_qis/target")
                    .join(profile)
                    .join(&filename);
            if compiler_from_crate.exists() {
                return Some(compiler_from_crate);
            }
        }
    }

    // Check target directories in current project
    for profile in &["release", "debug"] {
        let path = PathBuf::from("target").join(profile).join(&filename);
        if path.exists() {
            return Some(path);
        }

        // Check parent directories (in case we're in a workspace member)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let workspace_target = PathBuf::from(manifest_dir)
                .parent()?
                .parent()? // Go up to workspace root
                .join("target")
                .join(profile)
                .join(&filename);
            if workspace_target.exists() {
                return Some(workspace_target);
            }
        }
    }

    // Check system paths
    for sys_path in &["/usr/local/lib", "/usr/lib", "/opt/pecos/lib"] {
        let path = PathBuf::from(sys_path).join(&filename);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Create a Selene runtime automatically
///
/// This loads a runtime that was built by the build script. The name should be
/// the library name (e.g., "`selene_simple_runtime`", "`selene_soft_rz_runtime`").
///
/// # Example
/// ```rust
/// use pecos_qis_selene::{selene_runtime_auto};
/// use pecos_qis_core::{qis_engine, QisEngine};
/// use pecos_engines::ClassicalControlEngineBuilder;
/// use pecos_qis_ffi_types::OperationCollector;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load a runtime by name (built during compilation)
/// match selene_runtime_auto("selene_simple_runtime") {
///     Ok(runtime) => {
///         let interface = OperationCollector::new();
///         let engine = qis_engine().runtime(runtime).program(interface).build()?;
///         // Engine is ready with the runtime
///     }
///     Err(e) => {
///         // Runtime not built - Selene repository not found
///         eprintln!("Could not load runtime: {}", e);
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the specified Selene runtime library cannot be found.
pub fn selene_runtime_auto(lib_name: &str) -> Result<SeleneRuntime, RuntimeFetchError> {
    let runtime_path = find_built_selene_runtime(lib_name)?;
    Ok(SeleneRuntime::new(runtime_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_selene_runtime() {
        // This might not find anything in test environment
        let result = find_selene_runtime("simple");
        // Just verify it doesn't panic
        if let Some(path) = result {
            assert!(path.to_string_lossy().contains("selene_simple"));
        }
    }
}
