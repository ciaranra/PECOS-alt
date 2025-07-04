//! Debug HUGR-LLVM compilation to see the IR format

use pecos_hugr_llvm::compile_hugr_to_llvm;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Path to bell state HUGR
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let hugr_path = workspace_root.join("pecos/tests/test_data/hugr/bell_state.hugr");

    println!("Compiling HUGR with pecos-hugr-llvm...");

    // Compile HUGR to LLVM IR using the standard pipeline
    let llvm_ir_path = compile_hugr_to_llvm(&hugr_path, None)?;

    // Read and display the LLVM IR
    let llvm_ir = fs::read_to_string(&llvm_ir_path)?;
    println!("HUGR-LLVM generated LLVM IR:");
    println!("{llvm_ir}");

    Ok(())
}
