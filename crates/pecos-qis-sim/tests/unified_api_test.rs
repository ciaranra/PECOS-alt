//! Test the unified API `sim_builder().classical(qis_engine())`

use pecos_engines::{DepolarizingNoise, sim_builder};
use pecos_qis_sim::qis_engine;
use pecos_programs::QisProgram;

const SIMPLE_IR: &str = r#"
@str_c = constant [2 x i8] c"c\00"

define void @main() #0 {
%q0 = call i64 @__quantum__rt__qubit_allocate()
%q1 = call i64 @__quantum__rt__qubit_allocate()

call void @__quantum__qis__h__body(i64 %q0)
call void @__quantum__qis__cnot__body(i64 %q0, i64 %q1)

%r0 = call i64 @__quantum__rt__result_allocate()
%m0 = call i32 @__quantum__qis__m__body(i64 %q0, i64 %r0)
%r1 = call i64 @__quantum__rt__result_allocate()
%m1 = call i32 @__quantum__qis__m__body(i64 %q1, i64 %r1)

%r_ptr = inttoptr i64 %r0 to i8*
call void @__quantum__rt__result_record_output(i8* %r_ptr, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @str_c, i32 0, i32 0))

ret void
}

declare i64 @__quantum__rt__qubit_allocate()
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cnot__body(i64, i64)
declare i64 @__quantum__rt__result_allocate()
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i8*, i8*)

attributes #0 = { "EntryPoint" }
"#;

/// Check if LLVM tools are available
fn is_llvm_available() -> bool {
    if cfg!(windows) {
        std::env::var("PATH")
            .map(|paths| {
                paths
                    .split(';')
                    .any(|dir| std::path::Path::new(dir).join("clang.exe").exists())
            })
            .unwrap_or(false)
    } else {
        std::env::var("PATH")
            .map(|paths| {
                paths
                    .split(':')
                    .any(|dir| std::path::Path::new(dir).join("llc").exists())
            })
            .unwrap_or(false)
    }
}

#[test]
fn test_unified_api_basic() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test the unified API
    let shot_vec = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(SIMPLE_IR)))
        .seed(42)
        .qubits(2)
        .run(100)
        .expect("Unified API should work");

    assert_eq!(shot_vec.len(), 100);

    // Convert to ShotMap for analysis
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let registers = shot_map.register_names();

    // Should have register 'c'
    assert!(registers.contains(&"c"));
    assert_eq!(shot_map.num_shots(), 100);
}

#[test]
fn test_unified_api_with_noise() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test the unified API with noise
    let shot_vec = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(SIMPLE_IR)))
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 })
        .qubits(2)
        .run(100)
        .expect("Unified API with noise should work");

    assert_eq!(shot_vec.len(), 100);

    // Convert to ShotMap for analysis
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");

    // Should have register 'c'
    assert!(shot_map.register_names().contains(&"c"));
    assert_eq!(shot_map.num_shots(), 100);
}

#[test]
fn test_unified_api_deterministic_behavior() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    let seed = 42;
    let shots = 50;

    // Test with first builder instance
    let shot_vec_sim = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(SIMPLE_IR)))
        .seed(seed)
        .workers(1) // Single worker for determinism
        .qubits(2)
        .run(shots)
        .expect("First instance should work");

    // Test with second builder instance for comparison
    let shot_vec_unified = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(SIMPLE_IR)))
        .seed(seed)
        .workers(1) // Single worker for determinism
        .qubits(2)
        .run(shots)
        .expect("Second instance should work");

    // Both should have same number of shots
    assert_eq!(shot_vec_sim.len(), shots);
    assert_eq!(shot_vec_unified.len(), shots);

    // Convert to ShotMaps
    let shot_map_first = shot_vec_sim
        .try_as_shot_map()
        .expect("Should convert first");
    let shot_map_second = shot_vec_unified
        .try_as_shot_map()
        .expect("Should convert second");

    // Both should have 'c' register
    assert!(shot_map_first.register_names().contains(&"c"));
    assert!(shot_map_second.register_names().contains(&"c"));

    // Both should have same number of shots
    assert_eq!(shot_map_first.num_shots(), shots);
    assert_eq!(shot_map_second.num_shots(), shots);
}
