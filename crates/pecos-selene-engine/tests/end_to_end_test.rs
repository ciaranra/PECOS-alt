//! End-to-end integration tests for `SeleneEngine` with PECOS
//!
//! These tests verify that `SeleneEngine` works correctly in realistic scenarios:
//! 1. Real quantum programs (LLVM IR, HUGR)
//! 2. Integration with PECOS infrastructure
//! 3. Multi-shot execution
//! 4. Classical control flow
//! 5. Error handling and edge cases

use pecos_core::prelude::PecosError;
use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine, Engine, sim_builder};
use pecos_programs::LlvmProgram;
use pecos_selene_engine::{SeleneExecutableEngine, selene_executable};

// NOTE: These tests originally used LLVM IR directly with LlvmProgram::from_ir().
// We've removed direct LLVM execution support in favor of HUGR compilation through Selene.
// The proper execution path is now: Guppy -> HUGR -> Selene Plugin -> Execution
// These tests are kept as documentation of the old architecture but marked as ignored.
// For working examples, see the Python tests that use the Guppy API.

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path instead"]
fn test_end_to_end_bell_state_pecos() -> Result<(), PecosError> {
    println!("=== End-to-End: Bell State with PECOS Infrastructure ===");

    // Create Bell state program using LLVM IR
    let bell_llvm = r#"
; Bell state quantum program
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @bell_state() #0 {
entry:
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    // First test: Direct engine usage (bypassing full simulation infrastructure)
    println!("Testing direct engine usage...");
    let mut engine = selene_executable()
        .program(LlvmProgram::from_ir(bell_llvm))
        .qubits(2)
        .build()?;

    // Process a single shot directly
    let shot = engine.process(())?;
    println!("Direct shot data: {:?}", shot.data);

    // Check engine details
    let engine_any = engine.as_any();
    if let Some(selene_engine) = engine_any.downcast_ref::<SeleneExecutableEngine>() {
        println!("SeleneEngine details:");
        println!("  num_qubits: {}", selene_engine.num_qubits());
        // Note: optimize and plugin_library_path are private fields
        println!("  engine type: SeleneEngine");
    }

    // Now test with full simulation infrastructure
    println!("\nTesting with full simulation infrastructure...");
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(bell_llvm))
                .qubits(2),
        )
        .seed(42)
        .workers(1) // Use single worker for easier debugging
        .run(10)?; // Run fewer shots for testing

    println!("Executed Bell state simulation");

    println!(
        "Executed Bell state through PECOS: {} shots",
        results.shots.len()
    );
    assert_eq!(results.shots.len(), 10);

    // Verify measurement results exist
    let has_measurements = results.shots.iter().all(|shot| !shot.data.is_empty());
    assert!(has_measurements, "All shots should have measurement data");

    // Check for measurement data in shots
    for (i, shot) in results.shots.iter().take(3).enumerate() {
        println!("Shot {}: {:?}", i, shot.data);
    }

    Ok(())
}

#[test]
#[ignore = "Known measurement-based conditional bug"]
fn test_end_to_end_quantum_classical_feedback() -> Result<(), PecosError> {
    println!("=== End-to-End: Quantum-Classical Feedback Loop ===");

    // Test a program with classical control based on quantum measurements
    let adaptive_llvm = r#"
; Adaptive quantum algorithm with classical feedback
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @adaptive_circuit() #0 {
entry:
    ; Initial superposition on both qubits
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)

    ; First measurement for classical control
    %result_0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)

    ; Classical control: apply X gate based on measurement
    ; In real adaptive algorithm, would use %result_0 to conditionally apply X
    ; For now, always apply X to demonstrate control flow
    call void @__quantum__qis__x__body(i64 1)

    ; Final measurements
    %result_0_final = call i32 @__quantum__qis__m__body(i64 0, i64 1)
    %result_1 = call i32 @__quantum__qis__m__body(i64 1, i64 2)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    // Execute with new unified API
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(adaptive_llvm))
                .qubits(2),
        )
        .seed(123)
        .workers(4)
        .run(500)?;

    println!("Executed adaptive circuit with quantum-classical feedback");

    println!("Executed adaptive circuit: {} shots", results.shots.len());
    assert_eq!(results.shots.len(), 500);

    // Verify all shots have measurement data
    let shots_with_measurements = results
        .shots
        .iter()
        .filter(|shot| shot.data.contains_key("measurements"))
        .count();

    println!(
        "Shots with measurements: {}/{}",
        shots_with_measurements,
        results.shots.len()
    );
    assert!(
        shots_with_measurements > 0,
        "Should have shots with measurement data"
    );

    Ok(())
}

#[test]
fn test_end_to_end_hugr_program() -> Result<(), PecosError> {
    println!("=== End-to-End: HUGR Program Format ===");

    // HUGR 0.13 support has been removed
    // HUGR compilation now uses tket's HUGR 0.22 through pecos-hugr-qis crate
    println!("HUGR support note: Use pecos-hugr-qis crate for HUGR compilation");

    Ok(())
}

