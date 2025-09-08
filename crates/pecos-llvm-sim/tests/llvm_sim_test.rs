//! Tests for the `llvm_sim()` API with full feature parity with `qasm_sim()`

use pecos_engines::{
    BiasedDepolarizingNoise, ClassicalControlEngineBuilder, DepolarizingNoise, sim_builder,
    sparse_stabilizer, state_vector,
};
use pecos_llvm_sim::llvm_engine;
use pecos_programs::LlvmProgram;
use tempfile::NamedTempFile;

mod common;
use common::get_register_i64;

/// Simple LLVM IR for a single qubit Hadamard + measurement
const SIMPLE_HADAMARD_IR: &str = r#"
define void @main() #0 {
%qubit = call i64 @__quantum__rt__qubit_allocate()
call void @__quantum__qis__h__body(i64 %qubit)
%result_id = call i64 @__quantum__rt__result_allocate()
%measurement = call i32 @__quantum__qis__m__body(i64 %qubit, i64 %result_id)
%result_ptr = inttoptr i64 %result_id to i8*
call void @__quantum__rt__result_record_output(i8* %result_ptr, i8* null)
ret void
}

declare i64 @__quantum__rt__qubit_allocate()
declare void @__quantum__qis__h__body(i64)
declare i64 @__quantum__rt__result_allocate()
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i8*, i8*)

attributes #0 = { "EntryPoint" }
"#;

/// Bell state LLVM IR with dynamic qubit allocation
const BELL_STATE_IR: &str = r#"
@str_c0 = constant [3 x i8] c"c0\00"
@str_c1 = constant [3 x i8] c"c1\00"

define void @bell_state() #0 {
%q0 = call i64 @__quantum__rt__qubit_allocate()
%q1 = call i64 @__quantum__rt__qubit_allocate()

call void @__quantum__qis__h__body(i64 %q0)
call void @__quantum__qis__cnot__body(i64 %q0, i64 %q1)

%r0 = call i64 @__quantum__rt__result_allocate()
%m0 = call i32 @__quantum__qis__m__body(i64 %q0, i64 %r0)
%r0_ptr = inttoptr i64 %r0 to i8*
call void @__quantum__rt__result_record_output(i8* %r0_ptr, i8* getelementptr inbounds ([3 x i8], [3 x i8]* @str_c0, i32 0, i32 0))

%r1 = call i64 @__quantum__rt__result_allocate()
%m1 = call i32 @__quantum__qis__m__body(i64 %q1, i64 %r1)
%r1_ptr = inttoptr i64 %r1 to i8*
call void @__quantum__rt__result_record_output(i8* %r1_ptr, i8* getelementptr inbounds ([3 x i8], [3 x i8]* @str_c1, i32 0, i32 0))

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
fn test_basic_llvm_sim() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Basic usage - should work like the simple v2 version
    let shot_vec = llvm_engine()
        .program(LlvmProgram::from_string(SIMPLE_HADAMARD_IR))
        .to_sim()
        .seed(42)
        .qubits(1) // SIMPLE_HADAMARD_IR allocates 1 qubit
        .run(100)
        .expect("Simulation should succeed");

    // Should have 100 shots
    assert_eq!(shot_vec.len(), 100);

    // Convert to ShotMap for columnar access
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");

    // Check that we have some register data
    assert!(!shot_map.register_names().is_empty());

    // Verify we got 100 measurements for each register
    assert_eq!(shot_map.num_shots(), 100);
}

