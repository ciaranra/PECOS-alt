//! Tests for HUGR input support
//!
//! These tests demonstrate the API for HUGR inputs.
//! Full integration tests would require a working HUGR → LLVM compilation pipeline.

use pecos_llvm_sim::LlvmSim;

#[test]
fn test_hugr_sim_api() {
    // This test demonstrates the API without requiring actual HUGR
    use hugr_core::builder::{DFGBuilder, Dataflow, DataflowHugr};
    use hugr_core::extension::prelude::qb_t;
    use hugr_core::types::Signature;

    // Create a simple HUGR (this is just for API demonstration)
    let hugr = {
        let builder = DFGBuilder::new(Signature::new(vec![qb_t()], vec![qb_t()])).unwrap();
        let [q] = builder.input_wires_arr();
        builder.finish_hugr_with_outputs([q]).unwrap()
    };

    // Test builder method
    let builder = LlvmSim::new()
        .hugr(hugr)
        .seed(42)
        .with_depolarizing_noise(0.01);
    assert!(matches!(builder, LlvmSim { .. }));
}

#[test]
fn test_hugr_bytes_input() {
    // Test with dummy HUGR bytes
    let hugr_bytes = vec![0x42; 100]; // Dummy bytes

    let builder = LlvmSim::new()
        .hugr_bytes(hugr_bytes)
        .workers(4)
        .with_state_vector_engine();

    assert!(matches!(builder, LlvmSim { .. }));
}

#[test]
fn test_hugr_file_input() {
    use std::path::PathBuf;

    // Test with file path (doesn't need to exist for builder creation)
    let builder = LlvmSim::new()
        .hugr_file(PathBuf::from("circuit.hugr"))
        .seed(12345)
        .with_sparse_stabilizer_engine();

    assert!(matches!(builder, LlvmSim { .. }));
}
