use assert_cmd::prelude::*;
use std::path::PathBuf;
use std::process::Command;

// Helper function to extract register keys from the new JSON shot array format
fn get_keys(json_output: &str) -> Vec<String> {
    let mut keys = std::collections::HashSet::new();

    // Parse the new JSON format: array of shot objects like [{"c": 3}, {"c": 0}, ...]
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_output) {
        if let Some(shots_array) = json.as_array() {
            // Extract register names from all shot objects
            for shot in shots_array {
                if let Some(shot_obj) = shot.as_object() {
                    for key in shot_obj.keys() {
                        keys.insert(key.clone());
                    }
                }
            }
        }
    }

    // Convert to sorted vector
    let mut result: Vec<String> = keys.into_iter().collect();
    result.sort();
    result
}

// Helper function to extract values from the new JSON shot array format
fn get_values(json_output: &str) -> Vec<String> {
    let mut values = Vec::new();

    // Parse the new JSON format: array of shot objects like [{"c": 3}, {"c": 0}, ...]
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_output) {
        if let Some(shots_array) = json.as_array() {
            // Extract values from each shot object
            for shot in shots_array {
                if let Some(shot_obj) = shot.as_object() {
                    // Convert each shot object to a string representation
                    let mut shot_values = Vec::new();
                    for (key, value) in shot_obj {
                        let val_str = if let Some(num) = value.as_u64() {
                            num.to_string()
                        } else if let Some(num) = value.as_i64() {
                            num.to_string()
                        } else if let Some(num) = value.as_f64() {
                            num.to_string()
                        } else {
                            value.to_string().replace('"', "")
                        };
                        shot_values.push(format!("{key}:{val_str}"));
                    }
                    // Sort keys within each shot for consistent ordering
                    shot_values.sort();
                    values.push(shot_values.join(","));
                }
            }
        }
    }

    // Sort for stable comparison
    values.sort();
    values
}

#[test]
fn test_seed_produces_consistent_results() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_file = manifest_dir.join("../../examples/phir/bell.json");

    // Run multiple times with seed 42, forcing JSON format
    let seed_42_run1 = Command::cargo_bin("pecos")?
        .env("RUST_LOG", "info")
        .arg("run")
        .arg(&test_file)
        .arg("-s")
        .arg("10") // Fewer shots for faster tests
        .arg("-w")
        .arg("1") // Single worker to avoid thread scheduling differences
        .arg("-p")
        .arg("0.1")
        .arg("-d")
        .arg("42")
        .output()?;

    let seed_42_run2 = Command::cargo_bin("pecos")?
        .env("RUST_LOG", "info")
        .arg("run")
        .arg(&test_file)
        .arg("-s")
        .arg("10")
        .arg("-w")
        .arg("1")
        .arg("-p")
        .arg("0.1")
        .arg("-d")
        .arg("42")
        .output()?;

    // Run multiple times with seed 43
    let seed_43_run1 = Command::cargo_bin("pecos")?
        .env("RUST_LOG", "info")
        .arg("run")
        .arg(&test_file)
        .arg("-s")
        .arg("10")
        .arg("-w")
        .arg("1")
        .arg("-p")
        .arg("0.1")
        .arg("-d")
        .arg("43")
        .output()?;

    let seed_43_run2 = Command::cargo_bin("pecos")?
        .env("RUST_LOG", "info")
        .arg("run")
        .arg(&test_file)
        .arg("-s")
        .arg("10")
        .arg("-w")
        .arg("1")
        .arg("-p")
        .arg("0.1")
        .arg("-d")
        .arg("43")
        .output()?;

    // Check that all commands ran successfully
    assert!(seed_42_run1.status.success(), "First seed 42 run failed");
    assert!(seed_42_run2.status.success(), "Second seed 42 run failed");
    assert!(seed_43_run1.status.success(), "First seed 43 run failed");
    assert!(seed_43_run2.status.success(), "Second seed 43 run failed");

    // Convert outputs to strings
    let seed_42_output1 = String::from_utf8(seed_42_run1.stdout)?;
    let seed_42_output2 = String::from_utf8(seed_42_run2.stdout)?;
    let seed_43_output1 = String::from_utf8(seed_43_run1.stdout)?;
    let seed_43_output2 = String::from_utf8(seed_43_run2.stdout)?;

    // We need to normalize the JSON by sorting the keys, to ensure a stable order for comparison
    // Since we can't use serde_json without adding a dependency, we'll just print the sorted keys
    // and check for key existence

    // Check that seed 42 runs have the same keys
    let keys_42_1 = get_keys(&seed_42_output1);
    let keys_42_2 = get_keys(&seed_42_output2);
    assert_eq!(
        keys_42_1, keys_42_2,
        "Results with seed 42 should have the same keys across runs"
    );

    // Check that seed 43 runs have the same keys
    let keys_43_1 = get_keys(&seed_43_output1);
    let keys_43_2 = get_keys(&seed_43_output2);
    assert_eq!(
        keys_43_1, keys_43_2,
        "Results with seed 43 should have the same keys across runs"
    );

    // Check that seed 42 runs have values in the same positions
    let values_42_1 = get_values(&seed_42_output1);
    let values_42_2 = get_values(&seed_42_output2);
    assert_eq!(
        values_42_1, values_42_2,
        "Results with seed 42 should have the same values across runs"
    );

    // Check that seed 43 runs have values in the same positions
    let values_43_1 = get_values(&seed_43_output1);
    let values_43_2 = get_values(&seed_43_output2);
    assert_eq!(
        values_43_1, values_43_2,
        "Results with seed 43 should have the same values across runs"
    );

    // Verify that different seeds produce different results by checking value patterns
    assert_ne!(
        values_42_1, values_43_1,
        "Results with different seeds (42 vs 43) should differ"
    );

    Ok(())
}
