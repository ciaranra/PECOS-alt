use std::collections::HashMap;
use std::path::PathBuf;

use pecos_engines::engines::MonteCarloEngine;
use pecos_engines::engines::qir::QirEngine;

/// Get the path to the QIR Bell state example
fn get_qir_program_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    workspace_dir.join("examples/qir/bell.ll")
}

#[test]
fn test_qir_bell_state_noiseless() {
    // Create a QIR engine directly with the file path
    let qir_engine = QirEngine::new(get_qir_program_path());

    // Run the Bell state example with 100 shots and 2 workers
    let results = MonteCarloEngine::run_with_classical_engine(
        Box::new(qir_engine),
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
    println!("Noiseless QIR Bell state results:");
    for (result, count) in &counts {
        println!("  {result}: {count}");
    }
}

#[test]
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::cast_precision_loss)]
pub fn test_qir_bell_state_with_noise() {
    let mut success_found = false;

    // Try multiple seeds to avoid flaky tests
    for seed in 1..=5 {
        println!("Testing with seed: {seed}");

        let noise_probability = 0.3;
        let shots = 500;

        // Create QirEngine
        let qir_engine = QirEngine::new(get_qir_program_path());

        // Run with the MonteCarloEngine directly, specifying the number of shots
        let results = MonteCarloEngine::run_with_classical_engine(
            Box::new(qir_engine),
            noise_probability,
            shots,
            2, // Number of workers
            Some(seed),
        )
        .unwrap();

        // Count results
        let mut bell_state_count = 0;
        let mut counts: HashMap<String, usize> = HashMap::new();

        for shot in &results.shots {
            let result_str = shot.get("result").unwrap();
            *counts.entry(result_str.clone()).or_insert(0) += 1;

            if result_str == "00" || result_str == "11" {
                bell_state_count += 1;
            }
        }

        // Print counts for debugging
        println!("Counts with noise (seed {seed}):");
        for (result, count) in &counts {
            println!("  {result}: {count}");
        }

        // Calculate percentage of non-Bell states
        let non_bell_percentage = 100.0 * (1.0 - (f64::from(bell_state_count) / shots as f64));
        println!("Non-Bell state percentage (seed {seed}): {non_bell_percentage:.2}%");

        // If we find at least 1% non-Bell states in any run, consider the test successful
        if non_bell_percentage > 1.0 {
            success_found = true;
            break;
        }
    }

    // Assert that at least one run showed a reasonable amount of noise
    assert!(
        success_found,
        "Noise does not appear to be working correctly. No run showed significant non-Bell states."
    );
}
