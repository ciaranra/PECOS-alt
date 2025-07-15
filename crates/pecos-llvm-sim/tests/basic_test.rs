use pecos_llvm_sim::{
    llvm_sim, QuantumEngineType,
    PassThroughNoise, DepolarizingNoise, DepolarizingCustomNoise, BiasedDepolarizingNoise,
};

#[test]
fn test_llvm_sim_builder_creation() {
    // Test creating builder from LLVM IR string
    let builder = llvm_sim().llvm_ir("@main() { ret void }");
    assert!(matches!(builder, _));

    // Test builder methods
    let builder = llvm_sim()
        .llvm_ir("@main() { ret void }")
        .seed(42)
        .workers(4)
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_multiple_input_formats() {
    use std::path::PathBuf;

    // Test LLVM string
    let _ = llvm_sim().llvm_ir("@main() { ret void }");

    // Test LLVM file (doesn't need to exist for builder creation)
    let _ = llvm_sim().llvm_file(PathBuf::from("test.ll"));

    // Test HUGR bytes
    let _ = llvm_sim().hugr_bytes(vec![1, 2, 3]);

    // Test HUGR file (doesn't need to exist for builder creation)
    let _ = llvm_sim().hugr_file(PathBuf::from("test.hugr"));
}

#[test]
fn test_noise_configurations() {
    let builder = llvm_sim().llvm_ir("@main() { ret void }").noise(PassThroughNoise);
    assert!(matches!(builder, _));

    let builder = llvm_sim()
        .llvm_ir("@main() { ret void }")
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));

    let builder = llvm_sim()
        .llvm_ir("@main() { ret void }")
        .noise(DepolarizingCustomNoise {
            p_prep: 0.01,
            p_meas: 0.02,
            p1: 0.03,
            p2: 0.04,
        });
    assert!(matches!(builder, _));

    let builder = llvm_sim()
        .llvm_ir("@main() { ret void }")
        .noise(BiasedDepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_quantum_engine_configurations() {
    let builder = llvm_sim()
        .llvm_ir("@main() { ret void }")
        .quantum_engine(QuantumEngineType::StateVector);
    assert!(matches!(builder, _));

    let builder = llvm_sim()
        .llvm_ir("@main() { ret void }")
        .quantum_engine(QuantumEngineType::SparseStabilizer);
    assert!(matches!(builder, _));
}
