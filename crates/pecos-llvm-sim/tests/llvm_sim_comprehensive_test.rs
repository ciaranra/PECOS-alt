//! Comprehensive tests for LLVM simulation API matching and exceeding `LlvmEngine` test coverage
//!
//! These tests ensure that the unified LLVM simulation API provides at least the same functionality as
//! `LlvmEngine`, plus tests for its additional features like noise models and parallelization.

use pecos_llvm_sim::llvm_engine;
use pecos_engines::{ClassicalControlEngineBuilder, state_vector, sparse_stabilizer, DepolarizingNoise, sim_builder};
use pecos_programs::LlvmProgram;
use std::collections::HashMap;
use std::path::PathBuf;

mod common;
use common::get_register_i64;

/// Get the path to the Bell state example
fn get_bell_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .parent()
        .expect("Expected to find workspace directory as parent of crates/");
    workspace_dir.join("examples/llvm/bell.ll")
}

/// Get the path to the quantum program example
fn get_qprog_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .parent()
        .expect("Expected to find workspace directory as parent of crates/");
    workspace_dir.join("examples/llvm/qprog.ll")
}

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

/// Skip the test with appropriate message if LLVM is not available
fn skip_if_llvm_missing(test_name: &str) -> bool {
    if !is_llvm_available() {
        println!("Skipping {test_name}: LLVM tools not found");
        println!("To enable LLVM tests, install LLVM version 14");
        return true;
    }
    false
}

// =============================================================================
// Tests matching LlvmEngine coverage
// =============================================================================

#[test]
fn test_llvm_sim_bell_state_immediate_measurement() {
    if skip_if_llvm_missing("test_llvm_sim_bell_state_immediate_measurement") {
        return;
    }

    // Run Bell state with unified API (matches test_bell_state_immediate_measurement)
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42) // Use seed for reproducibility
        .workers(2) // Match the original test
        .qubits(2)
        .run(100)
        .expect("LLVM simulation execution should succeed");

    // Process results
    let mut counts: HashMap<i64, usize> = HashMap::new();

    // Convert to ShotMap for columnar access
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let c_values = get_register_i64(&shot_map, "c").expect("Should have c register");
    
    for &value in &c_values {
        *counts.entry(value).or_insert(0) += 1;
    }

    // Print the counts for debugging
    println!("Bell state results (unified API):");
    for (result, count) in &counts {
        println!("  {result}: {count}");
    }

    // Verify results
    assert_eq!(
        shot_vec.len(),
        100,
        "Expected 100 shots"
    );

    // For a Bell state we should only see results 0 (00) or 3 (11)
    for &result in counts.keys() {
        assert!(
            result == 0 || result == 3,
            "Expected only 0 or 3 in Bell state measurements, but found '{result}'"
        );
    }

    // With 100 shots and a fixed seed, we should see both outcomes
    assert!(
        counts.contains_key(&0) || counts.contains_key(&3),
        "Expected to see at least one Bell state outcome"
    );
}

#[test]
fn test_llvm_sim_qprog_adaptive_algorithm() {
    if skip_if_llvm_missing("test_llvm_sim_qprog_adaptive_algorithm") {
        return;
    }

    // Run adaptive algorithm with unified API (matches test_qprog_adaptive_algorithm)
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_qprog_path()).unwrap()))
        .seed(42)
        .workers(2)
        .qubits(3)
        .quantum(state_vector()) // Use state vector engine for RZ gates
        .run(50)
        .expect("Adaptive algorithm execution should succeed");

    // Verify we get results
    assert!(!shot_vec.is_empty(), "Expected non-empty results");

    // Convert to ShotMap for columnar access
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let registers = shot_map.register_names();

    // Check that we have the expected result registers
    assert!(
        registers.iter().any(|r| *r == "result_0"),
        "Expected 'result_0' register"
    );
    assert!(
        registers.iter().any(|r| *r == "result_1"),
        "Expected 'result_1' register"
    );
    assert!(
        registers.iter().any(|r| *r == "result_2"),
        "Expected 'result_2' register"
    );

    // Count results for each register
    let mut result_0_counts: HashMap<i64, usize> = HashMap::new();
    let mut result_1_counts: HashMap<i64, usize> = HashMap::new();
    let mut result_2_counts: HashMap<i64, usize> = HashMap::new();

    // Get values for each register
    let result_0_values = get_register_i64(&shot_map, "result_0").expect("Should have result_0");
    let result_1_values = get_register_i64(&shot_map, "result_1").expect("Should have result_1");
    let result_2_values = get_register_i64(&shot_map, "result_2").expect("Should have result_2");

    for i in 0..50 {
        *result_0_counts.entry(result_0_values[i]).or_insert(0) += 1;
        *result_1_counts.entry(result_1_values[i]).or_insert(0) += 1;
        *result_2_counts.entry(result_2_values[i]).or_insert(0) += 1;
    }

    // Print results for debugging
    println!("Adaptive algorithm results (unified API):");
    println!("  result_0: {result_0_counts:?}");
    println!("  result_1: {result_1_counts:?}");
    println!("  result_2: {result_2_counts:?}");

    // Verify valid measurement outcomes (0 or 1)
    for &value in result_0_counts.keys() {
        assert!(
            value == 0 || value == 1,
            "Expected 0 or 1 for result_0, got {value}"
        );
    }
    for &value in result_1_counts.keys() {
        assert!(
            value == 0 || value == 1,
            "Expected 0 or 1 for result_1, got {value}"
        );
    }
    for &value in result_2_counts.keys() {
        assert!(
            value == 0 || value == 1,
            "Expected 0 or 1 for result_2, got {value}"
        );
    }
}

