//! Example: Compile a QIR program using QisLinker
//!
//! This example demonstrates how to use the QisLinker to compile
//! a QIR (Quantum Intermediate Representation) program into a
//! dynamically loadable library.

use pecos_qis_interface::QisLinker;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    // Path to the QIR program
    let qir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("bell_state.ll");

    println!("QIR Program: {}", qir_path.display());

    // Create a linker
    let linker = QisLinker::new()
        .with_cache_dir("./target/qis-cache");

    println!("Compiling QIR program...");

    // Compile the QIR program
    match linker.compile(&qir_path, Some("bell_state_lib")) {
        Ok(lib_path) => {
            println!("SUCCESS: Successfully compiled to: {}", lib_path.display());
            println!("\nThe compiled library can be loaded and executed by:");
            println!("  1. A QisControlEngine with a QisRuntime");
            println!("  2. Direct FFI loading for testing");
        }
        Err(e) => {
            eprintln!("FAILED: Compilation failed: {}", e);
            eprintln!("\nMake sure you have LLVM tools installed:");
            eprintln!("  - On macOS: brew install llvm");
            eprintln!("  - On Ubuntu: apt-get install llvm");
            eprintln!("  - On Windows: Download from https://llvm.org/");
            std::process::exit(1);
        }
    }
}