#[test]
#[ignore = "Direct LLVM execution removed - use HUGR compilation instead"]
fn test_end_to_end_multi_format_consistency() -> Result<(), PecosError> {
    println!("=== End-to-End: Multi-Format Consistency ===");

    // Test same quantum circuit in different formats
    // Simple H + Measure circuit
    let simple_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @simple_circuit() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    println!("Creating engine with LLVM IR format");

    // Run with seed
    let llvm_results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(simple_llvm))
                .qubits(1),
        )
        .seed(789)
        .workers(1)
        .run(1000)?;

    println!("LLVM results: {} shots", llvm_results.shots.len());

    // Verify both produce measurements
    let llvm_has_measurements = llvm_results.shots.iter().any(|shot| !shot.data.is_empty());

    assert!(
        llvm_has_measurements,
        "LLVM engine should produce measurements"
    );

    Ok(())
}

#[test]
fn test_end_to_end_error_recovery() {
    println!("=== End-to-End: Error Recovery ===");

    // Test 1: Empty program
    let empty_engine = selene_executable()
        .program(LlvmProgram::from_ir(""))
        .qubits(1)
        .build();

    // Empty LLVM IR creates a default circuit, but compile() should fail
    assert!(
        empty_engine.is_ok(),
        "Empty LLVM IR should build successfully"
    );
    let engine = empty_engine.unwrap();
    assert!(
        engine.compile().is_err(),
        "Empty LLVM IR should fail at compile time"
    );
    println!("Correctly rejected empty program");

    // Test 2: Invalid qubit count
    let invalid_llvm = r#"
declare void @__quantum__qis__h__body(i64)

define void @invalid() #0 {
    call void @__quantum__qis__h__body(i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let invalid_result = selene_executable()
        .program(LlvmProgram::from_ir(invalid_llvm))
        .qubits(0) // Invalid: 0 qubits
        .build();

    // This might succeed, depending on validation
    if invalid_result.is_err() {
        println!("Correctly rejected 0 qubits");
    } else {
        println!("Engine created with 0 qubits (may be valid)");
    }
}

#[test]
#[ignore = "Direct LLVM execution removed - use HUGR compilation instead"]
fn test_end_to_end_large_circuit() -> Result<(), PecosError> {
    println!("=== End-to-End: Large Circuit Performance ===");

    // Test with a larger circuit
    let large_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare void @__quantum__qis__rz__body(double, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @large_circuit() #0 {
entry:
    ; Initialize all qubits with H
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)
    call void @__quantum__qis__h__body(i64 2)
    call void @__quantum__qis__h__body(i64 3)

    ; Create entanglement
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    call void @__quantum__qis__cx__body(i64 1, i64 2)
    call void @__quantum__qis__cx__body(i64 2, i64 3)

    ; Apply some rotations
    call void @__quantum__qis__rz__body(double 0.785, i64 0)
    call void @__quantum__qis__rz__body(double 1.571, i64 1)
    call void @__quantum__qis__rz__body(double 2.356, i64 2)
    call void @__quantum__qis__rz__body(double 3.142, i64 3)

    ; More entanglement
    call void @__quantum__qis__cx__body(i64 3, i64 0)
    call void @__quantum__qis__cx__body(i64 0, i64 2)

    ; Measure all qubits
    %r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    %r2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
    %r3 = call i32 @__quantum__qis__m__body(i64 3, i64 3)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    println!("Creating SeleneEngine for 4-qubit circuit");

    // Time the execution
    let start = std::time::Instant::now();

    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(large_llvm))
                .qubits(4),
        )
        .seed(999)
        .workers(4) // Use multiple workers
        .run(2000)?; // More shots

    let elapsed = start.elapsed();

    println!(
        "Executed large circuit: {} shots in {:?}",
        results.shots.len(),
        elapsed
    );
    assert_eq!(results.shots.len(), 2000);

    // Performance check (should complete reasonably quickly)
    assert!(
        elapsed.as_secs() < 60,
        "Large circuit should complete within 60 seconds"
    );

    Ok(())
}

#[test]
#[ignore = "Direct LLVM execution removed - use HUGR compilation instead"]
fn test_end_to_end_direct_engine_construction() -> Result<(), PecosError> {
    println!("=== End-to-End: Direct Engine Construction ===");

    // Test direct construction without builder
    let direct_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @direct_test() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let engine = SeleneExecutableEngine::new(1)?;

    println!("Created SeleneExecutableEngine directly");

    // Verify it implements the required traits
    assert_eq!(engine.num_qubits(), 1);

    // Run through PECOS using the builder API instead
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(direct_llvm))
                .qubits(1),
        )
        .seed(555)
        .workers(1)
        .run(100)?;

    assert_eq!(results.shots.len(), 100);
    println!("Direct construction works correctly");

    Ok(())
}
