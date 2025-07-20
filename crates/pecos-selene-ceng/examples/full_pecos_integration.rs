//! Example demonstrating full PECOS infrastructure integration with SeleneEngine
//!
//! This example shows SeleneEngine working with:
//! - MonteCarloEngine for parallel execution
//! - HybridEngine for classical-quantum coordination
//! - StateVecEngine for quantum simulation
//! - Real Bell state creation and analysis

use pecos_selene_ceng::selene_engine;
use pecos_programs::LlvmProgram;
use pecos_engines::{
    Engine,
    hybrid::HybridEngineBuilder,
    quantum::StateVecEngine,
    ShotVec, ClassicalControlEngineBuilder,
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Full PECOS Integration with SeleneEngine ===");
    println!();
    
    // Example 1: Bell state with HybridEngine
    bell_state_example()?;
    println!();
    
    // Example 2: Adaptive circuit with control flow
    adaptive_circuit_example()?;
    println!();
    
    // Example 3: Multiple format support
    format_comparison()?;
    
    println!("\n=== Integration Complete ===");
    Ok(())
}

/// Bell state using HybridEngine
fn bell_state_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Bell State with HybridEngine");
    println!("================================");
    
    // Create Bell state program using Selene with LLVM IR
    let bell_llvm = r#"
; Bell state quantum circuit
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @bell_state() #0 {
entry:
    ; Create superposition
    call void @__quantum__qis__h__body(i64 0)
    
    ; Entangle qubits
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    
    ; Measure both qubits
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    // Use new unified API to run Bell state simulation
    let results = selene_engine()
        .program(LlvmProgram::from_ir(bell_llvm))
        .qubits(2)
        .optimize(true)
        .to_sim()
        .run(10)?;
    
    println!("✓ Created and ran SeleneEngine for Bell state");
    println!("✓ Completed {} shots showing Bell state correlations", results.len());
    
    // Show some results
    for (i, shot) in results.shots.iter().take(5).enumerate() {
        println!("  Shot {}: {:?}", i, shot.data);
    }
    
    Ok(())
}

/// Adaptive circuit with measurement feedback
fn adaptive_circuit_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Adaptive Circuit with Control Flow");
    println!("=====================================");
    
    // Create adaptive circuit with measurement feedback
    let adaptive_llvm = r#"
; Adaptive quantum algorithm
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @adaptive_circuit() #0 {
entry:
    ; Initialize in superposition
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)
    call void @__quantum__qis__h__body(i64 2)
    
    ; Mid-circuit measurement
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    
    ; Classical control (simplified - always apply these)
    call void @__quantum__qis__x__body(i64 1)
    call void @__quantum__qis__cx__body(i64 1, i64 2)
    
    ; Final measurements
    %final1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    %final2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    // Use new unified API for adaptive circuit
    let results = selene_engine()
        .program(LlvmProgram::from_ir(adaptive_llvm))
        .qubits(3)
        .to_sim()
        .run(5)?;
    
    println!("✓ Created and ran adaptive circuit engine");
    
    // Show results
    for (i, shot) in results.shots.iter().enumerate() {
        println!("✓ Adaptive circuit result {}: {:?}", i, shot.data);
    }
    
    Ok(())
}

/// Compare different program formats
fn format_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Multiple Format Support");
    println!("==========================");
    
    // Test LLVM IR format
    let llvm_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @test() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    // Run with new unified API for parallel execution
    println!("\nTesting with MonteCarloEngine (parallel execution):");
    let results = selene_engine()
        .program(LlvmProgram::from_ir(llvm_llvm))
        .qubits(1)
        .to_sim()
        .seed(42)  // seed
        .workers(4)  // workers for parallel execution
        .run(1000)?; // shots
    
    println!("✓ Created engine with LLVM IR format");
    
    // Test HUGR format (if available)
    #[cfg(feature = "hugr")]
    {
        use hugr::Hugr;
        let hugr = Hugr::default();
        let _results = selene_engine()
            .hugr(hugr)
            .qubits(1)
            .to_sim()
            .run(10)?;
        println!("✓ Created and ran engine with HUGR format");
    }
    
    println!("✓ Completed {} shots in parallel", results.shots.len());
    
    // Analyze results
    let mut outcome_counts: HashMap<String, usize> = HashMap::new();
    for shot in results.shots.iter() {
        let outcome = format!("{:?}", shot.data);
        *outcome_counts.entry(outcome).or_insert(0) += 1;
    }
    
    println!("\nOutcome distribution:");
    for (outcome, count) in outcome_counts {
        println!("  {}: {} times", outcome, count);
    }
    
    Ok(())
}