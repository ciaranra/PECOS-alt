use pecos_llvm_sim::LlvmSim;

#[test]
fn test_llvm_sim_builder_creation() {
    // Test creating builder from LLVM IR string
    let builder = LlvmSim::new().llvm("@main() { ret void }");
    assert!(matches!(builder, LlvmSim { .. }));

    // Test builder methods
    let builder = LlvmSim::new()
        .llvm("@main() { ret void }")
        .seed(42)
        .workers(4)
        .with_depolarizing_noise(0.01);
    assert!(matches!(builder, LlvmSim { .. }));
}

#[test]
fn test_multiple_input_formats() {
    use std::path::PathBuf;

    // Test LLVM string
    let _ = LlvmSim::new().llvm("@main() { ret void }");

    // Test LLVM file (doesn't need to exist for builder creation)
    let _ = LlvmSim::new().llvm_file(PathBuf::from("test.ll"));

    // Test HUGR bytes
    let _ = LlvmSim::new().hugr_bytes(vec![1, 2, 3]);

    // Test HUGR file (doesn't need to exist for builder creation)
    let _ = LlvmSim::new().hugr_file(PathBuf::from("test.hugr"));
}

#[test]
fn test_noise_configurations() {
    let builder = LlvmSim::new().llvm("@main() { ret void }").with_no_noise();
    assert!(matches!(builder, LlvmSim { .. }));

    let builder = LlvmSim::new()
        .llvm("@main() { ret void }")
        .with_depolarizing_noise(0.01);
    assert!(matches!(builder, LlvmSim { .. }));

    let builder = LlvmSim::new()
        .llvm("@main() { ret void }")
        .with_custom_depolarizing_noise(0.01, 0.02, 0.03, 0.04);
    assert!(matches!(builder, LlvmSim { .. }));

    let builder = LlvmSim::new()
        .llvm("@main() { ret void }")
        .with_biased_depolarizing_noise(0.01);
    assert!(matches!(builder, LlvmSim { .. }));
}

#[test]
fn test_quantum_engine_configurations() {
    let builder = LlvmSim::new()
        .llvm("@main() { ret void }")
        .with_state_vector_engine();
    assert!(matches!(builder, LlvmSim { .. }));

    let builder = LlvmSim::new()
        .llvm("@main() { ret void }")
        .with_sparse_stabilizer_engine();
    assert!(matches!(builder, LlvmSim { .. }));
}
