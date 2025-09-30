//! Example: Compile a QIR program with the interface library
//!
//! This example shows how to compile and link a QIR program
//! with the pecos-qis-interface static library.

use pecos_qis_interface::QisLinker;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    // Paths
    let qir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("bell_state.ll");

    let interface_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()  // Go up to workspace root
        .join("target/debug/libpecos_qis_interface.a");

    println!("QIR Program: {}", qir_path.display());
    println!("Interface Library: {}", interface_lib.display());

    // Create a linker with the interface library
    let linker = QisLinker::new()
        .with_cache_dir("./target/qis-cache")
        .with_interface_lib(&interface_lib);

    println!("\nCompiling QIR program with interface library...");

    // Compile the QIR program
    match linker.compile(&qir_path, Some("bell_state_linked")) {
        Ok(lib_path) => {
            println!("✓ Successfully compiled to: {}", lib_path.display());
            println!("\nThis library now contains:");
            println!("  - Your QIR program (bell_state)");
            println!("  - QIS interface FFI functions");
            println!("  - Ready to be loaded and executed!");
        }
        Err(e) => {
            eprintln!("✗ Compilation failed: {}", e);
            std::process::exit(1);
        }
    }
}