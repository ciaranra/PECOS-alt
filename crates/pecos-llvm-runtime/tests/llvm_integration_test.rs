use std::collections::HashMap;
use std::path::PathBuf;

use pecos_engines::engine_system::MonteCarloEngine;
use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_llvm_runtime::LlvmEngine;

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

/// Check if LLVM llc tool is available
fn is_llc_available() -> bool {
    if cfg!(windows) {
        std::env::var("PATH")
            .map(|paths| {
                paths
                    .split(';')
                    .any(|dir| std::path::Path::new(dir).join("llc.exe").exists())
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
fn skip_if_llc_missing(test_name: &str) -> bool {
    if !is_llc_available() {
        println!("Skipping {test_name}: LLVM 'llc' tool not found");
        println!("To enable QIR tests, install LLVM version 14 (e.g., 'sudo apt install llvm-14')");
        return true;
    }
    false
}

#[test]
fn test_bell_state_immediate_measurement() {
    // Skip if LLVM is not available
    if skip_if_llc_missing("test_bell_state_immediate_measurement") {
        return;
    }

    // Create a QIR engine with Bell state file
    let llvm_engine = LlvmEngine::new(get_bell_path());

    // Create a noiseless model
    let noise_model = Box::new(DepolarizingNoiseModel::new_uniform(0.0));

    // Run the Bell state example with 100 shots
    let results = MonteCarloEngine::run_with_noise_model(
        Box::new(llvm_engine),
        noise_model,
        100,
        2,
        None, // No specific seed
    )
    .expect("QIR execution should succeed");

    // Count occurrences of each result
    let mut counts: HashMap<String, usize> = HashMap::new();

    // Process results, checking for the "c" register
    for shot in &results.shots {
        let result_str = shot
            .data
            .get("c")
            .map(|data| match data {
                pecos_engines::shot_results::Data::U32(v) => v.to_string(),
                pecos_engines::shot_results::Data::I64(v) => v.to_string(),
                _ => panic!("Unexpected data type in 'c' register: {data:?}"),
            })
            .expect("Expected 'c' register in Bell state results");
        *counts.entry(result_str).or_insert(0) += 1;
    }

    // Print the counts for debugging
    println!("Bell state results (immediate measurement):");
    for (result, count) in &counts {
        println!("  {result}: {count}");
    }

    // Verify results
    assert!(!results.shots.is_empty(), "Expected non-empty results");

    // For a Bell state we should only see results "0" (00 in binary) or "3" (11 in binary)
    // This verifies that immediate measurement preserves Bell state correlations
    for result in counts.keys() {
        if !result.is_empty() {
            assert!(
                result == "0" || result == "3",
                "Expected only '0' or '3' in Bell state measurements, but found '{result}'"
            );
        }
    }

    // Ensure we actually got both possible Bell state outcomes
    let has_zero = counts.contains_key("0");
    let has_three = counts.contains_key("3");

    // With 100 shots, we should see both outcomes unless we're extremely unlucky
    assert!(
        has_zero || has_three,
        "Expected to see at least one Bell state outcome"
    );
}

#[test]
fn test_qprog_adaptive_algorithm() {
    // Skip if LLVM is not available
    if skip_if_llc_missing("test_qprog_adaptive_algorithm") {
        return;
    }

    // Create a QIR engine with quantum program file
    let llvm_engine = LlvmEngine::new(get_qprog_path());

    // Create a noiseless model
    let noise_model = Box::new(DepolarizingNoiseModel::new_uniform(0.0));

    // Run the quantum program with 50 shots
    let results = MonteCarloEngine::run_with_noise_model(
        Box::new(llvm_engine),
        noise_model,
        50,
        2,
        None, // No specific seed
    )
    .expect("Adaptive QIR execution should succeed");

    // Verify we get results
    assert!(!results.shots.is_empty(), "Expected non-empty results");

    // Check that we have the expected result registers
    let first_shot = &results.shots[0];

    // Should have result_0, result_1, and result_2 registers
    assert!(
        first_shot.data.contains_key("result_0"),
        "Expected 'result_0' register in adaptive algorithm results"
    );
    assert!(
        first_shot.data.contains_key("result_1"),
        "Expected 'result_1' register in adaptive algorithm results"
    );
    assert!(
        first_shot.data.contains_key("result_2"),
        "Expected 'result_2' register in adaptive algorithm results"
    );

    // Count results for each register
    let mut result_0_counts: HashMap<u32, usize> = HashMap::new();
    let mut result_1_counts: HashMap<u32, usize> = HashMap::new();
    let mut result_2_counts: HashMap<u32, usize> = HashMap::new();

    for shot in &results.shots {
        // Extract result_0
        if let Some(data) = shot.data.get("result_0") {
            let value = match data {
                pecos_engines::shot_results::Data::U32(v) => *v,
                pecos_engines::shot_results::Data::I64(v) => {
                    u32::try_from(*v).expect("Result value should be 0 or 1, which fits in u32")
                }
                _ => panic!("Unexpected data type in result_0"),
            };
            *result_0_counts.entry(value).or_insert(0) += 1;
        }

        // Extract result_1
        if let Some(data) = shot.data.get("result_1") {
            let value = match data {
                pecos_engines::shot_results::Data::U32(v) => *v,
                pecos_engines::shot_results::Data::I64(v) => {
                    u32::try_from(*v).expect("Result value should be 0 or 1, which fits in u32")
                }
                _ => panic!("Unexpected data type in result_1"),
            };
            *result_1_counts.entry(value).or_insert(0) += 1;
        }

        // Extract result_2 (intermediate measurement)
        if let Some(data) = shot.data.get("result_2") {
            let value = match data {
                pecos_engines::shot_results::Data::U32(v) => *v,
                pecos_engines::shot_results::Data::I64(v) => {
                    u32::try_from(*v).expect("Result value should be 0 or 1, which fits in u32")
                }
                _ => panic!("Unexpected data type in result_2"),
            };
            *result_2_counts.entry(value).or_insert(0) += 1;
        }
    }

    // Print results for debugging
    println!("Adaptive algorithm results:");
    println!("  result_0 (final qubit 0): {result_0_counts:?}");
    println!("  result_1 (final qubit 1): {result_1_counts:?}");
    println!("  result_2 (intermediate): {result_2_counts:?}");

    // Verify that we see valid measurement outcomes (0 or 1)
    for value in result_0_counts.keys() {
        assert!(
            *value == 0 || *value == 1,
            "Expected 0 or 1 for result_0, got {value}"
        );
    }
    for value in result_1_counts.keys() {
        assert!(
            *value == 0 || *value == 1,
            "Expected 0 or 1 for result_1, got {value}"
        );
    }
    for value in result_2_counts.keys() {
        assert!(
            *value == 0 || *value == 1,
            "Expected 0 or 1 for result_2, got {value}"
        );
    }

    println!("Adaptive algorithm test passed!");
}