#[test]
fn test_llvm_sim_with_noise() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Run with depolarizing noise
    let shot_vec = llvm_engine()
        .program(LlvmProgram::from_string(BELL_STATE_IR))
        .to_sim()
        .seed(42)
        .workers(2)
        .qubits(2) // BELL_STATE_IR allocates 2 qubits
        .noise(DepolarizingNoise { p: 0.1 }) // 10% error rate
        .run(1000)
        .expect("Simulation with noise should succeed");

    // Convert to ShotMap for columnar access
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");

    // With noise, we should see some non-perfect correlations in Bell state
    // Count the results
    let mut count_00 = 0;
    let mut count_11 = 0;
    let mut count_01 = 0;
    let mut count_10 = 0;

    // Get the measurement results
    let c0_results = get_register_i64(&shot_map, "c0").expect("Should have c0 results");
    let c1_results = get_register_i64(&shot_map, "c1").expect("Should have c1 results");

    for i in 0..1000 {
        let c0 = c0_results[i];
        let c1 = c1_results[i];

        match (c0, c1) {
            (0, 0) => count_00 += 1,
            (1, 1) => count_11 += 1,
            (0, 1) => count_01 += 1,
            (1, 0) => count_10 += 1,
            _ => panic!("Unexpected measurement values"),
        }
    }

    // With 10% noise, we should see some errors (01 and 10 outcomes)
    println!("Bell state with 10% depolarizing noise:");
    println!("  |00⟩: {} ({:.1}%)", count_00, f64::from(count_00) / 10.0);
    println!("  |01⟩: {} ({:.1}%)", count_01, f64::from(count_01) / 10.0);
    println!("  |10⟩: {} ({:.1}%)", count_10, f64::from(count_10) / 10.0);
    println!("  |11⟩: {} ({:.1}%)", count_11, f64::from(count_11) / 10.0);

    // With noise, we should see some errors
    assert!(
        count_01 > 0 || count_10 > 0,
        "Should see some errors with 10% noise"
    );
}

#[test]
fn test_llvm_sim_parallelization() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test with multiple workers
    let start = std::time::Instant::now();
    let results = llvm_engine()
        .program(LlvmProgram::from_string(SIMPLE_HADAMARD_IR))
        .to_sim()
        .seed(42)
        .workers(4) // Use 4 parallel workers
        .run(10000)
        .expect("Parallel simulation should succeed");
    let elapsed = start.elapsed();

    println!(
        "Parallel simulation (4 workers) took: {:.3}s",
        elapsed.as_secs_f64()
    );

    // Should have 10000 results
    assert_eq!(results.len(), 10000);
}

#[test]
fn test_llvm_sim_auto_workers() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test with auto workers
    let shot_vec = llvm_engine()
        .program(LlvmProgram::from_string(SIMPLE_HADAMARD_IR))
        .to_sim()
        .seed(42)
        .auto_workers() // Automatically detect CPU cores
        .run(1000)
        .expect("Auto-worker simulation should succeed");

    // Should have 1000 results
    assert_eq!(shot_vec.len(), 1000);
}

#[test]
fn test_llvm_sim_build_once_run_many() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Build once with qubits set to handle dynamic allocation
    let sim = llvm_engine()
        .program(LlvmProgram::from_string(BELL_STATE_IR))
        .to_sim()
        .seed(42)
        .workers(2)
        .qubits(2) // BELL_STATE_IR allocates 2 qubits
        .noise(DepolarizingNoise { p: 0.01 })
        .verbose(true)
        .build()
        .expect("Build should succeed");

    // Run multiple times
    let mut sim = sim;
    let shot_vec1 = sim.run(100).expect("First run should succeed");
    let shot_vec2 = sim.run(1000).expect("Second run should succeed");
    let shot_vec3 = sim.run(10).expect("Third run should succeed");

    // Check results
    assert_eq!(shot_vec1.len(), 100);
    assert_eq!(shot_vec2.len(), 1000);
    assert_eq!(shot_vec3.len(), 10);

    // MonteCarloEngine doesn't have a stats() method anymore
    // Just verify the runs completed successfully with the expected shot counts
}

// LLVM IR that tries to allocate 3 qubits
const THREE_QUBIT_IR: &str = r#"
define void @main() #0 {
%q0 = call i64 @__quantum__rt__qubit_allocate()
%q1 = call i64 @__quantum__rt__qubit_allocate()
%q2 = call i64 @__quantum__rt__qubit_allocate()
call void @__quantum__qis__h__body(i64 %q0)
call void @__quantum__qis__cx__body(i64 %q0, i64 %q1)
call void @__quantum__qis__cx__body(i64 %q1, i64 %q2)
ret void
}

declare i64 @__quantum__rt__qubit_allocate()
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)

attributes #0 = { "EntryPoint" }
"#;

