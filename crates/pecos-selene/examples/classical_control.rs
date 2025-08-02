//! Example demonstrating Selene as a ClassicalControlEngine
//!
//! This shows the proper separation of concerns:
//! - Selene handles classical control flow and command generation
//! - PECOS QuantumEngine handles the actual quantum simulation

use pecos_selene::selene_engine;
use pecos_programs::LlvmProgram;
use pecos_engines::{ControlEngine, EngineStage, ByteMessageBuilder, ClassicalControlEngineBuilder, sim_builder};
use pecos_core::prelude::PecosError;

fn main() -> Result<(), PecosError> {
    env_logger::init();
    
    println!("=== Selene Classical Control Engine Demo ===");
    println!();
    
    // Example 1: Basic quantum-classical feedback
    basic_feedback_example()?;
    println!();
    
    // Example 2: Adaptive algorithm
    adaptive_algorithm_example()?;
    println!();
    
    println!("=== Demo Complete ===");
    Ok(())
}

fn basic_feedback_example() -> Result<(), PecosError> {
    println!("1. Basic Quantum-Classical Feedback");
    println!("===================================");
    
    // LLVM IR program with measurement and classical control
    let feedback_llvm = r#"
; Quantum-classical feedback example
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @feedback_circuit() #0 {
entry:
    ; Prepare superposition
    call void @__quantum__qis__h__body(i64 0)
    
    ; Measure qubit 0
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    
    ; Classical control - apply X to qubit 1
    ; (In real control flow, this would be conditional on %result)
    call void @__quantum__qis__x__body(i64 1)
    
    ; Measure qubit 1
    %final = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    // Create a Selene classical control engine
    let mut engine = selene_engine()
        .program(LlvmProgram::from_ir(feedback_llvm))
        .qubits(2)
        .optimize(true)
        .verbose(true)
        .build()?;
    
    println!("✓ Created SeleneEngine with feedback circuit");
    
    // Demonstrate ControlEngine interface
    match engine.start(())? {
        EngineStage::NeedsProcessing(cmd) => {
            println!("✓ Initial stage - quantum operations needed");
            
            let ops = cmd.quantum_ops()?;
            println!("  Operations to execute: {} ops", ops.len());
            for op in &ops {
                println!("    - {:?}", op.gate_type);
            }
            
            // Simulate quantum execution returning measurement
            let mut response = ByteMessageBuilder::new();
            let _ = response.for_outcomes();
            response.add_outcomes(&[0]); // Measurement result = 0
            
            // Continue with classical control
            match engine.continue_processing(response.build())? {
                EngineStage::NeedsProcessing(cmd2) => {
                    println!("✓ After measurement - more operations needed");
                    let ops2 = cmd2.quantum_ops()?;
                    println!("  Additional operations: {} ops", ops2.len());
                }
                EngineStage::Complete(shot) => {
                    println!("✓ Completed: {:?}", shot.data);
                }
            }
        }
        EngineStage::Complete(shot) => {
            println!("✓ Completed immediately: {:?}", shot.data);
        }
    }
    
    Ok(())
}

fn adaptive_algorithm_example() -> Result<(), PecosError> {
    println!("2. Adaptive Quantum Algorithm");
    println!("=============================");
    
    // More complex adaptive algorithm
    let adaptive_llvm = r#"
; Adaptive quantum algorithm
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__z__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @adaptive_algorithm() #0 {
entry:
    ; Initialize qubits in superposition
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)
    call void @__quantum__qis__h__body(i64 2)
    
    ; First measurement for adaptation
    %r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    
    ; Adaptive operations based on measurement
    ; (In real implementation, these would be conditional)
    call void @__quantum__qis__x__body(i64 1)
    call void @__quantum__qis__cx__body(i64 1, i64 2)
    
    ; Second measurement
    %r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    
    ; Further adaptation
    call void @__quantum__qis__z__body(i64 2)
    
    ; Final measurement
    %r2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    // Run adaptive algorithm using unified API
    let results = sim_builder()
        .classical(selene_engine()
            .program(LlvmProgram::from_ir(adaptive_llvm))
            .qubits(3))
        .run(10)?;
    
    println!("✓ Ran adaptive algorithm: {} shots", results.len());
    
    // Show key features
    println!("\nKey Features of SeleneEngine:");
    println!("- Parses LLVM IR quantum programs");
    println!("- Implements ClassicalEngine and ControlEngine traits");
    println!("- Thread-safe (Send + Sync + Clone)");
    println!("- Integrates with PECOS HybridEngine");
    println!("- Supports mid-circuit measurements");
    println!("- Enables quantum-classical feedback loops");
    
    // Show architecture
    println!("\nArchitecture:");
    println!("┌─────────────────┐");
    println!("│  SeleneEngine   │ ← Classical Control (LLVM IR)");
    println!("└────────┬────────┘");
    println!("         │");
    println!("┌────────▼────────┐");
    println!("│  HybridEngine   │ ← Orchestration");
    println!("└────────┬────────┘");
    println!("         │");
    println!("┌────────▼────────┐");
    println!("│ QuantumEngine   │ ← Quantum Simulation");
    println!("└─────────────────┘");
    
    Ok(())
}