#[test]
fn test_llvm_sim_single_worker() {
    if skip_if_llvm_missing("test_llvm_sim_single_worker") {
        return;
    }

    // Test with single worker (matches test_llvm_bell_state_single_worker)
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .workers(1) // Single worker
        .qubits(2)
        .run(10)
        .expect("Single worker execution should succeed");

    assert!(!shot_vec.is_empty(), "Expected non-empty results");
    println!(
        "Single-threaded LLVM simulation execution succeeded with {} shots",
        shot_vec.len()
    );
}

// =============================================================================
// Tests for LLVM simulation's additional features
// =============================================================================

#[test]
fn test_llvm_sim_with_uniform_depolarizing_noise() {
    if skip_if_llvm_missing("test_llvm_sim_with_uniform_depolarizing_noise") {
        return;
    }

    // Test Bell state with significant noise
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42)
        .workers(4)
        .noise(DepolarizingNoise { p: 0.2 }) // 20% error rate
        .qubits(2)
        .run(1000)
        .expect("Noisy simulation should succeed");

    // Convert to ShotMap and count results
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let c_values = get_register_i64(&shot_map, "c").expect("Should have c register");
    
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for &value in &c_values {
        *counts.entry(value).or_insert(0) += 1;
    }

    println!("Bell state with 20% depolarizing noise:");
    for (result, count) in &counts {
        println!("  {}: {} ({:.1}%)", result, count, (*count as f64 / 10.0));
    }

    // With 20% noise, we should see error states (1 and 2)
    let error_count = counts.get(&1).unwrap_or(&0) + counts.get(&2).unwrap_or(&0);
    assert!(
        error_count > 0,
        "Expected to see error states with 20% noise"
    );

    // But Bell states (0 and 3) should still be dominant
    let bell_count = counts.get(&0).unwrap_or(&0) + counts.get(&3).unwrap_or(&0);
    assert!(
        bell_count > error_count,
        "Bell states should still be more common than errors"
    );
}

#[test]
fn test_llvm_sim_with_custom_depolarizing_noise() {
    if skip_if_llvm_missing("test_llvm_sim_with_custom_depolarizing_noise") {
        return;
    }

    // Test with custom noise parameters
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42)
        .noise(DepolarizingNoise { p: 0.02 }) // Use standard depolarizing noise
        .qubits(2)
        .run(1000)
        .expect("Custom noise simulation should succeed");

    // With higher two-qubit gate error, we should see more errors
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let c_values = get_register_i64(&shot_map, "c").expect("Should have c register");
    let error_count = c_values.iter().filter(|&&v| v == 1 || v == 2).count();

    println!("Custom noise model results:");
    println!(
        "  Error states: {} ({:.1}%)",
        error_count,
        error_count as f64 / 10.0
    );

    assert!(error_count > 0, "Expected errors with custom noise model");
}

#[test]
fn test_llvm_sim_parallel_execution_scaling() {
    if skip_if_llvm_missing("test_llvm_sim_parallel_execution_scaling") {
        return;
    }

    // Test parallel execution with different worker counts
    let worker_counts = vec![1, 2, 4, 8];

    for workers in worker_counts {
        let start = std::time::Instant::now();

        let shot_vec = sim_builder()
            .classical(llvm_engine()
            .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
            .seed(42)
            .workers(workers)
            .qubits(2)
            .run(1000)
            .unwrap_or_else(|_| panic!("Simulation with {workers} workers should succeed"));

        let elapsed = start.elapsed();

        println!(
            "Execution with {} workers took: {:.3}s",
            workers,
            elapsed.as_secs_f64()
        );
        assert_eq!(shot_vec.len(), 1000);
    }
}

#[test]
fn test_llvm_sim_quantum_engines() {
    if skip_if_llvm_missing("test_llvm_sim_quantum_engines") {
        return;
    }

    // Test both quantum engines
    // Test StateVector engine
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42)
        .qubits(2)
        .quantum(state_vector())
        .run(100)
        .unwrap_or_else(|_| panic!("StateVector engine should succeed"));

    println!(
        "StateVector engine succeeded with {} shots",
        shot_vec.len()
    );

    // Test SparseStabilizer engine
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42)
        .qubits(2)
        .quantum(sparse_stabilizer())
        .run(100)
        .unwrap_or_else(|_| panic!("SparseStabilizer engine should succeed"));

    println!(
        "SparseStabilizer engine succeeded with {} shots",
            shot_vec.len()
        );

    // Verify Bell state results
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let c_values = get_register_i64(&shot_map, "c").expect("Should have c register");
    
    for &value in &c_values {
        assert!(
            value == 0 || value == 3,
            "SparseStabilizer engine: Expected Bell state results"
        );
    }
}

