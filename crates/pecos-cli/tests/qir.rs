/// QIR Compilation Test
///
/// This test verifies that QIR files can be compiled and executed correctly.
/// Note: This test requires LLVM tools and GCC toolchain to be available.
use assert_cmd::prelude::*;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_pecos_compile_and_run() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_file = manifest_dir.join("../../examples/qir/qprog.ll");

    // Remove the cached library to ensure we see compilation messages
    let build_dir = manifest_dir.join("../../examples/qir/build");
    if build_dir.exists() {
        let _ = std::fs::remove_dir_all(&build_dir);
    }

    // Test compilation
    // Add cargo to PATH for the QIR runtime builder
    let mut path = std::env::var("PATH").unwrap_or_default();
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        path = format!("{cargo_home}/bin:{path}");
    } else {
        path = format!(
            "{}/.cargo/bin:{}",
            std::env::var("HOME").unwrap_or_default(),
            path
        );
    }

    let output = Command::cargo_bin("pecos")?
        .env("RUST_LOG", "info")
        .env("PATH", path.clone())
        .arg("compile")
        .arg(&test_file)
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed. Error: {stderr}"
    );

    // Verify compilation worked by checking logs
    assert!(
        stderr.contains("Starting compilation") || stderr.contains("Compilation successful"),
        "Should show compilation activity. Got stderr: {stderr}"
    );

    // Test execution
    let output = Command::cargo_bin("pecos")?
        .env("RUST_LOG", "info")
        .arg("run")
        .arg(&test_file)
        .arg("-s")
        .arg("1") // Run just 1 shot for the test
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Execution should succeed. Error: {stderr}"
    );

    // For QIR run, check that it produced output
    assert!(
        stdout.contains('[') && stdout.contains(']'),
        "Should output JSON results. Got stdout: {stdout}"
    );

    // Since we changed "Using cached library" to debug level, we can't check for it at info level
    // Instead, just verify the execution succeeded and produced output
    // The JSON output check above is sufficient to verify execution worked

    Ok(())
}
