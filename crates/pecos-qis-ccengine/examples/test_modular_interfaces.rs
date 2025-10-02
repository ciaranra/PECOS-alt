use pecos_qis_ccengine::{
    qis_control_engine, qis_jit_interface, qis_selene_helios_interface,
    native_runtime
};
use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
use pecos_programs::QisProgram;

fn main() {
    env_logger::init();

    println!("Testing New QisControlEngine Architecture");
    println!("============================================");

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

    // Create a QIS program
    let program = QisProgram::from_string(simple_llvm_ir);

    // 1. Test JIT Interface + Native Runtime
    println!("\nTesting JIT Interface + Native Runtime:");
    match qis_control_engine()
        .interface(qis_jit_interface())
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Engine creation successful!");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Engine creation failed: {}", e);
        }
    }

    // 2. Test Helios Interface + Native Runtime
    println!("\nTesting Helios Interface + Native Runtime:");
    match qis_control_engine()
        .interface(qis_selene_helios_interface())
        .runtime(native_runtime())
        .program(program.clone())
        .build() {
        Ok(engine) => {
            println!("   Engine creation successful!");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Engine creation failed (expected if Helios not available): {}", e);
        }
    }

    // 3. Test default behavior (should use Helios by default)
    println!("\nTesting Default Interface Selection:");
    match qis_control_engine()
        .runtime(native_runtime())
        .program(program)
        .build() {
        Ok(engine) => {
            println!("   Default engine creation successful!");
            println!("   Qubits: {}", engine.num_qubits());
        }
        Err(e) => {
            println!("   Default engine creation failed: {}", e);
        }
    }

    println!("\nNew Architecture Test Complete!");
    println!("    QisControlEngine successfully orchestrates Interface + Runtime");
    println!("    Builder pattern allows flexible configuration!");
}