use pecos_engines::engines::MonteCarloEngine;
use pecos_engines::engines::classical::setup_engine;
use std::path::PathBuf;
use std::collections::HashMap;

#[test]
fn test_bell_state_noiseless() {
    // Get the path to the Bell state example
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let bell_file = workspace_dir.join("examples/phir/bell.json");
    
    // Run the Bell state example with 100 shots and 2 workers
    let classical_engine = setup_engine(&bell_file).unwrap();
    let results = MonteCarloEngine::run_with_classical_engine(
        classical_engine,
        0.0, // No noise
        100,
        2,
    )
    .unwrap();
    
    // Count occurrences of each result
    let mut counts: HashMap<String, usize> = HashMap::new();
    
    // Verify that all results are either "00" or "11" (Bell state property)
    for shot in &results.shots {
        let result_str = shot.get("result").unwrap();
        *counts.entry(result_str.clone()).or_insert(0) += 1;
        assert!(result_str == "00" || result_str == "11", 
                "Expected '00' or '11', got '{}'", result_str);
    }
    
    // Print the counts for debugging
    println!("Noiseless Bell state results:");
    for (result, count) in counts.iter() {
        println!("  {}: {}", result, count);
    }
}

#[test]
fn test_bell_state_with_noise() {
    // Get the path to the Bell state example
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir.parent().unwrap().parent().unwrap();
    let bell_file = workspace_dir.join("examples/phir/bell.json");
    
    // Run the Bell state example with 100 shots, 2 workers, and 0.2 noise probability
    let classical_engine = setup_engine(&bell_file).unwrap();
    let results = MonteCarloEngine::run_with_classical_engine(
        classical_engine,
        0.2, // 20% noise
        1000, // More shots for better statistics
        2,
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
    println!("Noisy Bell state results (p=0.2):");
    for (result, count) in counts.iter() {
        println!("  {}: {}", result, count);
    }
    
    // Check that we have some non-Bell state results (01 or 10)
    // With 20% noise and 1000 shots, it's extremely unlikely not to see any noise effects
    let non_bell_count = counts.get("01").unwrap_or(&0) + counts.get("10").unwrap_or(&0);
    let total_count = results.shots.len();
    
    // Calculate the percentage of non-Bell state results
    let non_bell_percentage = (non_bell_count as f64 / total_count as f64) * 100.0;
    println!("Non-Bell state percentage: {:.2}%", non_bell_percentage);
    
    // With 20% noise, we expect roughly 20% of the results to be non-Bell states
    // But to avoid flaky tests, we'll use a very conservative threshold
    // The probability of getting less than 5% non-Bell states with 20% noise is extremely low
    assert!(
        non_bell_percentage > 5.0,
        "Expected at least 5% non-Bell states with 20% noise, but got {:.2}%",
        non_bell_percentage
    );
} 