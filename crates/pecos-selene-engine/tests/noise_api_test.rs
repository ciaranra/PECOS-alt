//! Test the ergonomic noise API for `selene_executable()`

use pecos_engines::noise::GeneralNoiseModelBuilder;
use pecos_engines::{
    BiasedDepolarizingNoise, DepolarizingNoise, PassThroughNoise, sim_builder, sparse_stabilizer,
};
use pecos_programs::QisProgram;
use pecos_selene_engine::selene_executable;

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
    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .noise(PassThroughNoise)
        .build()
        .unwrap();

    // Test with DepolarizingNoise struct
    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .noise(DepolarizingNoise { p: 0.01 })
        .build()
        .unwrap();

    // Test with DepolarizingCustomNoise struct
    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .noise(DepolarizingNoise { p: 0.002 })
        .build()
        .unwrap();

    // Test with BiasedDepolarizingNoise struct
    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .noise(BiasedDepolarizingNoise { p: 0.01 })
        .build()
        .unwrap();

    // Test with GeneralNoiseModelBuilder
    let general = GeneralNoiseModelBuilder::new()
        .with_p1_probability(0.001)
        .with_p2_probability(0.002);
    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
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

    // Use noise structs directly
    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .noise(DepolarizingNoise { p: 0.01 })
        .build()
        .unwrap();

    let _sim = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .noise(DepolarizingNoise { p: 0.002 })
        .build()
        .unwrap();
}

#[test]
#[ignore = "Legacy test - LLVM execution removed. Use Guppy->HUGR->Selene path"]
fn test_noise_api_matches_qasm_sim() {
    // This test demonstrates that selene_executable() has the same noise API as qasm_sim()
    let llvm_ir = r#"
    declare void @__quantum__qis__h__body(i64)

    define void @test() #0 {
        call void @__quantum__qis__h__body(i64 0)
        ret void
    }

    attributes #0 = { "EntryPoint" }
    "#;

    // The .noise() method accepts structs just like qasm_sim()
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 })
        .run(100)
        .unwrap();

    assert_eq!(results.len(), 100);

    // Can also chain with other methods
    let results = sim_builder()
        .classical(
            selene_executable()
                .program(QisProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .workers(2)
        .noise(BiasedDepolarizingNoise { p: 0.005 })
        .quantum(sparse_stabilizer())
        .run(50)
        .unwrap();

    assert_eq!(results.len(), 50);
}
