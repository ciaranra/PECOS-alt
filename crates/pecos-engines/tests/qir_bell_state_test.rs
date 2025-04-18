use pecos_engines::engines::MonteCarloEngine;
use pecos_engines::engines::qir::QirEngine;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_qir_bell_state_noiseless() {
    // Get the path to the QIR Bell state example
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let bell_file = workspace_dir.join("examples/qir/bell.ll");

    // Create a QIR engine directly with the file path
    let qir_engine = QirEngine::new(bell_file.clone());

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

#[allow(clippy::cast_precision_loss)]
#[test]
fn test_qir_bell_state_with_noise() {
    // Get the path to the QIR Bell state example
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let bell_file = workspace_dir.join("examples/qir/bell.ll");

    // Create a QIR engine directly with the file path
    let qir_engine = QirEngine::new(bell_file.clone());

    // Run the Bell state example with 100 shots, 2 workers, and 0.2 noise probability
    let results = MonteCarloEngine::run_with_classical_engine(
        Box::new(qir_engine),
        0.2,  // 20% noise
        1000, // More shots for better statistics
        2,
        None, // No specific seed
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
    println!("Noisy QIR Bell state results (p=0.2):");
    for (result, count) in &counts {
        println!("  {result}: {count}");
    }

    // Check that we have some non-Bell state results (01 or 10)
    // With 20% noise and 1000 shots, it's extremely unlikely not to see any noise effects
    let non_bell_count = counts.get("01").unwrap_or(&0) + counts.get("10").unwrap_or(&0);
    let total_count = results.shots.len();

    // Calculate the percentage of non-Bell state results
    let non_bell_percentage = (non_bell_count as f64 / total_count as f64) * 100.0;
    println!("Non-Bell state percentage: {non_bell_percentage:.2}%");

    // With 20% noise, we expect roughly 20% of the results to be non-Bell states
    // But to avoid flaky tests, we'll use a very conservative threshold
    // The probability of getting less than 5% non-Bell states with 20% noise is extremely low
    assert!(
        non_bell_percentage > 5.0,
        "Expected at least 5% non-Bell states with 20% noise, but got {non_bell_percentage:.2}%"
    );
}
