use pecos_engines::engine_system::MonteCarloEngine;
use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_qis_runtime::QisEngine;
use std::path::PathBuf;

/// Get the path to the HUGR Bell state example
fn get_llvm_program_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .parent()
        .expect("Expected to find workspace directory as parent of crates/");
    workspace_dir.join("examples/llvm/bell.ll")
}

#[test]
fn test_qis_bell_state_single_worker() {
    // Create an LLVM engine directly with the file path
    let llvm_engine = QisEngine::new(get_llvm_program_path());

    // Create a noiseless model
    let noise_model = Box::new(DepolarizingNoiseModel::new_uniform(0.0));

    // Run the Bell state example with 10 shots and 1 worker (single-threaded)
    let results = MonteCarloEngine::run_with_noise_model(
        Box::new(llvm_engine),
        noise_model,
        10,
        1,    // Single worker to test basic functionality
        None, // No specific seed
    )
    .expect("LLVM execution should succeed with single worker");

    // The test passes if there are no errors in execution
    assert!(!results.shots.is_empty(), "Expected non-empty results");
    println!(
        "Single-threaded LLVM execution succeeded with {} shots",
        results.shots.len()
    );
}
