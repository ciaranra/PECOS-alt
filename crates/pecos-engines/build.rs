use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build script for the pecos-engines crate
///
/// This script automatically builds the QIR runtime library that is used by the QIR compiler.
/// The library is built only when necessary (when source files have changed) and is placed
/// in both the debug and release target directories.
///
/// The QIR runtime library is used by the QIR compiler to implement quantum operations
/// when compiling QIR programs. By pre-building this library, we significantly speed up
/// the compilation process for QIR programs, especially during testing.
///
/// The script:
/// 1. Checks if the QIR runtime library needs to be rebuilt
/// 2. If needed, creates a temporary Rust project with the necessary files
/// 3. Builds the library in release mode
/// 4. Copies the built library to both debug and release target directories
///
/// See `QIR_RUNTIME.md` for more details on the QIR runtime library.
#[allow(clippy::too_many_lines)]
fn main() {
    // Tell Cargo to rerun this script if any of these files change
    println!("cargo:rerun-if-changed=src/engines/qir/runtime.rs");
    println!("cargo:rerun-if-changed=src/engines/qir/common.rs");
    println!("cargo:rerun-if-changed=src/engines/qir/state.rs");
    println!("cargo:rerun-if-changed=src/result_id.rs");
    println!("cargo:rerun-if-changed=src/byte_message/quantum_cmd.rs");

    // Build the QIR runtime library
    if let Err(e) = build_qir_runtime() {
        eprintln!("Warning: Failed to build QIR runtime library: {e}");
        eprintln!("QIR compilation will be slower as it will build the runtime on-demand.");
    }
}

