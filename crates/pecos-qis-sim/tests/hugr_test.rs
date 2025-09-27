//! Tests for LLVM program input
//!
//! Note: Direct HUGR support has been removed. HUGR compilation now uses
//! tket's HUGR 0.22 through the pecos-hugr-qis crate.

use pecos_engines::{DepolarizingNoise, sim_builder, sparse_stabilizer, state_vector};
use pecos_programs::QisProgram;
use pecos_qis_sim::qis_engine;

#[test]
fn test_qis_sim_api() {
    // Test with LLVM IR input
    let llvm_ir = r"
        ; ModuleID = 'test'
        declare i8* @__pecos__new_array(i64)
        declare void @__pecos__end_array(i8*)

        define void @main() {
            %q = call i8* @__pecos__new_array(i64 1)
            call void @__pecos__end_array(i8* %q)
            ret void
        }
    ";

    // Test builder method with LLVM program
    let builder = sim_builder()
        .classical(qis_engine().program(QisProgram::from_ir(llvm_ir)))
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_qis_bytes_input() {
    // Test with LLVM bitcode bytes
    let llvm_bytes = vec![0x42; 100]; // Dummy bytes for API testing

    let builder = sim_builder()
        .classical(qis_engine().program(QisProgram::from_bitcode(llvm_bytes)))
        .workers(4)
        .quantum(state_vector());
    assert!(matches!(builder, _));
}

#[test]
fn test_hugr_compilation_note() {
    println!("Note: HUGR compilation has moved to the pecos-hugr-qis crate");
    println!("To compile HUGR to LLVM:");
    println!("1. Use pecos_hugr_qis::compile_hugr_bytes_to_string() in Rust");
    println!("2. Use pecos_rslib.compile_hugr_to_llvm_rust() in Python");
    println!("3. Feed the resulting LLVM IR to QisProgram::from_ir()");
}

#[test]
fn test_program_with_different_quantum_backends() {
    let llvm_ir = r"
        define void @main() { ret void }
    ";

    // Test with state vector backend
    let _sv_builder = sim_builder()
        .classical(qis_engine().program(QisProgram::from_ir(llvm_ir)))
        .quantum(state_vector());

    // Test with sparse stabilizer backend
    let _stab_builder = sim_builder()
        .classical(qis_engine().program(QisProgram::from_ir(llvm_ir)))
        .quantum(sparse_stabilizer());
}