#[test]
fn test_llvm_sim_qubits_exceeded() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Build with qubits=2 but try to allocate 3
    let sim = llvm_engine()
        .program(LlvmProgram::from_string(THREE_QUBIT_IR))
        .to_sim()
        .seed(42)
        .qubits(2) // Only allow 2 qubits
        .verbose(true) // Enable verbose output to see what's happening
        .build()
        .expect("Build should succeed");

    // Run should fail because we try to use qubits beyond the quantum engine's capacity
    println!("Running simulation that should exceed quantum engine capacity...");
    let mut sim = sim;
    let result = sim.run(1);
    assert!(
        result.is_err(),
        "Should fail when using qubits beyond engine capacity"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("index out of bounds")
            || err_msg.contains("qubit")
            || err_msg.contains("bound")
            || err_msg.contains("limit"),
        "Error should indicate a bounds/qubit issue, got: {err_msg}"
    );
}

#[test]
fn test_llvm_sim_quantum_engines() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test with state vector engine (default)
    let shot_vec_sv = llvm_engine()
        .program(LlvmProgram::from_string(SIMPLE_HADAMARD_IR))
        .to_sim()
        .seed(42)
        .qubits(1)
        .quantum(state_vector())
        .run(100)
        .expect("State vector simulation should succeed");

    // Test with sparse stabilizer engine
    let shot_vec_ss = llvm_engine()
        .program(LlvmProgram::from_string(SIMPLE_HADAMARD_IR))
        .to_sim()
        .seed(42)
        .qubits(1)
        .quantum(sparse_stabilizer())
        .run(100)
        .expect("Sparse stabilizer simulation should succeed");

    // Both should give valid results
    assert_eq!(shot_vec_sv.len(), 100);
    assert_eq!(shot_vec_ss.len(), 100);
}

#[test]
fn test_llvm_sim_custom_noise_models() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test custom depolarizing noise with different error rates
    let shot_vec = llvm_engine()
        .program(LlvmProgram::from_string(BELL_STATE_IR))
        .to_sim()
        .seed(42)
        .noise(DepolarizingNoise { p: 0.02 }) // Use standard depolarizing noise
        .run(1000)
        .expect("Custom noise simulation should succeed");

    assert_eq!(shot_vec.len(), 1000);

    // Test biased depolarizing noise
    let shot_vec_biased = llvm_engine()
        .program(LlvmProgram::from_string(BELL_STATE_IR))
        .to_sim()
        .seed(42)
        .noise(BiasedDepolarizingNoise { p: 0.02 })
        .run(1000)
        .expect("Biased noise simulation should succeed");

    assert_eq!(shot_vec_biased.len(), 1000);
}

#[test]
fn test_llvm_sim_from_file() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Create a temporary file with LLVM IR
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    std::io::Write::write_all(&mut temp_file, SIMPLE_HADAMARD_IR.as_bytes())
        .expect("Failed to write LLVM IR");

    // Test loading from file
    let shot_vec = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_file(temp_file.path()).unwrap()))
        .seed(42)
        .run(100)
        .expect("Simulation from file should succeed");

    assert_eq!(shot_vec.len(), 100);
}

// Note: keep_temp_files functionality is not available in the new unified API
// This test is commented out until/unless this feature is re-added
/*
#[test]
fn test_llvm_sim_keep_temp_files() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test with keep_temp_files option
    let _sim = llvm_engine().program(LlvmProgram::from_string(SIMPLE_HADAMARD_IR)).to_sim()
        .build()
        .expect("Build should succeed");

    // The temp file should be kept after drop
    // (We can't easily test this without accessing private fields,
    // but the feature is implemented)
}
*/

#[test]
fn test_llvm_sim_error_handling() {
    if !is_llvm_available() {
        println!("Skipping test: LLVM tools not available");
        return;
    }

    // Test with invalid LLVM IR
    let invalid_ir = "This is not valid LLVM IR";
    let result = llvm_engine()
        .program(LlvmProgram::from_string(invalid_ir))
        .to_sim()
        .run(100);

    assert!(result.is_err(), "Invalid LLVM IR should fail");

    // Test with LLVM IR that has no entry point
    let no_entry_ir = r"
    define void @not_main() {
        ret void
    }
    ";

    let result = llvm_engine()
        .program(LlvmProgram::from_string(no_entry_ir))
        .to_sim()
        .run(100);
    assert!(result.is_err(), "LLVM IR without entry point should fail");
}
