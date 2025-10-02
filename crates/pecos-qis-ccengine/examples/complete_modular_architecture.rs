use pecos_qis_ccengine::{
    // Builder functions
    qis_control_engine, qis_jit_interface, qis_selene_helios_interface,
    native_runtime,
    // Runtime types
    QisSeleneSimpleRuntime, QisRuntime,
};
use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
use pecos_programs::QisProgram;

fn main() {
    env_logger::init();

    println!("Complete QisControlEngine Architecture");
    println!("=========================================");

    demo_engine_creation_patterns();
    demo_runtime_implementations();
    demo_interface_runtime_combinations();
    demo_program_handling();

    println!("\nComplete Architecture Demonstration Finished!");
}

fn demo_engine_creation_patterns() {
    println!("\nEngine Creation Pattern Demonstrations:");
    println!("------------------------------------------");

    // Test LLVM IR program
    let simple_llvm_ir = r#"; ModuleID = 'test'
source_filename = "test"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

define i64 @qmain(i64 %0) {
entry:
  ret i64 42
}
"#;

    let program = QisProgram::from_string(simple_llvm_ir);

    // 1. Explicit interface and runtime selection
    println!("\nExplicit JIT + Native:");
    match qis_control_engine()
        .interface(qis_jit_interface())
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Success: JIT interface + Native runtime engine created");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Failed: Engine creation failed: {}", e);
        }
    }

    // 2. Default interface with explicit runtime
    println!("\nDefault Interface + Native Runtime:");
    match qis_control_engine()
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Success: Default interface + Native runtime engine created");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Engine creation failed: {}", e);
        }
    }

    // 3. Fully default configuration
    println!("\nFully Default Configuration:");
    match qis_control_engine()
        .program(program)
        .build() {
        Ok(engine) => {
            println!("   Success: Fully default engine created");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Engine creation failed: {}", e);
        }
    }
}

fn demo_runtime_implementations() {
    println!("\nRuntime Implementation Demonstrations:");
    println!("----------------------------------------");

    // 1. Native Runtime
    println!("\nNativeRuntime:");
    let native = native_runtime();
    println!("   Type: Pure Rust implementation");
    println!("   Qubits: {}", native.num_qubits());

    // 2. Selene Simple Runtime (if available)
    println!("\nQisSeleneSimpleRuntime:");
    match QisSeleneSimpleRuntime::new() {
        Ok(runtime) => {
            println!("   Success: Selene Simple Runtime created");
            println!("   Qubits: {}", runtime.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Selene not available: {}", e);
        }
    }
}

fn demo_interface_runtime_combinations() {
    println!("\nInterface + Runtime Combinations:");
    println!("------------------------------------");

    let simple_llvm_ir = r#"; ModuleID = 'simple'
define i64 @qmain(i64 %0) { entry: ret i64 0 }
"#;
    let program = QisProgram::from_string(simple_llvm_ir);

    // Demo JIT Interface with different runtimes
    println!("\nJIT Interface + Native Runtime:");
    match qis_control_engine()
        .interface(qis_jit_interface())
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Success: Successfully created engine");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Engine creation failed: {}", e);
        }
    }

    println!("\nJIT Interface + Native Runtime (second test):");
    match qis_control_engine()
        .interface(qis_jit_interface())
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Success: Successfully created engine");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Engine creation failed: {}", e);
        }
    }

    println!("\nHelios Interface + Native Runtime:");
    match qis_control_engine()
        .interface(qis_selene_helios_interface())
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Success: Successfully created engine");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Engine creation failed: {}", e);
        }
    }
}

fn demo_program_handling() {
    println!("\nProgram Handling Demonstrations:");
    println!("-----------------------------------");

    println!("\nSupported Program Types:");
    println!("   - LLVM IR text → Processed by interface implementations");
    println!("   - LLVM bitcode → Processed by interface implementations");
    println!("   - HUGR bytes → Converted to LLVM then processed");

    println!("\nBuilder API Features:");
    println!("   - .interface() → Explicit interface selection");
    println!("   - .runtime() → Explicit runtime selection");
    println!("   - .program() → Accepts QisProgram objects");
    println!("   - .build() → Creates QisControlEngine instance");

    // Demo that the builder works without programs
    match qis_control_engine()
        .interface(qis_jit_interface())
        .runtime(native_runtime())
        .build() {
        Ok(engine) => {
            println!("   Success: Engine created without program");
            println!("   Qubits: {} (program can be loaded later)", engine.num_qubits());
        }
        Err(e) => {
            println!("   Warning: Engine build failed: {}", e);
        }
    }
}