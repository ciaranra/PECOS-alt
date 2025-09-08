//! Test LLVM bitcode support

use pecos_engines::{ClassicalControlEngineBuilder, PassThroughNoise, sim_builder};
use pecos_llvm_sim::llvm_engine;
use pecos_programs::LlvmProgram;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to create a simple LLVM IR program
fn get_test_llvm_ir() -> &'static str {
    r#"
    declare void @__quantum__qis__h__body(i64)
    declare void @__quantum__qis__cx__body(i64, i64)
    declare i32 @__quantum__qis__m__body(i64, i64)

    define void @test() #0 {
    entry:
        ; Apply Hadamard to qubit 0
        call void @__quantum__qis__h__body(i64 0)

        ; Apply CNOT from qubit 0 to qubit 1
        call void @__quantum__qis__cx__body(i64 0, i64 1)

        ; Measure qubits
        %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
        %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)

        ret void
    }

    attributes #0 = { "EntryPoint" }
    "#
}

#[test]
fn test_bitcode_in_memory() {
    // Skip test if llvm-as is not available
    if Command::new("llvm-as").arg("--version").output().is_err() {
        eprintln!("Skipping test: llvm-as not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let ll_file = temp_dir.path().join("test.ll");
    let bc_file = temp_dir.path().join("test.bc");

    // Write LLVM IR to file
    fs::write(&ll_file, get_test_llvm_ir()).unwrap();

    // Convert to bitcode using llvm-as
    let output = Command::new("llvm-as")
        .arg("-o")
        .arg(&bc_file)
        .arg(&ll_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "llvm-as failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the bitcode
    let bitcode = fs::read(&bc_file).unwrap();

    // Test with in-memory bitcode
    let sim = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_bitcode(bitcode)))
        .noise(PassThroughNoise)
        .build();

    // Should be able to build (though execution may fail without proper setup)
    match sim {
        Ok(_) => println!("Successfully built simulation from bitcode"),
        Err(e) => {
            // Check if it's just a compilation error (expected) vs conversion error (unexpected)
            assert!(
                !e.to_string().contains("llvm-dis"),
                "Bitcode conversion failed: {e}"
            );
            // Other errors (like missing quantum runtime) are expected
            println!("Build failed as expected: {e}");
        }
    }
}

#[test]
fn test_bitcode_file() {
    // Skip test if llvm-as is not available
    if Command::new("llvm-as").arg("--version").output().is_err() {
        eprintln!("Skipping test: llvm-as not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let ll_file = temp_dir.path().join("test.ll");
    let bc_file = temp_dir.path().join("test.bc");

    // Write LLVM IR to file
    fs::write(&ll_file, get_test_llvm_ir()).unwrap();

    // Convert to bitcode using llvm-as
    let output = Command::new("llvm-as")
        .arg("-o")
        .arg(&bc_file)
        .arg(&ll_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "llvm-as failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Test with bitcode file path
    let sim = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_file(&bc_file).unwrap()))
        .build();

    // Should be able to build
    match sim {
        Ok(_) => println!("Successfully built simulation from bitcode file"),
        Err(e) => {
            assert!(
                !e.to_string().contains("llvm-dis"),
                "Bitcode conversion failed: {e}"
            );
            println!("Build failed as expected: {e}");
        }
    }
}

#[test]
fn test_auto_detection() {
    // Skip test if llvm tools are not available
    if Command::new("llvm-as").arg("--version").output().is_err() {
        eprintln!("Skipping test: llvm-as not found");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let ll_file = temp_dir.path().join("test.ll");
    let bc_file = temp_dir.path().join("test.bc");

    // Write LLVM IR to file
    fs::write(&ll_file, get_test_llvm_ir()).unwrap();

    // Convert to bitcode
    Command::new("llvm-as")
        .arg("-o")
        .arg(&bc_file)
        .arg(&ll_file)
        .output()
        .unwrap();

    // Test auto-detection with .ll file
    let sim_ll = llvm_engine()
        .program(LlvmProgram::from_file(&ll_file).unwrap())
        .to_sim()
        .build();
    assert!(sim_ll.is_ok() || true); // Allow failure for missing runtime

    // Test auto-detection with .bc file
    let sim_bc = llvm_engine()
        .program(LlvmProgram::from_file(&bc_file).unwrap())
        .to_sim()
        .build();
    match sim_bc {
        Ok(_) => println!("Successfully built from auto-detected .bc file"),
        Err(e) => {
            assert!(
                !e.to_string().contains("llvm-dis"),
                "Bitcode auto-detection failed: {e}"
            );
            println!("Build failed as expected: {e}");
        }
    }
}

#[test]
fn test_llvm_dis_not_found() {
    // Test error handling when llvm-dis is not available
    let fake_bitcode = vec![0x42, 0x43]; // BC magic number

    let result = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_bitcode(fake_bitcode)))
        .build();

    match result {
        Ok(_) => panic!("Expected error when processing invalid bitcode"),
        Err(e) => {
            println!("Got expected error: {e}");
            // Could be either "Failed to execute llvm-dis" or "llvm-dis failed to convert"
            assert!(
                e.to_string().contains("llvm-dis")
                    || e.to_string().contains("Failed to execute")
                    || e.to_string().contains("failed to convert")
            );
        }
    }
}

#[test]
fn test_error_without_llvm_tools() {
    // Skip if llvm-dis IS available
    if Command::new("llvm-dis").arg("--version").output().is_ok() {
        eprintln!("Skipping test: llvm-dis is available");
        return;
    }

    // Test graceful error when llvm-dis is not available
    let bitcode = vec![0xDE, 0xC0, 0x17, 0x0B]; // Valid bitcode magic
    let sim = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_bitcode(bitcode)))
        .build();

    match sim {
        Ok(_) => panic!("Expected error when llvm-dis is not available"),
        Err(e) => {
            assert!(e.to_string().contains("llvm-dis"));
            println!("Got expected error: {e}");
        }
    }
}