#[allow(clippy::too_many_lines)]
fn build_qir_runtime() -> Result<(), String> {
    println!("Building QIR runtime library...");

    // Get the workspace root directory
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();

    // Determine library name based on platform
    let lib_filename = if cfg!(windows) {
        "qir_runtime.lib"
    } else {
        "libqir_runtime.a"
    };

    // Check if the library already exists and is up-to-date
    let debug_lib_path = workspace_dir.join(format!("target/debug/{lib_filename}"));
    let release_lib_path = workspace_dir.join(format!("target/release/{lib_filename}"));

    // Check if we need to rebuild based on file modification times
    let should_rebuild = needs_rebuild(&manifest_dir, &debug_lib_path)
        || needs_rebuild(&manifest_dir, &release_lib_path);

    if !should_rebuild {
        println!("QIR runtime library is up-to-date, skipping build.");
        return Ok(());
    }

    // Create a temporary directory for building
    let build_dir = workspace_dir.join("target/qir_runtime_build");
    fs::create_dir_all(&build_dir).map_err(|e| format!("Failed to create build directory: {e}"))?;
    fs::create_dir_all(build_dir.join("src"))
        .map_err(|e| format!("Failed to create src directory: {e}"))?;
    fs::create_dir_all(build_dir.join("src/byte_message"))
        .map_err(|e| format!("Failed to create byte_message directory: {e}"))?;

    // Create Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "qir_runtime"
version = "0.1.0"
edition = "2021"

[lib]
name = "qir_runtime"
crate-type = ["staticlib"]

[dependencies]
once_cell = "1.8.0"
pecos-core = {{ path = "{}" }}

[workspace]
"#,
        workspace_dir.join("crates/pecos-core").display()
    );

    fs::write(build_dir.join("Cargo.toml"), cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

    // Copy necessary files
    fs::copy(
        manifest_dir.join("src/engines/qir/common.rs"),
        build_dir.join("src/common.rs"),
    )
    .map_err(|e| format!("Failed to copy common.rs: {e}"))?;

    // Copy and modify state.rs
    let state_content = fs::read_to_string(manifest_dir.join("src/engines/qir/state.rs"))
        .map_err(|e| format!("Failed to read state.rs: {e}"))?;
    let modified_state =
        state_content.replace("use crate::engines::qir::common::", "use crate::common::");
    fs::write(build_dir.join("src/state.rs"), modified_state)
        .map_err(|e| format!("Failed to write state.rs: {e}"))?;

    // Copy result_id.rs
    fs::copy(
        manifest_dir.join("src/result_id.rs"),
        build_dir.join("src/result_id.rs"),
    )
    .map_err(|e| format!("Failed to copy result_id.rs: {e}"))?;

    // Copy quantum_cmd.rs
    fs::copy(
        manifest_dir.join("src/byte_message/quantum_cmd.rs"),
        build_dir.join("src/byte_message/quantum_cmd.rs"),
    )
    .map_err(|e| format!("Failed to copy quantum_cmd.rs: {e}"))?;

    // Create byte_message.rs
    let byte_message_content = r"pub mod quantum_cmd;
pub use quantum_cmd::QuantumCmd;
";
    fs::write(build_dir.join("src/byte_message.rs"), byte_message_content)
        .map_err(|e| format!("Failed to write byte_message.rs: {e}"))?;

    // Read and modify runtime.rs
    let runtime_content = fs::read_to_string(manifest_dir.join("src/engines/qir/runtime.rs"))
        .map_err(|e| format!("Failed to read runtime.rs: {e}"))?;
    let modified_runtime = runtime_content
        .replace("use crate::engines::qir::common::", "use crate::common::")
        .replace("use crate::engines::qir::state::", "use crate::state::")
        .replace(
            "use crate::byte_message::quantum_cmd::",
            "use crate::byte_message::",
        );

    // Add module declarations to the top of the file
    let module_declarations = r"pub mod byte_message;
pub mod result_id;
pub mod common;
pub mod state;

";

    // Write the modified runtime.rs as lib.rs with module declarations
    fs::write(
        build_dir.join("src/lib.rs"),
        format!("{module_declarations}{modified_runtime}"),
    )
    .map_err(|e| format!("Failed to write lib.rs: {e}"))?;

    // Build the library
    println!("Running cargo build in {build_dir:?}...");
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&build_dir)
        .status()
        .map_err(|e| format!("Failed to execute cargo: {e}"))?;

    if !status.success() {
        return Err(format!("Cargo build failed with status: {status}"));
    }

    // Create target directories if they don't exist
    fs::create_dir_all(workspace_dir.join("target/debug"))
        .map_err(|e| format!("Failed to create debug directory: {e}"))?;
    fs::create_dir_all(workspace_dir.join("target/release"))
        .map_err(|e| format!("Failed to create release directory: {e}"))?;

    // Copy the built library to the target directories
    let built_lib_path = build_dir.join(format!("target/release/{lib_filename}"));
    fs::copy(
        &built_lib_path,
        workspace_dir.join(format!("target/debug/{lib_filename}")),
    )
    .map_err(|e| format!("Failed to copy library to debug directory: {e}"))?;
    fs::copy(
        &built_lib_path,
        workspace_dir.join(format!("target/release/{lib_filename}")),
    )
    .map_err(|e| format!("Failed to copy library to release directory: {e}"))?;

    println!("QIR runtime library built successfully!");
    Ok(())
}

fn needs_rebuild(manifest_dir: &Path, lib_path: &Path) -> bool {
    // If the library doesn't exist, we need to build it
    if !lib_path.exists() {
        return true;
    }

    // Get the modification time of the library
    let Ok(lib_modified) = fs::metadata(lib_path).and_then(|m| m.modified()) else {
        // If we can't get the modification time, rebuild to be safe
        return true;
    };

    // Check if any source files are newer than the library
    let source_files = [
        "src/engines/qir/runtime.rs",
        "src/engines/qir/common.rs",
        "src/engines/qir/state.rs",
        "src/result_id.rs",
        "src/byte_message/quantum_cmd.rs",
    ];

    for file in &source_files {
        let file_path = manifest_dir.join(file);
        if let Ok(metadata) = fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                if modified > lib_modified {
                    println!("Source file {file_path:?} is newer than library, rebuilding");
                    return true;
                }
            }
        }
    }

    false
}
