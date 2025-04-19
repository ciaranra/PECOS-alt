use assert_cmd::prelude::*;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_seed_produces_consistent_results() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_file = manifest_dir.join("../../examples/phir/bell.json");

    // Run multiple times with seed 42
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

    // Verify consistency within each seed group
    assert_eq!(
        seed_42_output1, seed_42_output2,
        "Results with seed 42 should be identical across runs"
    );

    assert_eq!(
        seed_43_output1, seed_43_output2,
        "Results with seed 43 should be identical across runs"
    );

    // Verify that different seeds produce different results
    assert_ne!(
        seed_42_output1, seed_43_output1,
        "Results with different seeds (42 vs 43) should differ"
    );

    Ok(())
}
