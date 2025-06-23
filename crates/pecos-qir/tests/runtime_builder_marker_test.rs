use pecos_qir::linker::QirLinker;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::TempDir;

/// Get the path to the marker file
fn get_marker_path() -> PathBuf {
    let base_dir = if let Ok(cargo_home) = env::var("CARGO_HOME") {
        PathBuf::from(cargo_home)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".cargo")
    } else if let Ok(userprofile) = env::var("USERPROFILE") {
        PathBuf::from(userprofile).join(".cargo")
    } else {
        PathBuf::from(".cargo")
    };

    base_dir.join("pecos-qir").join(".needs_rebuild")
}

/// Get the path to the runtime library
fn get_runtime_lib_path() -> PathBuf {
    let base_dir = if let Ok(cargo_home) = env::var("CARGO_HOME") {
        PathBuf::from(cargo_home)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".cargo")
    } else if let Ok(userprofile) = env::var("USERPROFILE") {
        PathBuf::from(userprofile).join(".cargo")
    } else {
        PathBuf::from(".cargo")
    };

    let lib_name = if cfg!(target_os = "windows") {
        "pecos_qir.lib"
    } else {
        "libpecos_qir.a"
    };
    base_dir.join("pecos-qir").join(lib_name)
}

/// Helper to create a simple test QIR file
fn create_test_qir_file(dir: &Path, name: &str) -> PathBuf {
    let qir_file = dir.join(format!("{name}.ll"));
    fs::write(
        &qir_file,
        r"
; Simple QIR test file for marker testing
%Qubit = type opaque
%Result = type opaque

declare void @__quantum__rt__initialize(i8*)
declare %Qubit* @__quantum__rt__qubit_allocate()
declare void @__quantum__rt__qubit_release(%Qubit*)
declare void @__quantum__qis__h__body(%Qubit*)

define void @main() {
entry:
    call void @__quantum__rt__initialize(i8* null)
    %q = call %Qubit* @__quantum__rt__qubit_allocate()
    call void @__quantum__qis__h__body(%Qubit* %q)
    call void @__quantum__rt__qubit_release(%Qubit* %q)
    ret void
}
",
    )
    .unwrap();
    qir_file
}

#[test]
fn test_marker_based_rebuild_system() {
    println!("\n=== Testing marker-based rebuild system (via QirLinker) ===");

    let marker_path = get_marker_path();
    let lib_path = get_runtime_lib_path();

    // Create a test QIR file
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "marker_test");
    let output_dir = test_dir.path().to_path_buf();

    // Step 1: Test normal build (no marker)
    test_normal_build(&marker_path, &lib_path, &qir_file, &output_dir);

    // Step 2: Test with marker file
    test_build_with_marker(&marker_path, &lib_path, &qir_file, &output_dir, &test_dir);

    // Step 3: Test without changes (should use existing library)
    test_no_rebuild(&lib_path, &qir_file, &output_dir);

    println!("\n=== All marker-based rebuild tests passed! ===\n");
}

fn test_normal_build(marker_path: &Path, lib_path: &Path, qir_file: &Path, output_dir: &Path) {
    println!("\n1. Testing normal build (no marker)...");

    // Remove marker if it exists
    if marker_path.exists() {
        fs::remove_file(marker_path).unwrap();
        println!("   - Removed existing marker file");
    }

    // Check if runtime library exists before
    let lib_existed_before = lib_path.exists();
    println!("   - Runtime library exists before: {lib_existed_before}");

    // Compile QIR (this will trigger runtime build if needed)
    let result = QirLinker::compile(&qir_file, Some(&output_dir));
    assert!(result.is_ok(), "QirLinker::compile() failed: {result:?}");

    // Verify runtime library exists
    assert!(lib_path.exists(), "Runtime library was not created");
    println!("   - Runtime library exists after: true");

    // Verify marker doesn't exist
    assert!(
        !marker_path.exists(),
        "Marker file should not exist after build"
    );
    println!("   - Marker exists after build: false");
}

