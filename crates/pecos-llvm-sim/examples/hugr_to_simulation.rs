//! Example showing how to use HUGR input with pecos-llvm-sim
//!
//! This example demonstrates the full pipeline from HUGR to simulation results.

use pecos_llvm_sim::LlvmSim;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Using LLVM IR directly
    println!("=== Example 1: LLVM IR Input ===");

    // Simple LLVM IR that allocates qubits
    let llvm_ir = r"
        ; ModuleID = 'quantum_example'
        
        declare i8* @__pecos__new_array(i64)
        declare void @__pecos__end_array(i8*)
        
        define void @main() {
        entry:
            ; Allocate 2 qubits
            %qubits = call i8* @__pecos__new_array(i64 2)
            call void @__pecos__end_array(i8* %qubits)
            ret void
        }
    ";

    // Run simulation with LLVM IR
    let results = LlvmSim::new()
        .llvm(llvm_ir)
        .seed(42)
        .auto_workers() // Use all available CPU cores
        .with_depolarizing_noise(0.01)
        .with_state_vector_engine()
        .run(100)?;

    println!("LLVM simulation completed with {} registers", results.len());

    // Example 2: Using HUGR input (requires HUGR → LLVM compilation)
    println!("\n=== Example 2: HUGR Input ===");

    // Create a simple HUGR
    use hugr_core::builder::{DFGBuilder, Dataflow, DataflowHugr};
    use hugr_core::extension::prelude::qb_t;
    use hugr_core::types::Signature;

    let hugr = {
        let builder = DFGBuilder::new(Signature::new(vec![qb_t()], vec![qb_t()]))?;
        let [q] = builder.input_wires_arr();
        builder.finish_hugr_with_outputs([q])?
    };

    // Create simulation from HUGR
    let _builder = LlvmSim::new().hugr(hugr).seed(42).with_no_noise();

    println!("Created simulation builder from HUGR");

    // Note: Actually running this would require:
    // 1. HUGR → LLVM compilation support (pecos-hugr-llvm)
    // 2. Valid quantum operations in the HUGR
    //
    // let results = builder.run(100)?;

    // Example 3: Loading from files
    println!("\n=== Example 3: File-based Input ===");

    // From LLVM file
    let _llvm_builder = LlvmSim::new().llvm_file("circuit.ll").seed(123).workers(8);
    println!("Created builder from LLVM file");

    // From HUGR file
    let _hugr_builder = LlvmSim::new()
        .hugr_file("circuit.hugr")
        .with_biased_depolarizing_noise(0.005);
    println!("Created builder from HUGR file");

    Ok(())
}
