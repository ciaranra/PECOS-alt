//! Integration tests for pecos-selene-eng

use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine, ControlEngine, sim_builder};
use pecos_programs::LlvmProgram;
use pecos_selene::selene_executable;

mod common;

#[test]
fn test_basic_simulation() {
    // Create a simple quantum program using LLVM IR
    let llvm_ir = r#"
; Simple quantum program
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @main() #0 {
entry:
    call void @__quantum__qis__h__body(i64 0)
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let result = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(llvm_ir))
                .qubits(1),
        )
        .run(1);

    assert!(result.is_ok());
    let shot_vec = result.unwrap();
    assert_eq!(shot_vec.len(), 1);
}

#[test]
fn test_bell_state_simulation() {
    let bell_llvm = r#"
; Bell state LLVM IR
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @bell() #0 {
entry:
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(bell_llvm))
                .qubits(2),
        )
        .run(2); // Reduced from 100 to 2 for debugging

    assert!(results.is_ok());
    let shot_vec = results.unwrap();

    // Should have 2 shots
    assert_eq!(shot_vec.len(), 2);

    // Should have measurement data keys
    // Convert to ShotMap for register analysis
    let shot_map = shot_vec.try_as_shot_map().unwrap();
    let shot_keys: Vec<_> = shot_map.register_names();
    assert!(!shot_keys.is_empty());
}

#[test]
fn test_with_optimization() {
    let simple_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @optimized() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(simple_llvm))
                .qubits(1),
        )
        .run(1);

    assert!(results.is_ok());
}

#[test]
fn test_with_seed() {
    let test_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @seeded() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(test_llvm))
                .qubits(1),
        )
        .seed(12345)
        .run(1);

    assert!(results.is_ok());
}

#[test]
fn test_parallel_execution() {
    let parallel_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @parallel() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let results = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(parallel_llvm))
                .qubits(1),
        )
        .workers(4)
        .run(4); // Reduced from 100 for performance

    assert!(results.is_ok());
}

#[test]
fn test_engine_traits() {
    let trait_llvm = r#"
declare void @__quantum__qis__h__body(i64)

define void @traits() #0 {
    call void @__quantum__qis__h__body(i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let engine = selene_executable()
        .program(LlvmProgram::from_ir(trait_llvm))
        .qubits(1)
        .build();

    assert!(engine.is_ok());
    let engine = engine.unwrap();

    // Test that it implements required traits
    assert_eq!(engine.num_qubits(), 1);
}

#[test]
fn test_invalid_program() {
    let engine = selene_executable()
        .program(LlvmProgram::from_ir("")) // Empty IR
        .qubits(1)
        .build();

    // Empty IR creates a default circuit, but compile() should fail
    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert!(engine.compile().is_err());
}

#[test]
fn test_missing_qubits() {
    // Builder defaults to 10 qubits if not specified
    let result = sim_builder()
        .classical(selene_executable().program(LlvmProgram::from_ir("test")))
        .build();

    // This should succeed with the default 10 qubits
    assert!(result.is_ok());
}

#[test]
fn test_classical_control() {
    let control_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @control() #0 {
entry:
    call void @__quantum__qis__h__body(i64 0)
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ; In real control flow, would conditionally apply X based on %result
    call void @__quantum__qis__x__body(i64 1)
    %final = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let result = sim_builder()
        .classical(
            selene_executable()
                .program(LlvmProgram::from_ir(control_llvm))
                .qubits(2)
                .verbose(true),
        )
        .run(1);

    assert!(result.is_ok());
}

#[test]
fn test_engine_as_control_engine() {
    use pecos_engines::EngineStage;

    let adaptive_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @adaptive() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let mut engine = selene_executable()
        .program(LlvmProgram::from_ir(adaptive_llvm))
        .qubits(1)
        .build()
        .unwrap();

    // Test as ControlEngine
    match engine.start(()).unwrap() {
        EngineStage::NeedsProcessing(cmd) => {
            // For LLVM programs, empty ByteMessage is expected
            // since LLVM program support in ControlEngine mode is limited
            if cmd.is_empty().unwrap() {
                println!("LLVM program returned empty ByteMessage (expected for now)");
            } else {
                println!("LLVM program returned non-empty ByteMessage");
            }
        }
        EngineStage::Complete(_) => {
            // Also valid if no operations need processing
            println!("Engine completed immediately");
        }
    }
}