fn test_build_with_marker(
    marker_path: &Path,
    lib_path: &Path,
    qir_file: &Path,
    output_dir: &Path,
    test_dir: &TempDir,
) {
    println!("\n2. Testing build with marker file...");

    // Create marker file
    fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
    fs::write(marker_path, "rebuild needed").unwrap();
    println!("   - Created marker file");
    assert!(marker_path.exists(), "Failed to create marker file");

    // Get library modification time before rebuild
    let (lib_mtime_before, lib_size_before) = prepare_rebuild_test(lib_path, test_dir);

    // Verify marker exists right before compilation
    assert!(
        marker_path.exists(),
        "Marker disappeared before compilation!"
    );
    println!("   - Marker confirmed to exist before compilation");

    // Compile QIR again (should trigger runtime rebuild due to marker)
    println!("   - Starting QIR compilation...");
    let result2 = QirLinker::compile(&qir_file, Some(&output_dir));
    assert!(
        result2.is_ok(),
        "QirLinker::compile() failed with marker: {result2:?}"
    );
    println!("   - QIR compilation completed");

    // Verify runtime library was rebuilt
    verify_rebuild(lib_path, marker_path, lib_mtime_before, lib_size_before);
}

fn prepare_rebuild_test(lib_path: &Path, test_dir: &TempDir) -> (Option<SystemTime>, Option<u64>) {
    if lib_path.exists() {
        let mtime = fs::metadata(lib_path).unwrap().modified().unwrap();
        println!("   - Library exists with timestamp: {mtime:?}");

        // Detect timestamp granularity
        let delay = detect_timestamp_granularity(test_dir);
        std::thread::sleep(delay);

        let size = fs::metadata(lib_path).unwrap().len();
        (Some(mtime), Some(size))
    } else {
        println!("   - Library doesn't exist yet");
        (None, None)
    }
}

fn detect_timestamp_granularity(test_dir: &TempDir) -> std::time::Duration {
    let temp_file = test_dir.path().join("timestamp_test");
    fs::write(&temp_file, "test1").unwrap();
    let t1 = fs::metadata(&temp_file).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    fs::write(&temp_file, "test2").unwrap();
    let t2 = fs::metadata(&temp_file).unwrap().modified().unwrap();
    fs::remove_file(&temp_file).unwrap();

    if t1 == t2 {
        println!("   - Filesystem has coarse timestamp granularity, waiting 1.1s");
        std::time::Duration::from_millis(1100)
    } else {
        println!("   - Filesystem has fine timestamp granularity, waiting 150ms");
        std::time::Duration::from_millis(150)
    }
}

fn verify_rebuild(
    lib_path: &Path,
    marker_path: &Path,
    lib_mtime_before: Option<SystemTime>,
    lib_size_before: Option<u64>,
) {
    if let Some(mtime_before) = lib_mtime_before {
        let lib_mtime_after = fs::metadata(lib_path).unwrap().modified().unwrap();
        let lib_size_after = fs::metadata(lib_path).unwrap().len();

        // Debug output for timing issues
        println!("   - Library mtime before: {mtime_before:?}");
        println!("   - Library mtime after:  {lib_mtime_after:?}");
        println!("   - Library size before: {lib_size_before:?}");
        println!("   - Library size after:  {lib_size_after}");
        println!(
            "   - Marker exists after compilation: {}",
            marker_path.exists()
        );

        // Check if library was actually rebuilt
        let was_rebuilt = lib_mtime_after > mtime_before
            || (lib_size_before.is_some() && lib_size_before.unwrap() != lib_size_after);

        assert!(
            was_rebuilt,
            "Runtime library was not rebuilt despite marker. Before: {mtime_before:?}, After: {lib_mtime_after:?}"
        );
        println!("   - Runtime library was rebuilt");
    } else {
        println!("   - Runtime library was created (didn't exist before)");
    }

    // Verify marker was removed
    assert!(
        !marker_path.exists(),
        "Marker file was not removed after rebuild"
    );
    println!("   - Marker was removed after rebuild");
}

fn test_no_rebuild(lib_path: &Path, qir_file: &Path, output_dir: &Path) {
    println!("\n3. Testing build without changes...");

    // Get runtime library modification time
    let lib_mtime_before = fs::metadata(lib_path).unwrap().modified().unwrap();

    // Compile again without marker
    let result = QirLinker::compile(&qir_file, Some(&output_dir));
    assert!(result.is_ok(), "QirLinker::compile() failed: {result:?}");

    let lib_mtime_after = fs::metadata(lib_path).unwrap().modified().unwrap();
    assert_eq!(
        lib_mtime_before, lib_mtime_after,
        "Runtime library was rebuilt unnecessarily"
    );
    println!("   - Runtime library was not rebuilt (same timestamp)");
}
