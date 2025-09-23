//! Test for HUGR to LLVM compilation pipeline
//!
//! Note: Direct HUGR support has been removed from pecos-llvm-sim.
//! HUGR compilation now uses tket's HUGR 0.22 through the pecos-hugr-qis crate.

#[test]
fn test_hugr_compilation_pipeline() {
    println!("HUGR to LLVM compilation pipeline:");
    println!();
    println!("1. HUGR compilation is now handled by the pecos-hugr-qis crate");
    println!("2. pecos-hugr-qis uses tket's HUGR 0.22 for compatibility with Selene");
    println!("3. The compilation flow is:");
    println!("   - HUGR (JSON or envelope) → pecos-hugr-qis → LLVM IR");
    println!("   - LLVM IR → pecos-llvm-sim → Simulation results");
    println!();
    println!("For examples of HUGR compilation, see:");
    println!("   - crates/pecos-hugr-qis/src/compiler.rs");
    println!("   - python/tests/guppy/test_hugr_compiler_parity.py");
}

#[test]
#[ignore = "Example of how to use compiled HUGR"]
fn test_compiled_hugr_example() {
    use pecos_engines::{sim_builder, state_vector};
    use pecos_llvm_sim::llvm_engine;
    use pecos_programs::LlvmProgram;

    // Step 1: Compile HUGR to LLVM (would be done by pecos-hugr-qis)
    // In practice: let llvm_ir = pecos_hugr_qis::compile_hugr_bytes_to_string(hugr_bytes)?;

    // Step 2: Use the compiled LLVM IR
    let example_llvm_ir = r#"
        ; ModuleID = 'hugr_compiled'
        declare void @__quantum__qis__h__body(i64)

        define void @main() #0 {
            call void @__quantum__qis__h__body(i64 0)
            ret void
        }

        attributes #0 = { "EntryPoint" }
    "#;

    // Step 3: Run simulation
    let _builder = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_ir(example_llvm_ir)))
        .quantum(state_vector())
        .seed(42);

    println!("Successfully created simulation from compiled HUGR");
}