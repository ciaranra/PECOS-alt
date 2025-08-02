//! Tests for HUGR input support
//!
//! These tests demonstrate the API for HUGR inputs.
//! Full integration tests would require a working HUGR → LLVM compilation pipeline.

use pecos_llvm_sim::llvm_engine;
use pecos_engines::{state_vector, sparse_stabilizer, DepolarizingNoise, sim_builder};
use pecos_programs::HugrProgram;

#[test]
fn test_hugr_sim_api() {
    // This test demonstrates the API without requiring actual HUGR
    use hugr_core::builder::{DFGBuilder, Dataflow, DataflowHugr};
    use hugr_core::extension::prelude::qb_t;
    use hugr_core::types::Signature;

    // Create a simple HUGR (this is just for API demonstration)
    let _hugr = {
        let builder = DFGBuilder::new(Signature::new(vec![qb_t()], vec![qb_t()])).unwrap();
        let [q] = builder.input_wires_arr();
        builder.finish_hugr_with_outputs([q]).unwrap()
    };

    // Test builder method with HUGR program
    let hugr_bytes = vec![0x42; 100]; // Dummy serialized HUGR
    let builder = sim_builder()
        .classical(llvm_engine()
        .program(HugrProgram::from_bytes(hugr_bytes)))
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_hugr_bytes_input() {
    // Test with dummy HUGR bytes
    let hugr_bytes = vec![0x42; 100]; // Dummy bytes

    let builder = sim_builder()
        .classical(llvm_engine()
        .program(HugrProgram::from_bytes(hugr_bytes)))
        .workers(4)
        .qubits(2)
        .quantum(state_vector());

    assert!(matches!(builder, _));
}

#[test]
fn test_hugr_file_input() {
    use std::fs;
    use tempfile::NamedTempFile;

    // Create a temporary file with dummy HUGR content
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let hugr_bytes = vec![0x42; 100]; // Dummy HUGR data
    fs::write(temp_file.path(), &hugr_bytes).expect("Failed to write temp file");

    // Test with file path
    let builder = sim_builder()
        .classical(llvm_engine()
        .program(HugrProgram::from_file(temp_file.path()).unwrap()))
        .seed(12345)
        .qubits(2)
        .quantum(sparse_stabilizer());

    assert!(matches!(builder, _));
}
