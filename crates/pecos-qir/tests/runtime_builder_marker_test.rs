use pecos_qir::linker::QirLinker;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

    // Step 1: Test normal build (no marker)
    println!("\n1. Testing normal build (no marker)...");

    // Remove marker if it exists
    if marker_path.exists() {
        fs::remove_file(&marker_path).unwrap();
        println!("   - Removed existing marker file");
    }

    // Check if runtime library exists before
    let lib_existed_before = lib_path.exists();
    println!("   - Runtime library exists before: {lib_existed_before}");

    // Compile QIR (this will trigger runtime build if needed)
    let output_dir = test_dir.path().to_path_buf();
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

    // Step 2: Test with marker file
    println!("\n2. Testing build with marker file...");

    // Get library modification time before
    let lib_mtime_before = if lib_path.exists() {
        Some(fs::metadata(&lib_path).unwrap().modified().unwrap())
    } else {
        None
    };

    // Create marker file
    fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
    fs::write(&marker_path, "rebuild needed").unwrap();
    println!("   - Created marker file");
    assert!(marker_path.exists(), "Failed to create marker file");

    // Wait a bit to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(1100));

    // Compile QIR again (should trigger runtime rebuild due to marker)
    let result2 = QirLinker::compile(&qir_file, Some(&output_dir));
    assert!(
        result2.is_ok(),
        "QirLinker::compile() failed with marker: {result2:?}"
    );

    // Verify runtime library was rebuilt (has newer timestamp) if it existed before
    if let Some(mtime_before) = lib_mtime_before {
        let lib_mtime_after = fs::metadata(&lib_path).unwrap().modified().unwrap();
        assert!(
            lib_mtime_after > mtime_before,
            "Runtime library was not rebuilt despite marker"
        );
        println!("   - Runtime library was rebuilt (newer timestamp)");
    } else {
        println!("   - Runtime library was created (didn't exist before)");
    }

    // Verify marker was removed
    assert!(
        !marker_path.exists(),
        "Marker file was not removed after rebuild"
    );
    println!("   - Marker was removed after rebuild");

    // Step 3: Test without changes (should use existing library)
    println!("\n3. Testing build without changes...");

    // Get runtime library modification time
    let lib_mtime_before_3 = fs::metadata(&lib_path).unwrap().modified().unwrap();

    // Compile again without marker
    let result3 = QirLinker::compile(&qir_file, Some(&output_dir));
    assert!(result3.is_ok(), "QirLinker::compile() failed: {result3:?}");

    let lib_mtime_after_3 = fs::metadata(&lib_path).unwrap().modified().unwrap();
    assert_eq!(
        lib_mtime_before_3, lib_mtime_after_3,
        "Runtime library was rebuilt unnecessarily"
    );
    println!("   - Runtime library was not rebuilt (same timestamp)");

    println!("\n=== All marker-based rebuild tests passed! ===\n");
}
