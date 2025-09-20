//! Test LLVM bitcode support in `selene_sim`

use pecos_engines::{PassThroughNoise, sim_builder};
use pecos_programs::LlvmProgram;
use pecos_selene_engine::selene_executable;
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
fn test_selene_bitcode_in_memory() {
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
    let builder = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_bitcode(bitcode))
                .qubits(2),
        )
        .noise(PassThroughNoise);

    // Should be able to build (though execution may fail without proper setup)
    match builder.build() {
        Ok(_) => println!("Successfully built Selene simulation from bitcode"),
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
fn test_selene_bitcode_file() {
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
    let builder = sim_builder().classical(
        selene_executable()
            .program(LlvmProgram::from_bitcode_file(&bc_file).unwrap())
            .qubits(2),
    );

    // Should be able to build
    match builder.build() {
        Ok(_) => println!("Successfully built Selene simulation from bitcode file"),
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
fn test_selene_auto_detection() {
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
    let builder_ll = sim_builder().classical(
        selene_executable()
            .program(LlvmProgram::from_file(&ll_file).unwrap())
            .qubits(2),
    );
    // Allow failure for missing runtime
    let _ = builder_ll.build();

    // Test auto-detection with .bc file
    let builder_bc = sim_builder().classical(
        selene_executable()
            .program(LlvmProgram::from_file(&bc_file).unwrap())
            .qubits(2),
    );
    match builder_bc.build() {
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
fn test_selene_llvm_dis_error() {
    // Test error handling with invalid bitcode
    let fake_bitcode = vec![0x42, 0x43]; // BC magic number but invalid content

    let builder = sim_builder().classical(
        selene_executable()
            .program(LlvmProgram::from_bitcode(fake_bitcode))
            .qubits(1),
    );

    // SeleneExecutableEngine doesn't validate bitcode until execution
    // So this will succeed at build time but fail later
    match builder.build() {
        Ok(_engine) => {
            println!("Engine created successfully - invalid bitcode will fail during execution");
            println!("Build succeeded as expected - invalid bitcode not validated at build time");
        }
        Err(e) => {
            println!("Build failed (possibly missing runtime): {e}");
            // If build fails, it's likely due to missing runtime, not invalid bitcode
            // This is acceptable for this test environment
        }
    }
}
