//! Tests for SeleneEngine integration with HybridEngine
//! 
//! This demonstrates the most realistic usage pattern where:
//! - SeleneEngine provides classical control and command generation
//! - HybridEngine coordinates between classical and quantum execution
//! - StateVecEngine (or other quantum engine) handles quantum operations

use pecos_selene::selene_engine;
use pecos_programs::LlvmProgram;
use pecos_engines::{ClassicalControlEngineBuilder, 
    Engine,
    ClassicalEngine,
    hybrid::HybridEngineBuilder, 
    quantum::StateVecEngine,
    ShotVec,
    };
use pecos_core::prelude::PecosError;

#[test]
fn test_selene_with_hybrid_engine_bell_state() -> Result<(), PecosError> {
    env_logger::try_init().ok();
    
    println!("=== Testing SeleneEngine + HybridEngine for Bell State ===");
    
    // Create Selene classical control engine using LLVM IR
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
    
    let selene_engine = selene_engine()
        .program(LlvmProgram::from_ir(bell_llvm))
        .qubits(2)
        .optimize(true)
        .verbose(true)
        .build()?;
    
    println!("Created SeleneEngine for Bell state");
    
    // Create quantum engine
    let quantum_engine = StateVecEngine::new(2);
    println!("Created StateVecEngine for quantum simulation");
    
    // Create hybrid engine combining classical and quantum
    let mut hybrid_engine = HybridEngineBuilder::new()
        .with_classical_engine(Box::new(selene_engine))
        .with_quantum_engine(Box::new(quantum_engine))
        .build();
    
    println!("Created HybridEngine combining Selene and StateVec engines");
    
    // Run a single shot
    let shot = hybrid_engine.process(())?;
    
    println!("Executed Bell state circuit");
    println!("  Shot data: {:?}", shot.data);
    
    // Verify we got measurement results
    assert!(!shot.data.is_empty(), "Should have measurement results");
    
    // Run multiple shots to see Bell state correlations
    hybrid_engine.reset()?;
    let mut shots = ShotVec::new();
    for i in 0..10 {
        let shot = hybrid_engine.process(())?;
        println!("  Shot {}: {:?}", i, shot.data);
        shots.shots.push(shot);
        hybrid_engine.reset()?;
    }
    
    println!("Completed {} shots showing Bell state correlations", shots.len());
    
    Ok(())
}

#[test] 
fn test_selene_adaptive_algorithm() -> Result<(), PecosError> {
    env_logger::try_init().ok();
    
    println!("=== Testing SeleneEngine Adaptive Algorithm ===");
    
    // Adaptive algorithm that applies X conditionally
    let adaptive_llvm = r#"
; Adaptive quantum algorithm
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @adaptive_algorithm() #0 {
entry:
    ; Apply H to qubit 0 and measure
    call void @__quantum__qis__h__body(i64 0)
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    
    ; In a real adaptive algorithm, we'd branch based on %result
    ; For now, always apply X to qubit 1
    call void @__quantum__qis__x__body(i64 1)
    
    ; Measure qubit 1
    %final = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    let selene_engine = selene_engine()
        .program(LlvmProgram::from_ir(adaptive_llvm))
        .qubits(2)
        .verbose(true)
        .build()?;
    
    let quantum_engine = StateVecEngine::new(2);
    
    let mut hybrid_engine = HybridEngineBuilder::new()
        .with_classical_engine(Box::new(selene_engine))
        .with_quantum_engine(Box::new(quantum_engine))
        .build();
    
    println!("Created HybridEngine with adaptive algorithm");
    
    // Run the adaptive algorithm
    let shot = hybrid_engine.process(())?;
    
    println!("Executed adaptive algorithm");
    println!("  Results: {:?}", shot.data);
    
    assert!(!shot.data.is_empty(), "Should have measurement results");
    
    Ok(())
}

#[test]
fn test_selene_multi_qubit_operations() -> Result<(), PecosError> {
    env_logger::try_init().ok();
    
    println!("=== Testing SeleneEngine Multi-Qubit Operations ===");
    
    // Multi-qubit circuit with various gates
    let multi_qubit_llvm = r#"
; Multi-qubit quantum circuit
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare void @__quantum__qis__rz__body(double, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @multi_qubit_circuit() #0 {
entry:
    ; Initialize qubits with H gates
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)
    call void @__quantum__qis__h__body(i64 2)
    
    ; Apply some X gates
    call void @__quantum__qis__x__body(i64 0)
    
    ; Apply CNOT gates
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    call void @__quantum__qis__cx__body(i64 1, i64 2)
    
    ; Apply RZ rotation
    call void @__quantum__qis__rz__body(double 1.57, i64 1)
    
    ; Measure all qubits
    %r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    %r2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    let selene_engine = selene_engine()
        .program(LlvmProgram::from_ir(multi_qubit_llvm))
        .qubits(3)
        .build()?;
    
    let quantum_engine = StateVecEngine::new(3);
    
    let mut hybrid_engine = HybridEngineBuilder::new()
        .with_classical_engine(Box::new(selene_engine))
        .with_quantum_engine(Box::new(quantum_engine))
        .build();
    
    println!("Created HybridEngine for 3-qubit circuit");
    
    // Run multiple shots
    let mut shots = ShotVec::new();
    for i in 0..5 {
        let shot = hybrid_engine.process(())?;
        println!("  Shot {}: {:?}", i, shot.data);
        shots.shots.push(shot);
        hybrid_engine.reset()?;
    }
    
    println!("Completed {} shots of multi-qubit circuit", shots.len());
    
    Ok(())
}

#[test]
fn test_selene_engine_reset() -> Result<(), PecosError> {
    env_logger::try_init().ok();
    
    println!("=== Testing SeleneEngine Reset Functionality ===");
    
    // Simple circuit for reset testing
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
    
    let selene_engine = selene_engine()
        .program(LlvmProgram::from_ir(reset_llvm))
        .qubits(1)
        .build()?;
    
    let quantum_engine = StateVecEngine::new(1);
    
    let mut hybrid_engine = HybridEngineBuilder::new()
        .with_classical_engine(Box::new(selene_engine))
        .with_quantum_engine(Box::new(quantum_engine))
        .build();
    
    // Run first shot
    let shot1 = hybrid_engine.process(())?;
    println!("First shot: {:?}", shot1.data);
    
    // Reset and run again
    hybrid_engine.reset()?;
    println!("Reset completed");
    
    let shot2 = hybrid_engine.process(())?;
    println!("Second shot after reset: {:?}", shot2.data);
    
    // Verify both shots have results
    assert!(!shot1.data.is_empty());
    assert!(!shot2.data.is_empty());
    
    Ok(())
}

#[test]
fn test_selene_error_handling() -> Result<(), PecosError> {
    env_logger::try_init().ok();
    
    println!("=== Testing SeleneEngine Error Handling ===");
    
    // Try to create engine with invalid configuration
    let mut engine = selene_engine()
        .program(LlvmProgram::from_ir("")) // Empty IR should cause error
        .qubits(1) // Valid qubit count (0 qubits would be rejected by builder)
        .build()?;
    
    // The error should occur when trying to compile/use the engine
    let result = engine.generate_commands();
    
    assert!(result.is_err(), "Should fail with empty program");
    println!("Correctly rejected empty program");
    
    // Also test zero qubits case
    let result = selene_engine()
        .program(LlvmProgram::from_ir("define void @main() { ret void }"))
        .qubits(0)
        .build();
    
    assert!(result.is_err(), "Should fail with zero qubits");
    println!("Correctly rejected zero qubits configuration");
    
    Ok(())
}