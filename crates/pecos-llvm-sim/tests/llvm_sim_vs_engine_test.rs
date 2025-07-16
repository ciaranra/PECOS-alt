//! Tests comparing `llvm_sim()` with direct `LlvmEngine` usage to ensure equivalence

use pecos_engines::engine_system::MonteCarloEngine;
use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_llvm_sim::{llvm_sim, LlvmEngine, DepolarizingNoise};
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
fn test_llvm_sim_vs_engine_noiseless() {
    if skip_if_no_llvm() {
        return;
    }

    let seed = 42;
    let shots = 100;

    // Run with llvm_sim
    let sim_shot_vec = llvm_sim()
        .llvm_file(get_bell_path())
        .seed(seed)
        .workers(1) // Single worker for determinism
        .run(shots)
        .expect("llvm_sim should succeed");

    // Run with LlvmEngine directly
    let llvm_engine = LlvmEngine::new(get_bell_path());
    let noise_model = Box::new(DepolarizingNoiseModel::new_uniform(0.0));
    let engine_results = MonteCarloEngine::run_with_noise_model(
        Box::new(llvm_engine),
        noise_model,
        shots,
        1, // Single worker
        Some(seed),
    )
    .expect("LlvmEngine should succeed");

    // Compare results
    println!("Comparing llvm_sim vs LlvmEngine (noiseless):");

    // Convert sim results to ShotMap
    let sim_shot_map = sim_shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let sim_c_values = get_register_i64(&sim_shot_map, "c").expect("Should have c register");
    
    // Convert engine results to columnar format for comparison
    let mut engine_columnar: HashMap<String, Vec<i64>> = HashMap::new();
    for shot in &engine_results.shots {
        for (key, value) in &shot.data {
            let val = match value {
                pecos_engines::shot_results::Data::I64(v) => *v,
                pecos_engines::shot_results::Data::U32(v) => i64::from(*v),
                _ => panic!("Unexpected data type"),
            };
            engine_columnar.entry(key.clone()).or_default().push(val);
        }
    }

    // Both should have "c" register
    let sim_registers = sim_shot_map.register_names();
    assert!(
        sim_registers.iter().any(|r| *r == "c"),
        "llvm_sim should have 'c' register"
    );
    assert!(
        engine_columnar.contains_key("c"),
        "LlvmEngine should have 'c' register"
    );

    // Compare distributions (not exact values due to potential ordering differences)
    let sim_counts = count_values(&sim_c_values);
    let engine_counts = count_values(&engine_columnar["c"]);

    println!("llvm_sim distribution: {sim_counts:?}");
    println!("LlvmEngine distribution: {engine_counts:?}");

    // Both should only have 0 and 3 (Bell states)
    for val in sim_counts.keys() {
        assert!(*val == 0 || *val == 3, "llvm_sim: unexpected value {val}");
    }
    for val in engine_counts.keys() {
        assert!(*val == 0 || *val == 3, "LlvmEngine: unexpected value {val}");
    }
}

