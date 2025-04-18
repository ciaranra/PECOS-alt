use pecos_engines::engines::MonteCarloEngine;
use pecos_engines::engines::classical::setup_engine;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_bell_state_noiseless() {
    // Get the path to the Bell state example
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let bell_file = workspace_dir.join("examples/phir/bell.json");

    // Run the Bell state example with 100 shots and 2 workers
    let classical_engine = setup_engine(&bell_file, None).unwrap();
    let results = MonteCarloEngine::run_with_classical_engine(
        classical_engine,
        0.0, // No noise
        100,
        2,
        None, // No specific seed
    )
    .unwrap();

    // Count occurrences of each result
    let mut counts: HashMap<String, usize> = HashMap::new();

    // Verify that all results are either "00" or "11" (Bell state property)
    for shot in &results.shots {
        let result_str = shot.get("result").unwrap();
        *counts.entry(result_str.clone()).or_insert(0) += 1;
        assert!(
            result_str == "00" || result_str == "11",
            "Expected '00' or '11', got '{result_str}'"
        );
    }

    // Print the counts for debugging
    println!("Noiseless Bell state results:");
    for (result, count) in &counts {
        println!("  {result}: {count}");
    }
}

#[allow(clippy::cast_precision_loss)]
#[test]
fn test_bell_state_with_noise() {
    // Get the path to the Bell state example
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let bell_file = workspace_dir.join("examples/phir/bell.json");

    // Try multiple runs with different seeds to avoid flakiness
    let mut successful_run = false;
    for seed in 1..=5 {
        println!("Attempting test with seed {seed}");

        // Run the Bell state example with high noise probability for more reliable testing
        let classical_engine = setup_engine(&bell_file, None).unwrap();
        let results = MonteCarloEngine::run_with_classical_engine(
            classical_engine,
            0.3, // 30% noise - higher to ensure we get some noise effects
            500, // Fewer shots but repeated runs
            2,
            Some(seed), // Use the current iteration as seed
        )
        .unwrap();

        // Count occurrences of each result
        let mut counts: HashMap<String, usize> = HashMap::new();

        // For the noisy version, we just ensure it runs without errors
        assert!(!results.shots.is_empty(), "Expected non-empty results");

        // Count all results
        for shot in &results.shots {
            let result_str = shot.get("result").unwrap();
            *counts.entry(result_str.clone()).or_insert(0) += 1;
        }

        // Print the counts for debugging
        println!("Noisy Bell state results (p=0.3, seed={seed}):");
        for (result, count) in &counts {
            println!("  {result}: {count}");
        }

        // Check that we have some non-Bell state results (01 or 10)
        let non_bell_count = counts.get("01").unwrap_or(&0) + counts.get("10").unwrap_or(&0);
        let total_count = results.shots.len();

        // Calculate the percentage of non-Bell state results
        let non_bell_percentage = (non_bell_count as f64 / total_count as f64) * 100.0;
        println!("Non-Bell state percentage: {non_bell_percentage:.2}%");

        // If we find at least one run where noise is applied correctly, the test passes
        if non_bell_percentage > 1.0 {
            successful_run = true;
            break;
        }
    }

    // Verify that at least one run had a reasonable amount of noise
    assert!(
        successful_run,
        "Failed to see noise effects in any of the test runs. Is noise application working correctly?"
    );
}