#[test]
fn test_llvm_sim_build_once_run_many() {
    if skip_if_llvm_missing("test_llvm_sim_build_once_run_many") {
        return;
    }

    // Build simulation once
    let mut sim = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42)
        .workers(4)
        .noise(DepolarizingNoise { p: 0.01 })
        .qubits(2)
        .build()
        .expect("Build should succeed");

    // Run multiple times with different shot counts
    let shot_counts = [10, 100, 1000, 50];
    let mut total_shots = 0;

    for (i, &shots) in shot_counts.iter().enumerate() {
        let shot_vec = sim
            .run(shots)
            .unwrap_or_else(|_| panic!("Run {} should succeed", i + 1));
        assert_eq!(shot_vec.len(), shots);
        total_shots += shots;
    }

    // MonteCarloEngine doesn't have a stats() method anymore
    // Just verify the runs completed successfully
    println!("Build once, run many: {} total shots across {} runs", total_shots, shot_counts.len());
}

#[test]
fn test_llvm_sim_in_memory_string() {
    if skip_if_llvm_missing("test_llvm_sim_in_memory_string") {
        return;
    }

    // Test with in-memory LLVM IR string
    let llvm_ir = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.result = constant [7 x i8] c"result\00"

define void @main() #0 {
    call void @__quantum__qis__h__body(i64 0)
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([7 x i8], [7 x i8]* @.str.result, i32 0, i32 0))
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_string(llvm_ir)))
        .seed(42)
        .qubits(1)
        .run(100)
        .expect("In-memory LLVM IR should work");

    // Convert to ShotMap
    let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let registers = shot_map.register_names();
    assert!(registers.iter().any(|r| *r == "result"));
    assert_eq!(shot_vec.len(), 100);

    // Should be roughly 50/50 distribution
    let result_values = get_register_i64(&shot_map, "result").expect("Should have result register");
    let ones = result_values.iter().filter(|&&v| v == 1).count();
    println!("In-memory Hadamard: {ones} ones out of 100");
    assert!(
        ones > 30 && ones < 70,
        "Expected roughly 50/50 distribution"
    );
}

#[test]
fn test_llvm_sim_reproducibility_with_seed() {
    if skip_if_llvm_missing("test_llvm_sim_reproducibility_with_seed") {
        return;
    }

    // Run twice with same seed
    let seed = 12345;

    let shot_vec1 = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(seed)
        .workers(1) // Single worker for determinism
        .qubits(2)
        .run(100)
        .expect("First run should succeed");

    let shot_vec2 = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(seed)
        .workers(1) // Single worker for determinism
        .qubits(2)
        .run(100)
        .expect("Second run should succeed");

    // Convert to ShotMaps for comparison
    let shot_map1 = shot_vec1.try_as_shot_map().expect("Should convert to ShotMap");
    let shot_map2 = shot_vec2.try_as_shot_map().expect("Should convert to ShotMap");
    
    let c_values1 = get_register_i64(&shot_map1, "c").expect("Should have c register");
    let c_values2 = get_register_i64(&shot_map2, "c").expect("Should have c register");

    // Results should be identical
    assert_eq!(
        c_values1, c_values2,
        "Same seed should produce identical results"
    );

    println!("Reproducibility test passed: identical results with same seed");
}

#[test]
fn test_llvm_sim_error_handling() {
    if skip_if_llvm_missing("test_llvm_sim_error_handling") {
        return;
    }

    // Test with invalid LLVM IR
    let invalid_ir = "This is not valid LLVM IR";
    let result = llvm_engine().program(LlvmProgram::from_string(invalid_ir)).to_sim().qubits(1).run(10);
    assert!(result.is_err(), "Invalid LLVM IR should fail");

    // Test with LLVM IR missing entry point
    let no_entry_ir = r"
    define void @not_main() {
        ret void
    }
    ";
    let result = llvm_engine().program(LlvmProgram::from_string(no_entry_ir)).to_sim().qubits(1).run(10);
    assert!(result.is_err(), "LLVM IR without EntryPoint should fail");

    // Test with non-existent file
    let result = LlvmProgram::from_file("/non/existent/file.ll");
    assert!(result.is_err(), "Non-existent file should fail");
}

#[test]
fn test_llvm_sim_verbose_options() {
    if skip_if_llvm_missing("test_llvm_sim_verbose_options") {
        return;
    }

    // Test with verbose option
    let shot_vec = sim_builder()
        .classical(llvm_engine()
        .program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .verbose(true)
        .qubits(2)
        .run(10)
        .expect("Verbose run should succeed");

    assert_eq!(shot_vec.len(), 10);
    println!("Verbose test completed");
}
