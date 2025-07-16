//! Test the ergonomic noise API for selene_sim()

use pecos_selene_ceng::{
    selene_sim, 
    NoiseModelConfig, QuantumEngineType,
    PassThroughNoise, DepolarizingNoise, DepolarizingCustomNoise, BiasedDepolarizingNoise,
};
use pecos_engines::noise::GeneralNoiseModelBuilder;

mod common;

#[test]
fn test_noise_method_with_structs() {
    let llvm_ir = r#"
    declare void @__quantum__qis__h__body(i64)
    declare i32 @__quantum__qis__m__body(i64, i64)
    
    define void @test() #0 {
        call void @__quantum__qis__h__body(i64 0)
        %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
        ret void
    }
    
    attributes #0 = { "EntryPoint" }
    "#;
    
    // Test with PassThroughNoise struct
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(PassThroughNoise)
        .build()
        .unwrap();
    
    // Test with DepolarizingNoise struct
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(DepolarizingNoise { p: 0.01 })
        .build()
        .unwrap();
    
    // Test with DepolarizingCustomNoise struct
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(DepolarizingCustomNoise {
            p_prep: 0.001,
            p_meas: 0.002,
            p1: 0.003,
            p2: 0.004,
        })
        .build()
        .unwrap();
    
    // Test with BiasedDepolarizingNoise struct
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(BiasedDepolarizingNoise { p: 0.01 })
        .build()
        .unwrap();
    
    // Test with GeneralNoiseModelBuilder
    let general = GeneralNoiseModelBuilder::new()
        .with_p1_probability(0.001)
        .with_p2_probability(0.002);
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(general)
        .build()
        .unwrap();
}

#[test]
fn test_noise_method_with_enum() {
    let llvm_ir = r#"
    declare void @__quantum__qis__x__body(i64)
    
    define void @test() #0 {
        call void @__quantum__qis__x__body(i64 0)
        ret void
    }
    
    attributes #0 = { "EntryPoint" }
    "#;
    
    // Can still use the enum directly
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(NoiseModelConfig::Depolarizing(0.01))
        .build()
        .unwrap();
    
    let _sim = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .noise(NoiseModelConfig::DepolarizingCustom {
            p_prep: 0.001,
            p_meas: 0.002,
            p1: 0.003,
            p2: 0.004,
        })
        .build()
        .unwrap();
}

#[test]
fn test_noise_api_matches_qasm_sim() {
    // This test demonstrates that selene_sim() has the same noise API as qasm_sim()
    let llvm_ir = r#"
    declare void @__quantum__qis__h__body(i64)
    
    define void @test() #0 {
        call void @__quantum__qis__h__body(i64 0)
        ret void
    }
    
    attributes #0 = { "EntryPoint" }
    "#;
    
    // The .noise() method accepts structs just like qasm_sim()
    let results = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 })
        .run(100)
        .unwrap();
    
    assert_eq!(results.len(), 100);
    
    // Can also chain with other methods
    let results = selene_sim()
        .llvm_ir(llvm_ir)
        .qubits(1)
        .workers(2)
        .noise(BiasedDepolarizingNoise { p: 0.005 })
        .quantum_engine(QuantumEngineType::SparseStabilizer)
        .run(50)
        .unwrap();
    
    assert_eq!(results.len(), 50);
}