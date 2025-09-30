//! Example: Execute a compiled QIR program
//!
//! This example demonstrates the complete flow:
//! 1. Compile a QIR program with QisLinker
//! 2. Load it dynamically
//! 3. Execute it and collect quantum operations

use pecos_qis_interface::{QisLinker, reset_interface, with_interface};
use std::path::PathBuf;

fn main() {
    env_logger::init();

    // Path to the QIR program
    let qir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("bell_state.ll");

    // Step 1: Compile the QIR program
    println!("Step 1: Compiling QIR program...");
    let linker = QisLinker::new()
        .with_cache_dir("./target/qis-cache");

    let lib_path = match linker.compile(&qir_path, Some("bell_state_lib")) {
        Ok(path) => {
            println!("  ✓ Compiled to: {}", path.display());
            path
        }
        Err(e) => {
            eprintln!("  ✗ Compilation failed: {}", e);
            std::process::exit(1);
        }
    };

    // Step 2: Load the compiled library
    println!("\nStep 2: Loading compiled library...");
    unsafe {
        let lib = match libloading::Library::new(&lib_path) {
            Ok(lib) => {
                println!("  ✓ Library loaded successfully");
                lib
            }
            Err(e) => {
                eprintln!("  ✗ Failed to load library: {}", e);
                std::process::exit(1);
            }
        };

        // Step 3: Execute the Bell state function
        println!("\nStep 3: Executing Bell state program...");

        // Reset the interface before execution
        reset_interface();

        // Get the bell_state function
        let bell_state_fn: libloading::Symbol<unsafe extern "C" fn()> =
            match lib.get(b"bell_state") {
                Ok(func) => {
                    println!("  ✓ Found bell_state function");
                    func
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to find bell_state function: {}", e);
                    std::process::exit(1);
                }
            };

        // Execute the function
        bell_state_fn();
        println!("  ✓ Executed bell_state function");

        // Step 4: Collect the operations
        println!("\nStep 4: Collected quantum operations:");
        with_interface(|interface| {
            for (i, op) in interface.operations.iter().enumerate() {
                println!("  [{}] {:?}", i, op);
            }

            println!("\nSummary:");
            println!("  - Total operations: {}", interface.operations.len());
            println!("  - Allocated qubits: {:?}", interface.allocated_qubits);
            println!("  - Allocated results: {:?}", interface.allocated_results);
        });
    }

    println!("\n✓ Example completed successfully!");
}