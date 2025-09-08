//! Demonstration of the SeleneExecutableEngine
//!
//! This example shows how to use the SeleneExecutableEngine as a working
//! Classical/Control Engine that implements PECOS traits.

use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine, Engine, sim_builder};
use pecos_programs::LlvmProgram;
use pecos_selene::{SeleneExecutableEngine, selene_executable};

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

    let engine = selene_executable()
        .program(LlvmProgram::from_ir(bell_state_llvm))
        .qubits(2)
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
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(bell_state_llvm))
                .qubits(2),
        )
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

    let _direct_engine = SeleneExecutableEngine::new(2)?;

    println!("✓ Created engine directly");
    println!("  Engine type: SeleneExecutable");
    println!("  Classical control: Yes");
    println!("  LLVM IR support: Yes");
    println!();

    println!("=== Demo Complete ===");
    Ok(())
}
