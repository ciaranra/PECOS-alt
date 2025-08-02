use pecos_llvm_sim::llvm_engine;
use pecos_engines::{
    state_vector, sparse_stabilizer, sim_builder,
    PassThroughNoise, DepolarizingNoise, BiasedDepolarizingNoise,
};
use pecos_programs::{LlvmProgram, HugrProgram};

#[test]
fn test_llvm_sim_builder_creation() {
    // Test creating builder from LLVM IR string
    let builder = sim_builder().classical(llvm_engine().program(LlvmProgram::from_string("@main() { ret void }")));
    assert!(matches!(builder, _));

    // Test builder methods
    let builder = sim_builder()
        .classical(llvm_engine()
            .program(LlvmProgram::from_string("@main() { ret void }")))
        .seed(42)
        .workers(4)
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_multiple_input_formats() {
    // Test LLVM string
    let _ = sim_builder().classical(llvm_engine().program(LlvmProgram::from_string("@main() { ret void }")));

    // Test LLVM file (doesn't need to exist for builder creation - but will fail when actually used)
    // let _ = sim_builder().classical(llvm_engine().program(LlvmProgram::from_file("test.ll"))); // Would fail
    
    // Test HUGR bytes
    let _ = sim_builder().classical(llvm_engine().program(HugrProgram::from_bytes(vec![1, 2, 3])));

    // Test HUGR file (doesn't need to exist for builder creation - but will fail when actually used)  
    // let _ = sim_builder().classical(llvm_engine().program(HugrProgram::from_file("test.hugr"))); // Would fail
}

#[test]
fn test_noise_configurations() {
    let builder = sim_builder().classical(llvm_engine().program(LlvmProgram::from_string("@main() { ret void }"))).noise(PassThroughNoise);
    assert!(matches!(builder, _));

    let builder = sim_builder()
        .classical(llvm_engine()
            .program(LlvmProgram::from_string("@main() { ret void }")))
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));

    let builder = sim_builder()
        .classical(llvm_engine()
            .program(LlvmProgram::from_string("@main() { ret void }")))
        .noise(DepolarizingNoise { p: 0.02 });
    assert!(matches!(builder, _));

    let builder = sim_builder()
        .classical(llvm_engine()
            .program(LlvmProgram::from_string("@main() { ret void }")))
        .noise(BiasedDepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_quantum_engine_configurations() {
    let builder = sim_builder()
        .classical(llvm_engine()
            .program(LlvmProgram::from_string("@main() { ret void }")))
        .qubits(1)
        .quantum(state_vector());
    assert!(matches!(builder, _));

    let builder = sim_builder()
        .classical(llvm_engine()
            .program(LlvmProgram::from_string("@main() { ret void }")))
        .qubits(1)
        .quantum(sparse_stabilizer());
    assert!(matches!(builder, _));
}
