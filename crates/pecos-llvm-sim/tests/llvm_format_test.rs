//! Test the new consistent LLVM format support

use pecos_llvm_sim::llvm_engine;
use pecos_engines::{ClassicalControlEngineBuilder, PassThroughNoise};
use pecos_programs::LlvmProgram;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_llvm_ir_text_format() {
    let llvm_ir = r#"
    declare void @__quantum__qis__h__body(i64)
    
    define void @test() #0 {
        call void @__quantum__qis__h__body(i64 0)
        ret void
    }
    
    attributes #0 = { "EntryPoint" }
    "#;
    
    // Test with in-memory LLVM IR text
    let sim = llvm_engine()
        .program(LlvmProgram::from_ir(llvm_ir))
        .to_sim()
        .noise(PassThroughNoise)
        .build();
    
    assert!(sim.is_ok());
}

#[test]
fn test_llvm_file_auto_detection() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create .ll file
    let ll_file = temp_dir.path().join("test.ll");
    fs::write(&ll_file, "define void @main() { ret void }").unwrap();
    
    // Test auto-detection of .ll file
    let sim = llvm_engine()
        .program(LlvmProgram::from_file(&ll_file).unwrap())
        .to_sim()
        .build();
    
    // Should succeed (though actual compilation may fail without proper LLVM IR)
    // We expect this to succeed at the builder level
    assert!(sim.is_ok());
}

#[test]
fn test_llvm_ir_file_explicit() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create .ll file
    let ll_file = temp_dir.path().join("circuit.ll");
    let llvm_ir = r#"
    define void @main() #0 {
        ret void
    }
    attributes #0 = { "EntryPoint" }
    "#;
    fs::write(&ll_file, llvm_ir).unwrap();
    
    // Test explicit .ll file loading
    let sim = llvm_engine()
        .program(LlvmProgram::from_file(&ll_file).unwrap())
        .to_sim()
        .build();
    
    assert!(sim.is_ok());
}

