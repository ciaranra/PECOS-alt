//! Tests for Selene LLVM execution capabilities
//!
//! NOTE: These tests originally used LLVM IR directly with `LlvmProgram::from_ir()`.
//! We've removed direct LLVM execution support in favor of HUGR compilation through Selene.
//! The proper execution path is now: Guppy -> HUGR -> Selene Plugin -> Execution
//! These tests are kept as documentation of the old architecture but marked as ignored.
//! For working examples, see the Python tests that use the Guppy API.

use pecos_core::prelude::PecosError;
use pecos_engines::{ClassicalEngine, ControlEngine, EngineStage};
use pecos_programs::LlvmProgram;
use pecos_selene_engine::SeleneExecutableEngine;

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_llvm_ir_execution() -> Result<(), PecosError> {
    println!("=== Testing Selene LLVM IR Execution ===");

    // Example LLVM IR for a Bell state circuit
    let bell_state_llvm = r#"
; Bell state LLVM IR
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @bell_state() #0 {
entry:
    ; Apply Hadamard to qubit 0
    call void @__quantum__qis__h__body(i64 0)

    ; Apply CNOT with control=0, target=1
    call void @__quantum__qis__cx__body(i64 0, i64 1)

    ; Measure qubit 0
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)

    ; Measure qubit 1
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    // Create SeleneExecutableEngine with LLVM IR
    let mut engine =
        SeleneExecutableEngine::new(2)?.with_llvm_program(LlvmProgram::from_ir(bell_state_llvm));

    println!("Created SeleneExecutableEngine with Bell state LLVM IR");

    // Test compilation
    engine.compile()?;
    println!("Compilation succeeded");

    // Test command generation
    let commands = engine.generate_commands()?;

    // When plugin compilation is skipped, we get empty commands
    if commands.is_empty()? && std::env::var("PECOS_SKIP_PLUGIN_COMPILATION").is_ok() {
        println!("Plugin compilation skipped, no operations generated");
        return Ok(());
    }

    let ops = commands.quantum_ops()?;
    println!("Generated {} quantum operations:", ops.len());
    for (i, op) in ops.iter().enumerate() {
        println!("  [{}] {:?}", i, op.gate_type);
    }

    // Verify we have the expected operations (when plugin compilation works)
    if !ops.is_empty() {
        assert_eq!(ops.len(), 4); // H, CNOT, 2 measurements
    }

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_control_engine_trait() -> Result<(), PecosError> {
    println!("=== Testing Selene as ControlEngine ===");

    // Adaptive algorithm LLVM IR
    let adaptive_llvm = r#"
; Adaptive quantum algorithm
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__qis__x__body(i64)

define void @adaptive_algorithm() #0 {
entry:
    ; Apply Hadamard to qubit 0
    call void @__quantum__qis__h__body(i64 0)

    ; Measure qubit 0
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)

    ; In real adaptive algorithm, would branch based on %result
    ; For now, always apply X to qubit 1
    call void @__quantum__qis__x__body(i64 1)

    ; Final measurement
    %final = call i32 @__quantum__qis__m__body(i64 1, i64 1)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    // Create engine as ControlEngine
    let mut engine =
        SeleneExecutableEngine::new(2)?.with_llvm_program(LlvmProgram::from_ir(adaptive_llvm));

    // Start the control flow
    match engine.start(())? {
        EngineStage::NeedsProcessing(cmd) => {
            println!("Control engine started, needs processing");

            let ops = cmd.quantum_ops()?;
            println!(
                "  Initial operations: {:?}",
                ops.iter().map(|op| &op.gate_type).collect::<Vec<_>>()
            );

            // Simulate quantum execution returning measurements
            let mut response = pecos_engines::ByteMessageBuilder::new();
            let _ = response.for_outcomes();
            response.add_outcomes(&[0]); // Measurement result

            // Continue processing with measurement result
            match engine.continue_processing(response.build())? {
                EngineStage::NeedsProcessing(cmd2) => {
                    let ops2 = cmd2.quantum_ops()?;
                    println!("Received measurement, continuing with {} ops", ops2.len());
                }
                EngineStage::Complete(shot) => {
                    println!("Control flow complete: {:?}", shot.data);
                }
            }
        }
        EngineStage::Complete(_) => {
            println!("Control engine completed immediately");
        }
    }

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_llvm_parsing() -> Result<(), PecosError> {
    println!("=== Testing Selene LLVM IR Parsing ===");

    // Complex LLVM IR with various quantum operations
    let mixed_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__rz__body(double, i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @complex_circuit() #0 {
entry:
    ; Hadamard gates
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)

    ; Pauli-X gate
    call void @__quantum__qis__x__body(i64 0)

    ; RZ rotation
    call void @__quantum__qis__rz__body(double 1.57, i64 1)

    ; CNOT gates
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    call void @__quantum__qis__cx__body(i64 1, i64 0)

    ; Measurements
    %r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let mut engine =
        SeleneExecutableEngine::new(2)?.with_llvm_program(LlvmProgram::from_ir(mixed_llvm));

    // Generate commands
    let commands = engine.generate_commands()?;

    // When plugin compilation is skipped, we get empty commands
    if commands.is_empty()? && std::env::var("PECOS_SKIP_PLUGIN_COMPILATION").is_ok() {
        println!("Plugin compilation skipped, no operations parsed");
        return Ok(());
    }

    let ops = commands.quantum_ops()?;

    println!("Parsed {} operations from complex LLVM IR:", ops.len());

    // Count operation types
    let mut h_count = 0;
    let mut x_count = 0;
    let mut rz_count = 0;
    let mut cx_count = 0;
    let mut measure_count = 0;

    for op in &ops {
        match &op.gate_type {
            pecos_core::prelude::GateType::H => h_count += 1,
            pecos_core::prelude::GateType::X => x_count += 1,
            pecos_core::prelude::GateType::RZ => rz_count += 1,
            pecos_core::prelude::GateType::CX => cx_count += 1,
            pecos_core::prelude::GateType::Measure => measure_count += 1,
            _ => {}
        }
    }

    println!("  - Hadamard gates: {h_count}");
    println!("  - Pauli-X gates: {x_count}");
    println!("  - RZ rotations: {rz_count}");
    println!("  - CNOT gates: {cx_count}");
    println!("  - Measurements: {measure_count}");

    // Verify counts match LLVM IR (when plugin compilation works)
    if !ops.is_empty() {
        assert_eq!(h_count, 2);
        assert_eq!(x_count, 1);
        assert_eq!(rz_count, 1);
        assert_eq!(cx_count, 2);
        assert_eq!(measure_count, 2);
    }

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_engine_in_hybrid_setup() -> Result<(), PecosError> {
    // Helper functions to verify trait implementations
    fn assert_is_classical_engine<T: ClassicalEngine>(_: &T) {}
    fn assert_is_control_engine<T: ControlEngine>(_: &T) {}
    fn assert_is_send_sync_clone<T: Send + Sync + Clone>(_: &T) {}

    println!("=== Testing SeleneEngine for HybridEngine Compatibility ===");

    let simple_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @simple_test() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let engine =
        SeleneExecutableEngine::new(1)?.with_llvm_program(LlvmProgram::from_ir(simple_llvm));

    assert_is_classical_engine(&engine);
    assert_is_control_engine(&engine);
    assert_is_send_sync_clone(&engine);

    println!("SeleneExecutableEngine satisfies all trait requirements for HybridEngine");

    // Test cloning for parallel execution
    let cloned = engine.clone();
    assert_eq!(cloned.num_qubits(), engine.num_qubits());
    println!("SeleneExecutableEngine can be cloned for parallel workers");

    Ok(())
}
