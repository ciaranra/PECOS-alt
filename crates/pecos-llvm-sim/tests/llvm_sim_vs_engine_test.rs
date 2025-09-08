//! Tests for LLVM simulation unified API functionality and correctness

// use pecos_engines::engine_system::MonteCarloEngine;
// use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_engines::{DepolarizingNoise, sim_builder};
use pecos_llvm_sim::llvm_engine;
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

/// Check if LLVM tools are available
fn skip_if_no_llvm() -> bool {
    let has_llvm = if cfg!(windows) {
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
    };

    if has_llvm {
        false
    } else {
        println!("Skipping test: LLVM tools not available");
        true
    }
}

#[test]
fn test_llvm_unified_api_noiseless() {
    if skip_if_no_llvm() {
        return;
    }

    let seed = 42;
    let shots = 100;

    // Run with unified API
    let sim_shot_vec = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(seed)
        .workers(1) // Single worker for determinism
        .qubits(2)
        .run(shots)
        .expect("Unified API should succeed");

    // Analyze results
    println!("Testing unified API noiseless simulation:");

    // Convert results to ShotMap
    let sim_shot_map = sim_shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let sim_c_values = get_register_i64(&sim_shot_map, "c").expect("Should have c register");

    // Should have "c" register
    let sim_registers = sim_shot_map.register_names();
    assert!(
        sim_registers.contains(&"c"),
        "Unified API should have 'c' register"
    );

    // Analyze distribution
    let sim_counts = count_values(&sim_c_values);
    println!("Unified API distribution: {sim_counts:?}");

    // Should only have 0 and 3 (Bell states)
    for val in sim_counts.keys() {
        assert!(
            *val == 0 || *val == 3,
            "Unified API: unexpected value {val}"
        );
    }

    // Should have gotten the expected number of shots
    assert_eq!(sim_shot_vec.len(), shots);
}

#[test]
fn test_llvm_unified_api_with_noise() {
    if skip_if_no_llvm() {
        return;
    }

    let seed = 12345;
    let shots = 1000;
    let noise_level = 0.1; // 10% depolarizing noise

    // Run with unified API and noise
    let sim_shot_vec = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(seed)
        .workers(1)
        .noise(DepolarizingNoise { p: noise_level })
        .qubits(2)
        .run(shots)
        .expect("Unified API with noise should succeed");

    // Convert sim results to ShotMap
    let sim_shot_map = sim_shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let sim_c_values = get_register_i64(&sim_shot_map, "c").expect("Should have c register");

    // Analyze noise effects
    let sim_counts = count_values(&sim_c_values);

    println!("\nTesting unified API with 10% noise:");
    println!("Unified API distribution: {sim_counts:?}");

    // With noise, we should see all 4 possible outcomes (0, 1, 2, 3)
    let error_count = sim_counts.get(&1).unwrap_or(&0) + sim_counts.get(&2).unwrap_or(&0);
    let ideal_count = sim_counts.get(&0).unwrap_or(&0) + sim_counts.get(&3).unwrap_or(&0);

    println!("Error states (1,2): {error_count} out of {shots}");
    println!("Bell states (0,3): {ideal_count} out of {shots}");

    // With 10% noise, we should see some errors but Bell states should still dominate
    assert!(error_count > 0, "Should see some errors with 10% noise");
    assert!(
        ideal_count > error_count,
        "Bell states should still be more common than errors"
    );

    // Should have gotten the expected number of shots
    assert_eq!(sim_shot_vec.len(), shots);
}

#[test]
fn test_llvm_unified_api_advanced_features() {
    if skip_if_no_llvm() {
        return;
    }

    // Test advanced features of the unified API

    // 1. Easy noise model switching
    let noise_models = vec![
        ("No noise", 0.0),
        ("Light noise", 0.01),
        ("Medium noise", 0.05),
        ("Heavy noise", 0.2),
    ];

    for (name, level) in noise_models {
        let shot_vec = sim_builder()
            .classical(llvm_engine().program(LlvmProgram::from_file(get_bell_path()).unwrap()))
            .seed(42)
            .noise(DepolarizingNoise { p: level })
            .qubits(2)
            .run(100)
            .unwrap_or_else(|_| panic!("{name} should work"));

        // Convert to ShotMap and get c values
        let shot_map = shot_vec
            .try_as_shot_map()
            .expect("Should convert to ShotMap");
        let c_values = get_register_i64(&shot_map, "c").expect("Should have c register");
        let error_count = c_values.iter().filter(|&&v| v == 1 || v == 2).count();

        println!("{name}: {error_count} errors out of 100");

        // More noise should produce more errors
        if level > 0.0 {
            assert!(error_count > 0, "{name} should produce some errors");
        }
    }

    // 2. Easy parallelization control
    let worker_counts = vec![1, 2, 4, 8];
    for workers in worker_counts {
        let start = std::time::Instant::now();

        let shot_vec = sim_builder()
            .classical(llvm_engine().program(LlvmProgram::from_file(get_bell_path()).unwrap()))
            .seed(42)
            .workers(workers)
            .qubits(2)
            .run(1000)
            .unwrap_or_else(|_| panic!("{workers} workers should work"));

        let elapsed = start.elapsed();
        println!(
            "{} workers: {:.3}s for 1000 shots",
            workers,
            elapsed.as_secs_f64()
        );

        assert_eq!(shot_vec.len(), 1000);
    }

    // 3. Build once, run many with different configurations
    let sim = sim_builder()
        .classical(llvm_engine().program(LlvmProgram::from_file(get_bell_path()).unwrap()))
        .seed(42)
        .noise(DepolarizingNoise { p: 0.05 })
        .qubits(2)
        .build()
        .expect("Build should succeed");

    // Run multiple times with same configuration
    let mut sim = sim;
    for i in 1..=5 {
        let shots = i * 100;
        let shot_vec = sim
            .run(shots)
            .unwrap_or_else(|_| panic!("Run {i} should succeed"));
        assert_eq!(shot_vec.len(), shots);
    }

    // MonteCarloEngine doesn't have a stats() method anymore
    // Just verify the runs completed successfully
}

// Helper function to count occurrences
fn count_values(values: &[i64]) -> HashMap<i64, usize> {
    let mut counts = HashMap::new();
    for &val in values {
        *counts.entry(val).or_insert(0) += 1;
    }
    counts
}
