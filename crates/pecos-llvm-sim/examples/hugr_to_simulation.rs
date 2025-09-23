//! Example showing LLVM-based quantum simulation with pecos-llvm-sim
//!
//! Note: Direct HUGR support has been removed. HUGR compilation now uses
//! tket's HUGR 0.22 through the pecos-hugr-qis crate.
//!
//! This example demonstrates LLVM IR simulation.

use pecos_engines::{DepolarizingNoise, sim_builder, state_vector};
use pecos_llvm_sim::llvm_engine;
use pecos_programs::LlvmProgram;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLVM-based Quantum Simulation ===");
    println!();
    println!("Note: Direct HUGR input has been removed.");
    println!("To compile HUGR to LLVM, use the pecos-hugr-qis crate.");
    println!();

    // Example: Using LLVM IR directly
    println!("=== Example: LLVM IR Input ===");

    // Simple LLVM IR that allocates qubits
    let llvm_ir = r"
        ; ModuleID = 'quantum_example'

        declare i8* @__pecos__new_array(i64)
        declare void @__pecos__end_array(i8*)
        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cnot__body(i64, i64)

        define void @main() {
        entry:
            ; Allocate 2 qubits
            %qubits = call i8* @__pecos__new_array(i64 2)

            ; Create Bell state
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__cnot__body(i64 0, i64 1)

            call void @__pecos__end_array(i8* %qubits)
            ret void
        }
    ";

    // Run simulation without noise
    let results_no_noise = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_ir(llvm_ir)))
        .quantum(state_vector())
        .run(100)?;

    println!("Ran 100 shots without noise");
    println!("Results: {} unique outcomes", results_no_noise.shots.len());

    // Run simulation with noise
    let results_with_noise = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_ir(llvm_ir)))
        .quantum(state_vector())
        .noise(DepolarizingNoise { p: 0.01 })
        .run(100)?;

    println!("\nRan 100 shots with noise");
    println!("Results: {} unique outcomes", results_with_noise.shots.len());

    println!("\n=== HUGR Compilation ===");
    println!("To compile HUGR to LLVM IR:");
    println!("1. Use the pecos-hugr-qis crate directly in Rust");
    println!("2. Or use Python: pecos_rslib.compile_hugr_to_llvm_rust()");
    println!("3. Then feed the resulting LLVM IR to this simulator");

    Ok(())
}