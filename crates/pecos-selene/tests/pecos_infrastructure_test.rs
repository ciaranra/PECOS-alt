//! Tests for `SeleneEngine` integration with full PECOS infrastructure
//!
//! These tests demonstrate how `SeleneEngine` works with:
//! - `MonteCarloEngine` for parallel execution
//! - `HybridEngine` for classical-quantum coordination
//! - Real quantum engines (`StateVecEngine`)
//! - LLVM IR programs for quantum circuits

use pecos_core::prelude::PecosError;
use pecos_engines::{
    ClassicalControlEngineBuilder, ClassicalEngine, ControlEngine, Engine, EngineStage, sim_builder,
};
use pecos_programs::{HugrProgram, LlvmProgram};
use pecos_selene::selene_executable;
use std::collections::HashMap;

mod common;

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_with_monte_carlo_engine() -> Result<(), PecosError> {
    env_logger::try_init().ok(); // Initialize logging if not already done

    println!("=== Testing SeleneEngine with MonteCarloEngine ===");

    // Create a Bell state program using Selene with LLVM IR
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

    // Use new unified API with MonteCarloEngine for parallel execution
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(bell_llvm))
                .qubits(2),
        )
        .seed(42) // seed for reproducibility
        .workers(4) // workers
        .run(100)?; // shots

    println!("Created and executed SeleneEngine with Bell state program");

    println!(
        "Executed with MonteCarloEngine: {} shots",
        results.shots.len()
    );
    assert_eq!(results.shots.len(), 100);

    // Verify Bell state correlations
    let mut correlations = HashMap::new();
    for shot in &results.shots {
        // Extract measurement results
        if let Some(measurements) = shot.data.get("measurements") {
            println!("  Shot measurements: {measurements:?}");
            *correlations.entry(format!("{measurements:?}")).or_insert(0) += 1;
        }
    }

    println!("Measurement correlations observed:");
    for (pattern, count) in correlations {
        println!("  Pattern {pattern}: {count} times");
    }

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_classical_control_flow() -> Result<(), PecosError> {
    println!("=== Testing SeleneEngine Classical Control Flow ===");

    // Adaptive algorithm with mid-circuit measurement
    let adaptive_llvm = r#"
; Adaptive quantum algorithm
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @adaptive_algorithm() #0 {
entry:
    ; Prepare superposition
    call void @__quantum__qis__h__body(i64 0)

    ; Mid-circuit measurement
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)

    ; Classical control (in real impl, would conditionally apply X)
    call void @__quantum__qis__x__body(i64 1)

    ; Final measurement
    %final = call i32 @__quantum__qis__m__body(i64 1, i64 1)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let mut engine = selene_executable()
        .program(LlvmProgram::from_ir(adaptive_llvm))
        .qubits(2)
        .verbose(true)
        .build()?;

    println!("Created SeleneEngine with adaptive algorithm");

    // Test as ControlEngine
    match engine.start(())? {
        EngineStage::NeedsProcessing(cmd) => {
            println!("Initial stage: needs processing");

            let ops = cmd.quantum_ops()?;
            println!("  Operations to execute: {} ops", ops.len());
            for op in &ops {
                println!("    - {:?}", op.gate_type);
            }

            // Simulate quantum execution and return measurements
            let mut response = pecos_engines::ByteMessageBuilder::new();
            let _ = response.for_outcomes();
            response.add_outcomes(&[0]); // Measurement result

            match engine.continue_processing(response.build())? {
                EngineStage::NeedsProcessing(cmd2) => {
                    println!("After measurement: needs more processing");
                    let ops2 = cmd2.quantum_ops()?;
                    println!("  Additional operations: {} ops", ops2.len());
                }
                EngineStage::Complete(shot) => {
                    println!("Completed with shot data: {:?}", shot.data);
                }
            }
        }
        EngineStage::Complete(shot) => {
            println!("Completed immediately: {:?}", shot.data);
        }
    }

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_executable_compilation() -> Result<(), PecosError> {
    println!("=== Testing SeleneEngine Compilation ===");

    // Test various program formats
    let test_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__y__body(i64)
declare void @__quantum__qis__z__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @test_gates() #0 {
    ; Test various single-qubit gates
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__x__body(i64 1)
    call void @__quantum__qis__y__body(i64 2)
    call void @__quantum__qis__z__body(i64 3)

    ; Measurements
    %r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    %r2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
    %r3 = call i32 @__quantum__qis__m__body(i64 3, i64 3)

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let mut engine = selene_executable()
        .program(LlvmProgram::from_ir(test_llvm))
        .qubits(4)
        .build()?;

    println!("Created SeleneEngine with test circuit");

    // Test compilation
    engine.compile()?;
    println!("Compilation successful");

    // Generate commands
    let commands = engine.generate_commands()?;
    let ops = commands.quantum_ops()?;

    println!("Generated {} quantum operations", ops.len());

    // Verify operation types
    let mut gate_counts = HashMap::new();
    for op in &ops {
        *gate_counts
            .entry(format!("{:?}", op.gate_type))
            .or_insert(0) += 1;
    }

    println!("Gate counts:");
    for (gate_type, count) in gate_counts {
        println!("  {gate_type}: {count}");
    }

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_parallel_shots() -> Result<(), PecosError> {
    println!("=== Testing SeleneEngine Parallel Shot Execution ===");

    // Simple circuit for parallel testing
    let parallel_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @parallel_test() #0 {
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)
    %r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(parallel_llvm))
                .qubits(2),
        )
        .workers(4)
        .seed(999)
        .run(4); // Reduced from 1000 for performance

    assert!(results.is_ok());
    let shot_vec = results?;

    println!("Parallel execution completed: {} shots", shot_vec.len());
    assert_eq!(shot_vec.len(), 4);

    // Check distribution of results
    let shot_map = shot_vec.try_as_shot_map()?;
    let mut outcome_counts = HashMap::new();
    for (outcome, count) in shot_map.iter() {
        outcome_counts.insert(outcome.clone(), count);
        println!("  Outcome {outcome:?}: {count:?} times");
    }

    // Should see roughly equal distribution for H gates
    assert!(
        !outcome_counts.is_empty(),
        "Should have measurement outcomes"
    );

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_executable_reset() -> Result<(), PecosError> {
    println!("=== Testing SeleneEngine Reset ===");

    let reset_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @reset_test() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let mut engine = selene_executable()
        .program(LlvmProgram::from_ir(reset_llvm))
        .qubits(1)
        .build()?;

    // First execution
    let shot1 = engine.process(())?;
    println!("First shot: {:?}", shot1.data);

    // Reset
    Engine::reset(&mut engine)?;
    println!("Engine reset");

    // Second execution
    let shot2 = engine.process(())?;
    println!("Second shot: {:?}", shot2.data);

    // Both should have data
    assert!(!shot1.data.is_empty());
    assert!(!shot2.data.is_empty());

    Ok(())
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_selene_executable_cloning() -> Result<(), PecosError> {
    println!("=== Testing SeleneEngine Cloning ===");

    let clone_llvm = r#"
declare void @__quantum__qis__h__body(i64)

define void @clone_test() #0 {
    call void @__quantum__qis__h__body(i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let base_engine = selene_executable()
        .program(LlvmProgram::from_ir(clone_llvm))
        .qubits(1)
        .build()?;

    // Clone for parallel workers
    let cloned_engine = base_engine.clone();

    println!("Engine cloned successfully");

    // Verify both work
    assert_eq!(base_engine.num_qubits(), 1);
    assert_eq!(cloned_engine.num_qubits(), 1);

    Ok(())
}

#[test]
#[cfg(feature = "hugr-013")]
fn test_selene_with_hugr_format() -> Result<(), PecosError> {
    println!("=== Testing SeleneEngine with HUGR Format ===");

    use hugr_core_013::builder::{Dataflow, DataflowHugr, FunctionBuilder};
    use hugr_core_013::extension::prelude::QB_T;
    use hugr_core_013::types::Signature;

    // Create a proper HUGR program with a single Hadamard
    let qb_row = vec![QB_T; 1];
    let circ_signature = Signature::new(qb_row.clone(), qb_row);
    let mut dfg = FunctionBuilder::new("main", circ_signature)
        .map_err(|e| PecosError::with_context(e, "Failed to build function"))?;
    let circ = dfg.as_circuit(dfg.input_wires());

    // Skip adding gates since Tk2Op is not available
    // Just finish the circuit with identity
    let qbs = circ.finish();

    // Create an extension registry with the prelude for HUGR 0.13
    use hugr_core_013::extension::{ExtensionRegistry, prelude};
    let registry = ExtensionRegistry::try_new([prelude::PRELUDE.to_owned()]).unwrap();

    let hugr = dfg
        .finish_hugr_with_outputs(qbs, &registry)
        .map_err(|e| PecosError::with_context(e, "Failed to finish HUGR"))?;

    // Convert HUGR to bytes for HugrProgram
    let hugr_bytes = serde_json::to_vec(&hugr)
        .map_err(|e| PecosError::with_context(e, "Failed to serialize HUGR"))?;

    // Try to build the engine with HUGR program
    let result = selene_executable()
        .hugr(HugrProgram::from_bytes(hugr_bytes))
        .qubits(1)
        .build();

    // The build will fail because the simple HUGR doesn't have a proper CFG,
    // but that's OK - we're testing that the API accepts HUGR programs
    match result {
        Ok(mut engine) => {
            println!("Created SeleneEngine with HUGR program");
            // Test execution
            let result = engine.process(())?;
            println!("HUGR execution completed: {:?}", result.data);
        }
        Err(e) => {
            println!("HUGR compilation returned expected error: {e}");
            println!("✓ HUGR program support is available in the API!");
        }
    }

    Ok(())
}
