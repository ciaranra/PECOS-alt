use pecos_llvm_sim::llvm_engine;
use pecos_engines::{
    state_vector, sparse_stabilizer, ClassicalControlEngineBuilder,
    PassThroughNoise, DepolarizingNoise, DepolarizingCustomNoise, BiasedDepolarizingNoise,
};
use pecos_programs::{LlvmProgram, HugrProgram};

#[test]
fn test_llvm_sim_builder_creation() {
    // Test creating builder from LLVM IR string
    let builder = llvm_engine().program(LlvmProgram::from_string("@main() { ret void }")).to_sim();
    assert!(matches!(builder, _));

    // Test builder methods
    let builder = llvm_engine()
        .program(LlvmProgram::from_string("@main() { ret void }"))
        .to_sim()
        .seed(42)
        .workers(4)
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_multiple_input_formats() {
    use std::path::PathBuf;

    // Test LLVM string
    let _ = llvm_engine().program(LlvmProgram::from_string("@main() { ret void }")).to_sim();

    // Test LLVM file (doesn't need to exist for builder creation - but will fail when actually used)
    // let _ = llvm_engine().program(LlvmProgram::from_file("test.ll")).to_sim(); // Would fail
    
    // Test HUGR bytes
    let _ = llvm_engine().program(HugrProgram::from_bytes(vec![1, 2, 3])).to_sim();

    // Test HUGR file (doesn't need to exist for builder creation - but will fail when actually used)  
    // let _ = llvm_engine().program(HugrProgram::from_file("test.hugr")).to_sim(); // Would fail
}

#[test]
fn test_noise_configurations() {
    let builder = llvm_engine().program(LlvmProgram::from_string("@main() { ret void }")).to_sim().noise(PassThroughNoise);
    assert!(matches!(builder, _));

    let builder = llvm_engine()
        .program(LlvmProgram::from_string("@main() { ret void }"))
        .to_sim()
        .noise(DepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));

    let builder = llvm_engine()
        .program(LlvmProgram::from_string("@main() { ret void }"))
        .to_sim()
        .noise(DepolarizingCustomNoise {
            p_prep: 0.01,
            p_meas: 0.02,
            p1: 0.03,
            p2: 0.04,
        });
    assert!(matches!(builder, _));

    let builder = llvm_engine()
        .program(LlvmProgram::from_string("@main() { ret void }"))
        .to_sim()
        .noise(BiasedDepolarizingNoise { p: 0.01 });
    assert!(matches!(builder, _));
}

#[test]
fn test_quantum_engine_configurations() {
    let builder = llvm_engine()
        .program(LlvmProgram::from_string("@main() { ret void }"))
        .to_sim()
        .qubits(1)
        .quantum(state_vector());
    assert!(matches!(builder, _));

    let builder = llvm_engine()
        .program(LlvmProgram::from_string("@main() { ret void }"))
        .to_sim()
        .qubits(1)
        .quantum(sparse_stabilizer());
    assert!(matches!(builder, _));
}
