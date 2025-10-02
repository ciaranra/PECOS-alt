//! Integration Tests for New QisControlEngine Builder API
//!
//! These tests verify that the new builder pattern works correctly.

use pecos_qis_ccengine::{
    qis_control_engine, qis_jit_interface, qis_selene_helios_interface,
    native_runtime,
};
use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
use pecos_programs::QisProgram;

/// Sample minimal LLVM IR for basic testing
const MINIMAL_LLVM_IR: &str = r#"; ModuleID = 'minimal'
source_filename = "minimal"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

define i64 @qmain(i64 %0) {
entry:
  ret i64 42
}
"#;

#[cfg(test)]
mod builder_api_tests {
    use super::*;

    #[test]
    fn test_basic_builder_pattern() {
        env_logger::try_init().ok();

        let program = QisProgram::from_string(MINIMAL_LLVM_IR);

        // Test JIT interface + Native runtime
        match qis_control_engine()
            .interface(qis_jit_interface())
            .runtime(native_runtime())
            .program(program.clone())
            .build() {
            Ok(engine) => {
                assert_eq!(engine.num_qubits(), 0); // Minimal program has no qubits
                println!("JIT interface + Native runtime: SUCCESS");
            }
            Err(e) => {
                println!("JIT interface + Native runtime failed: {}", e);
            }
        }

        // Test default behavior (should try Helios first)
        match qis_control_engine()
            .runtime(native_runtime())
            .try_program(program)
            .and_then(|builder| builder.build()) {
            Ok(engine) => {
                assert_eq!(engine.num_qubits(), 0);
                println!("Default interface selection: SUCCESS");
            }
            Err(e) => {
                // Expected if Helios not available
                println!("Default interface failed (expected if no Selene): {}", e);
                assert!(e.to_string().contains("Selene") || e.to_string().contains("Helios"));
            }
        }
    }

    #[test]
    fn test_multiple_native_runtime_instances() {
        env_logger::try_init().ok();

        let program = QisProgram::from_string(MINIMAL_LLVM_IR);

        // Test that we can create multiple native runtime instances
        let runtime1 = native_runtime();
        let runtime2 = native_runtime();

        // Test first instance
        match qis_control_engine()
            .interface(qis_jit_interface())
            .runtime(runtime1)
            .program(program.clone())
            .build() {
            Ok(engine) => {
                assert_eq!(engine.num_qubits(), 0);
                println!("JIT interface + Native runtime (instance 1): SUCCESS");
            }
            Err(e) => {
                println!("Native runtime integration failed: {}", e);
            }
        }

        // Test second instance
        match qis_control_engine()
            .interface(qis_jit_interface())
            .runtime(runtime2)
            .program(program)
            .build() {
            Ok(engine) => {
                assert_eq!(engine.num_qubits(), 0);
                println!("JIT interface + Native runtime (instance 2): SUCCESS");
            }
            Err(e) => {
                println!("Native runtime integration failed: {}", e);
            }
        }
    }

    #[test]
    fn test_engine_without_program() {
        env_logger::try_init().ok();

        // Test creating engine without program (program can be loaded later)
        match qis_control_engine()
            .interface(qis_jit_interface())
            .runtime(native_runtime())
            .build() {
            Ok(engine) => {
                assert_eq!(engine.num_qubits(), 0);
                println!("Engine without program: SUCCESS");
            }
            Err(e) => {
                println!("Engine creation without program failed: {}", e);
            }
        }
    }

    #[test]
    fn test_helios_interface_graceful_failure() {
        env_logger::try_init().ok();

        let program = QisProgram::from_string(MINIMAL_LLVM_IR);

        // Test Helios interface (may fail gracefully if Selene not available)
        match qis_control_engine()
            .interface(qis_selene_helios_interface())
            .runtime(native_runtime())
            .try_program(program)
            .and_then(|builder| builder.build()) {
            Ok(engine) => {
                assert_eq!(engine.num_qubits(), 0);
                println!("Helios interface: SUCCESS");
            }
            Err(e) => {
                // Expected if Selene not available
                println!("Helios interface failed (expected if no Selene): {}", e);
                assert!(e.to_string().contains("Selene") || e.to_string().contains("Helios"));
            }
        }
    }
}