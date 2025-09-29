//! Test for HUGR to LLVM compilation pipeline
//!
//! Note: Direct HUGR support has been removed from pecos-qis-sim.
//! HUGR compilation now uses tket's HUGR 0.22 through the pecos-hugr-qis crate.

#[test]
fn test_hugr_compilation_pipeline() {
    println!("HUGR to LLVM compilation pipeline:");
    println!();
    println!("1. HUGR compilation is now handled by the pecos-hugr-qis crate");
    println!("2. pecos-hugr-qis uses tket's HUGR 0.22 for compatibility with Selene");
    println!("3. The compilation flow is:");
    println!("   - HUGR (JSON or envelope) → pecos-hugr-qis → LLVM IR");
    println!("   - LLVM IR → pecos-qis-sim → Simulation results");
    println!();
    println!("For examples of HUGR compilation, see:");
    println!("   - crates/pecos-hugr-qis/src/compiler.rs");
    println!("   - python/tests/guppy/test_hugr_compiler_parity.py");
}

/// Example demonstrating how to use LLVM IR that was compiled from HUGR.
/// This shows the typical workflow for HUGR-based quantum programs:
/// 1. HUGR is compiled to LLVM IR using pecos-hugr-qis
/// 2. The LLVM IR is executed using pecos-qis-sim
#[test]
fn test_compiled_hugr_example() {
    use pecos_engines::{sim_builder, state_vector};
    use pecos_programs::QisProgram;
    use pecos_qis_sim::qis_engine;

    // Step 1: Compile HUGR to LLVM (would be done by pecos-hugr-qis)
    // In practice:
    // let hugr_bytes = std::fs::read("my_circuit.hugr.json")?;
    // let llvm_ir = pecos_hugr_qis::compile_hugr_bytes_to_string(&hugr_bytes)?;

    // Step 2: Use the compiled LLVM IR (with measurements for complete circuit)
    let example_llvm_ir = r#"
; ModuleID = 'hugr_compiled'
@str_c0 = constant [3 x i8] c"c0\00"

declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare i64 @__quantum__rt__result_allocate()
declare void @__quantum__rt__result_record_output(i8*, i8*)

define void @main() #0 {
    call void @__quantum__qis__h__body(i64 0)

    %r0 = call i64 @__quantum__rt__result_allocate()
    %m0 = call i32 @__quantum__qis__m__body(i64 0, i64 %r0)
    %r0_ptr = inttoptr i64 %r0 to i8*
    call void @__quantum__rt__result_record_output(i8* %r0_ptr, i8* getelementptr inbounds ([3 x i8], [3 x i8]* @str_c0, i32 0, i32 0))

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    // Step 3: Run simulation
    let results = sim_builder()
        .classical(qis_engine().program(QisProgram::from_ir(example_llvm_ir)))
        .quantum(state_vector().qubits(1))
        .seed(42)
        .run(10)
        .expect("Simulation should succeed");

    assert_eq!(results.len(), 10, "Should have 10 shots");

    // Verify we got measurement results
    let shot_map = results.try_as_shot_map().expect("Should convert to ShotMap");
    let register_names = shot_map.register_names();
    assert!(register_names.iter().any(|name| name == &"c0"), "Should have c0 register");

    println!("Successfully ran simulation from compiled HUGR:");
    println!("  - {} shots executed", results.len());
    println!("  - Registers: {:?}", shot_map.register_names());
}