#[test]
fn test_llvm_sim_vs_engine_with_noise() {
    if skip_if_no_llvm() {
        return;
    }

    let seed = 12345;
    let shots = 1000;
    let noise_level = 0.1; // 10% depolarizing noise

    // Run with llvm_sim
    let sim_shot_vec = llvm_sim()
        .llvm_file(get_bell_path())
        .seed(seed)
        .workers(1)
        .noise(DepolarizingNoise { p: noise_level })
        .run(shots)
        .expect("llvm_sim with noise should succeed");

    // Run with LlvmEngine directly
    let llvm_engine = LlvmEngine::new(get_bell_path());
    let noise_model = Box::new(DepolarizingNoiseModel::new_uniform(noise_level));
    let engine_results = MonteCarloEngine::run_with_noise_model(
        Box::new(llvm_engine),
        noise_model,
        shots,
        1,
        Some(seed),
    )
    .expect("LlvmEngine with noise should succeed");

    // Convert sim results to ShotMap
    let sim_shot_map = sim_shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
    let sim_c_values = get_register_i64(&sim_shot_map, "c").expect("Should have c register");
    
    // Convert to columnar and count
    let mut engine_columnar: HashMap<String, Vec<i64>> = HashMap::new();
    for shot in &engine_results.shots {
        for (key, value) in &shot.data {
            let val = match value {
                pecos_engines::shot_results::Data::I64(v) => *v,
                pecos_engines::shot_results::Data::U32(v) => i64::from(*v),
                _ => panic!("Unexpected data type"),
            };
            engine_columnar.entry(key.clone()).or_default().push(val);
        }
    }

    let sim_counts = count_values(&sim_c_values);
    let engine_counts = count_values(&engine_columnar["c"]);

    println!("\nComparing llvm_sim vs LlvmEngine (10% noise):");
    println!("llvm_sim distribution: {sim_counts:?}");
    println!("LlvmEngine distribution: {engine_counts:?}");

    // With noise, we should see all 4 possible outcomes (0, 1, 2, 3)
    // But distributions should be similar
    for val in 0..=3 {
        let sim_count = sim_counts.get(&val).unwrap_or(&0);
        let engine_count = engine_counts.get(&val).unwrap_or(&0);

        // Allow for statistical variation but they should be reasonably close
        let diff = (*sim_count as f64 - *engine_count as f64).abs();
        let avg = f64::midpoint(*sim_count as f64, *engine_count as f64);
        let relative_diff = if avg > 0.0 { diff / avg } else { 0.0 };

        println!(
            "Value {}: sim={}, engine={}, relative_diff={:.2}%",
            val,
            sim_count,
            engine_count,
            relative_diff * 100.0
        );

        // With same seed and single worker, results should be very close
        assert!(
            relative_diff < 0.1,
            "Value {val} distributions differ too much: sim={sim_count}, engine={engine_count}"
        );
    }
}

#[test]
fn test_llvm_sim_capabilities_exceed_engine() {
    if skip_if_no_llvm() {
        return;
    }

    // Test features that llvm_sim has but direct LlvmEngine usage doesn't easily provide

    // 1. Easy noise model switching
    let noise_models = vec![
        ("No noise", 0.0),
        ("Light noise", 0.01),
        ("Medium noise", 0.05),
        ("Heavy noise", 0.2),
    ];

    for (name, level) in noise_models {
        let shot_vec = llvm_sim()
            .llvm_file(get_bell_path())
            .seed(42)
            .noise(DepolarizingNoise { p: level })
            .run(100)
            .unwrap_or_else(|_| panic!("{name} should work"));

        // Convert to ShotMap and get c values
        let shot_map = shot_vec.try_as_shot_map().expect("Should convert to ShotMap");
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

        let shot_vec = llvm_sim()
            .llvm_file(get_bell_path())
            .seed(42)
            .workers(workers)
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
    let mut sim = llvm_sim()
        .llvm_file(get_bell_path())
        .seed(42)
        .noise(DepolarizingNoise { p: 0.05 })
        .build()
        .expect("Build should succeed");

    // Run multiple times with same configuration
    for i in 1..=5 {
        let shots = i * 100;
        let shot_vec = sim
            .run(shots)
            .unwrap_or_else(|_| panic!("Run {i} should succeed"));
        assert_eq!(shot_vec.len(), shots);
    }

    let (total_shots, total_runs) = sim.stats();
    assert_eq!(total_shots, 1500); // 100 + 200 + 300 + 400 + 500
    assert_eq!(total_runs, 5);
}

// Helper function to count occurrences
fn count_values(values: &[i64]) -> HashMap<i64, usize> {
    let mut counts = HashMap::new();
    for &val in values {
        *counts.entry(val).or_insert(0) += 1;
    }
    counts
}
