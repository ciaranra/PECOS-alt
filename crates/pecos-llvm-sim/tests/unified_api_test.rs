//! Test the unified API llvm_engine().to_sim()

use pecos_llvm_sim::engine_builder::llvm_engine;
use pecos_engines::{ClassicalControlEngineBuilder, DepolarizingNoise};

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
    let shot_vec = llvm_engine()
        .llvm_ir(SIMPLE_IR)
        .to_sim()
        .seed(42)
        .run(100)
        .expect("Unified API should work");
    
    assert_eq!(shot_vec.len(), 100);
    
    // Convert to ShotMap for analysis
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let registers = shot_map.register_names();
    
    // Should have register 'c'
    assert!(registers.iter().any(|r| *r == "c"));
    assert_eq!(shot_map.num_shots(), 100);
}

#[test]
fn test_unified_api_with_noise() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test the unified API with noise
    let shot_vec = llvm_engine()
        .llvm_ir(SIMPLE_IR)
        .to_sim()
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 })
        .run(100)
        .expect("Unified API with noise should work");
    
    assert_eq!(shot_vec.len(), 100);
    
    // Convert to ShotMap for analysis
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    
    // Should have register 'c'
    assert!(shot_map.register_names().iter().any(|r| *r == "c"));
    assert_eq!(shot_map.num_shots(), 100);
}

#[test]
fn test_unified_api_parity_with_llvm_sim() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    let seed = 42;
    let shots = 50;

    // Test with llvm_sim()
    let shot_vec_sim = pecos_llvm_sim::llvm_sim()
        .llvm_ir(SIMPLE_IR)
        .seed(seed)
        .workers(1) // Single worker for determinism
        .run(shots)
        .expect("llvm_sim should work");

    // Test with llvm_engine().to_sim()
    let shot_vec_unified = llvm_engine()
        .llvm_ir(SIMPLE_IR)
        .to_sim()
        .seed(seed)
        .workers(1) // Single worker for determinism
        .run(shots)
        .expect("Unified API should work");
    
    // Both should have same number of shots
    assert_eq!(shot_vec_sim.len(), shots);
    assert_eq!(shot_vec_unified.len(), shots);
    
    // Convert to ShotMaps
    let shot_map_sim = shot_vec_sim.try_as_shot_map().expect("Should convert sim");
    let shot_map_unified = shot_vec_unified.try_as_shot_map().expect("Should convert unified");
    
    // Both should have 'c' register
    assert!(shot_map_sim.register_names().iter().any(|r| *r == "c"));
    assert!(shot_map_unified.register_names().iter().any(|r| *r == "c"));
    
    // Both should have same number of shots
    assert_eq!(shot_map_sim.num_shots(), shots);
    assert_eq!(shot_map_unified.num_shots(), shots);
}