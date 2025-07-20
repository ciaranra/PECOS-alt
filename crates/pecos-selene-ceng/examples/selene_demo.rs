//! Demonstration of the SeleneEngine
//!
//! This example shows how to use the SeleneEngine as a working
//! Classical/Control Engine that implements PECOS traits.

use env_logger;
use pecos_selene_ceng::selene_engine;
use pecos_engines::{ClassicalEngine, Engine, ClassicalControlEngineBuilder};
use pecos_programs::LlvmProgram;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("=== SeleneEngine Demo ===");
    println!();

    // Create a SeleneEngine using the builder with LLVM IR
    println!("1. Creating SeleneEngine using builder...");
    
    let bell_state_llvm = r#"
; Bell state quantum program in LLVM IR
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @bell_state() #0 {
entry:
    ; Apply Hadamard to qubit 0
    call void @__quantum__qis__h__body(i64 0)
    
    ; Apply CNOT with control=0, target=1
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    
    ; Measure both qubits
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    
    ret void
}

attributes #0 = { "EntryPoint" }
"#;
    
    let engine = selene_engine()
        .program(LlvmProgram::from_ir(bell_state_llvm))
        .qubits(2)
        .optimize(true)
        .verbose(true)
        .build()?;

    println!("✓ Successfully created SeleneEngine");
    println!("  Qubits: {}", engine.num_qubits());
    println!();

    // Test as ClassicalEngine
    println!("2. Testing as ClassicalEngine...");
    
    // Clone the engine (demonstrating it's Send + Sync + Clone)
    let mut engine_clone = engine.clone();
    println!("✓ Engine cloned successfully (ready for parallel execution)");
    println!();

    // Run a shot
    println!("3. Running quantum program...");
    let shot = engine_clone.process(())?;
    
    println!("✓ Execution completed!");
    println!("  Shot data: {:?}", shot.data);
    println!();

    // Run multiple shots
    println!("4. Running multiple shots...");
    let results = selene_engine()
        .program(LlvmProgram::from_ir(bell_state_llvm))
        .qubits(2)
        .to_sim()
        .seed(42)
        .workers(2)
        .run(10)?;
    
    println!("✓ Completed {} shots", results.len());
    
    // Convert to ShotMap for display
    let shot_map = results.try_as_shot_map()?;
    
    // Display results
    println!("  Total shots: {}", shot_map.num_shots());
    println!();

    // Demonstrate direct construction
    println!("5. Direct engine construction...");
    use pecos_selene_ceng::{SeleneEngine, program::SeleneProgram};
    
    let _direct_engine = SeleneEngine::new(
        SeleneProgram::LlvmIr(bell_state_llvm.to_string()),
        2,
        true, // optimize
    );
    
    println!("✓ Created engine directly");
    println!("  Engine type: Selene");
    println!("  Classical control: Yes");
    println!("  LLVM IR support: Yes");
    println!();

    println!("=== Demo Complete ===");
    Ok(())
}