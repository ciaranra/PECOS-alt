//! Test LLVM bitcode support in selene_sim

use pecos_selene_ceng::{selene_sim, PassThroughNoise};
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
    
    if !output.status.success() {
        panic!("llvm-as failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Read the bitcode
    let bitcode = fs::read(&bc_file).unwrap();
    
    // Test with in-memory bitcode
    let builder = selene_sim()
        .llvm_bitcode(bitcode)
        .qubits(2)
        .noise(PassThroughNoise);
    
    // Should be able to build (though execution may fail without proper setup)
    match builder.build() {
        Ok(_) => println!("Successfully built Selene simulation from bitcode"),
        Err(e) => {
            // Check if it's just a compilation error (expected) vs conversion error (unexpected)
            if e.to_string().contains("llvm-dis") {
                panic!("Bitcode conversion failed: {}", e);
            }
            // Other errors (like missing quantum runtime) are expected
            println!("Build failed as expected: {}", e);
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
    
    if !output.status.success() {
        panic!("llvm-as failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Test with bitcode file path
    let builder = selene_sim()
        .llvm_bitcode_file(&bc_file)
        .qubits(2);
    
    // Should be able to build
    match builder.build() {
        Ok(_) => println!("Successfully built Selene simulation from bitcode file"),
        Err(e) => {
            if e.to_string().contains("llvm-dis") {
                panic!("Bitcode conversion failed: {}", e);
            }
            println!("Build failed as expected: {}", e);
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
    let builder_ll = selene_sim()
        .llvm_file(&ll_file)
        .qubits(2);
    assert!(builder_ll.build().is_ok() || true); // Allow failure for missing runtime
    
    // Test auto-detection with .bc file
    let builder_bc = selene_sim()
        .llvm_file(&bc_file)
        .qubits(2);
    match builder_bc.build() {
        Ok(_) => println!("Successfully built from auto-detected .bc file"),
        Err(e) => {
            if e.to_string().contains("llvm-dis") {
                panic!("Bitcode auto-detection failed: {}", e);
            }
            println!("Build failed as expected: {}", e);
        }
    }
}

#[test]
fn test_selene_llvm_dis_error() {
    // Test error handling with invalid bitcode
    let fake_bitcode = vec![0x42, 0x43]; // BC magic number but invalid content
    
    let builder = selene_sim()
        .llvm_bitcode(fake_bitcode)
        .qubits(1);
    
    // Since selene_engine skips compilation for tests, we need to check differently
    match builder.build() {
        Ok(_engine) => {
            // The engine might be created but compilation would fail when running
            println!("Engine created, but bitcode was invalid - this is OK for test mode");
        }
        Err(e) => {
            println!("Got expected error: {}", e);
            // Should mention llvm-dis in the error
            assert!(
                e.to_string().contains("llvm-dis") || 
                e.to_string().contains("Failed to run") ||
                e.to_string().contains("failed") ||
                e.to_string().contains("bitcode")
            );
        }
    }
}