//! Test the new consistent LLVM format support

use pecos_engines::{PassThroughNoise, sim_builder};
use pecos_qis_sim::qis_engine;
use pecos_programs::QisProgram;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_qis_ir_text_format() {
    let llvm_ir = r#"
    declare void @__quantum__qis__h__body(i64)

    define void @test() #0 {
        call void @__quantum__qis__h__body(i64 0)
        ret void
    }

    attributes #0 = { "EntryPoint" }
    "#;

    // Test with in-memory LLVM IR text
    let sim = sim_builder()
        .classical(qis_engine().program(QisProgram::from_ir(llvm_ir)))
        .noise(PassThroughNoise)
        .build();

    assert!(sim.is_ok());
}

#[test]
fn test_qis_file_auto_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create .ll file
    let ll_file = temp_dir.path().join("test.ll");
    fs::write(&ll_file, "define void @main() { ret void }").unwrap();

    // Test auto-detection of .ll file
    let sim = sim_builder()
        .classical(qis_engine().program(QisProgram::from_file(&ll_file).unwrap()))
        .build();

    // Should succeed (though actual compilation may fail without proper LLVM IR)
    // We expect this to succeed at the builder level
    assert!(sim.is_ok());
}

#[test]
fn test_qis_ir_file_explicit() {
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
    let sim = sim_builder()
        .classical(qis_engine().program(QisProgram::from_file(&ll_file).unwrap()))
        .build();

    assert!(sim.is_ok());
}
