//! Tests for QIR executable rebuild scenarios
//!
//! This test suite verifies that QIR executables are properly cached and
//! rebuilt when necessary due to source changes or runtime updates.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use pecos_qir::linker::QirLinker;
use serial_test::serial;

/// Create a simple test QIR file
fn create_test_qir_file(dir: &Path, name: &str, content_suffix: &str) -> PathBuf {
    let qir_file = dir.join(format!("{name}.ll"));
    fs::write(
        &qir_file,
        format!(
            r"; Test QIR file {content_suffix}
%Qubit = type opaque
%Result = type opaque

declare void @__quantum__rt__initialize(i8*)
declare %Qubit* @__quantum__rt__qubit_allocate()
declare void @__quantum__rt__qubit_release(%Qubit*)
declare void @__quantum__qis__h__body(%Qubit*)

define void @main() {{
entry:
    call void @__quantum__rt__initialize(i8* null)
    %q = call %Qubit* @__quantum__rt__qubit_allocate()
    call void @__quantum__qis__h__body(%Qubit* %q)
    call void @__quantum__rt__qubit_release(%Qubit* %q)
    ret void
}}
"
        ),
    )
    .unwrap();
    qir_file
}

/// Get modification time of a file
fn get_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// Touch a file to update its modification time
fn touch_file(path: &Path) {
    if path.exists() {
        let content = fs::read(path).unwrap();
        fs::write(path, content).unwrap();
    }
}

#[test]
#[serial]
fn test_qir_executable_caching() {
    println!("\n=== Testing QIR executable caching ===");

    let test_dir = tempfile::tempdir().unwrap();
    let output_dir = test_dir.path().join("build");
    let qir_file = create_test_qir_file(test_dir.path(), "cache_test", "v1");

    // First compilation
    println!("1. First compilation...");
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let lib1_mtime = get_mtime(&lib1).expect("Failed to get library mtime");
    println!("   Created: {:?}", lib1.file_name().unwrap());

    // Wait to ensure any new compilation would have different timestamp
    thread::sleep(Duration::from_millis(1100));

    // Second compilation - should use cache
    println!("2. Second compilation (should use cache)...");
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let lib2_mtime = get_mtime(&lib2).expect("Failed to get library mtime");

    assert_eq!(lib1, lib2, "Expected same library path from cache");
    assert_eq!(
        lib1_mtime, lib2_mtime,
        "Library was rebuilt instead of using cache"
    );
    println!("   Used cached library (same file and timestamp)");
}

#[test]
#[serial]
fn test_qir_rebuild_on_source_change() {
    println!("\n=== Testing QIR rebuild on source change ===");

    let test_dir = tempfile::tempdir().unwrap();
    let output_dir = test_dir.path().join("build");
    let qir_file = create_test_qir_file(test_dir.path(), "source_change_test", "v1");

    // First compilation
    println!("1. Initial compilation...");
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    println!("   Created: {:?}", lib1.file_name().unwrap());

    // Wait to ensure timestamp difference
    thread::sleep(Duration::from_millis(1100));

    // Modify QIR file
    println!("2. Modifying QIR source file...");
    let content = fs::read_to_string(&qir_file).unwrap();
    fs::write(&qir_file, content.replace("v1", "v2")).unwrap();

    // Get the original modification time before recompilation
    let lib1_mtime_before = get_mtime(&lib1).unwrap();

    // Second compilation - should rebuild
    println!("3. Recompiling after source change...");
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Should be the same path (consistent naming)
    assert_eq!(lib1, lib2, "Should use the same library file path");
    assert!(lib2.exists(), "Library should exist");

    // Get the new modification time
    let lib2_mtime_after = get_mtime(&lib2).unwrap();

    // The modification time should be newer
    assert!(
        lib2_mtime_after > lib1_mtime_before,
        "Library should have been rebuilt with newer timestamp"
    );
    println!("   Library was rebuilt after source change");
}

#[test]
#[serial]
fn test_qir_rebuild_on_runtime_update() {
    println!("\n=== Testing QIR rebuild on runtime update ===");

    let test_dir = tempfile::tempdir().unwrap();
    let output_dir = test_dir.path().join("build");
    let qir_file = create_test_qir_file(test_dir.path(), "runtime_update_test", "v1");

    // Get runtime library path
    let runtime_lib = get_runtime_lib_path();
    assert!(
        runtime_lib.exists(),
        "Runtime library must exist for this test"
    );

    // First compilation
    println!("1. Initial compilation...");
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let lib1_mtime = get_mtime(&lib1).unwrap();
    println!("   Created: {:?}", lib1.file_name().unwrap());

    // Wait to ensure timestamp difference
    thread::sleep(Duration::from_millis(1500));

    // Touch the runtime library to make it newer
    println!("2. Simulating runtime library update...");
    touch_file(&runtime_lib);
    let runtime_mtime = get_mtime(&runtime_lib).unwrap();

    // Verify runtime is now newer than the QIR executable
    assert!(runtime_mtime > lib1_mtime, "Failed to make runtime newer");

    // Second compilation - should rebuild because runtime is newer
    println!("3. Recompiling after runtime update...");
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let lib2_mtime = get_mtime(&lib2).unwrap();

    assert_eq!(lib1, lib2, "Should use same library path");
    assert!(
        lib2_mtime > lib1_mtime,
        "Library should have been rebuilt after runtime update"
    );
    assert!(
        lib2_mtime >= runtime_mtime,
        "Rebuilt library should be at least as new as runtime"
    );
    println!("   QIR executable was rebuilt after runtime update");
}

#[test]
#[serial]
fn test_multiple_qir_files_independent_caching() {
    println!("\n=== Testing independent caching for multiple QIR files ===");

    let test_dir = tempfile::tempdir().unwrap();
    let output_dir = test_dir.path().join("build");

    // Create two different QIR files
    let qir1 = create_test_qir_file(test_dir.path(), "file1", "v1");
    let qir2 = create_test_qir_file(test_dir.path(), "file2", "v1");

    // Compile both
    println!("1. Compiling two QIR files...");
    let lib1 = QirLinker::compile(&qir1, Some(&output_dir)).unwrap();
    let lib2 = QirLinker::compile(&qir2, Some(&output_dir)).unwrap();

    assert_ne!(
        lib1, lib2,
        "Different QIR files should produce different libraries"
    );
    println!("   File 1: {:?}", lib1.file_name().unwrap());
    println!("   File 2: {:?}", lib2.file_name().unwrap());

    // Get original modification times
    let lib1_mtime_old = get_mtime(&lib1).unwrap();
    let lib2_mtime_old = get_mtime(&lib2).unwrap();

    // Wait for timestamp difference
    thread::sleep(Duration::from_millis(1100));

    // Modify only the first QIR file
    println!("2. Modifying only the first QIR file...");
    let content = fs::read_to_string(&qir1).unwrap();
    fs::write(&qir1, content.replace("v1", "v2")).unwrap();

    // Recompile both
    println!("3. Recompiling both files...");
    let lib1_new = QirLinker::compile(&qir1, Some(&output_dir)).unwrap();
    let lib2_new = QirLinker::compile(&qir2, Some(&output_dir)).unwrap();

    // Check modification times
    let lib1_mtime_new = get_mtime(&lib1_new).unwrap();
    let lib2_mtime_new = get_mtime(&lib2_new).unwrap();

    assert_eq!(lib1, lib1_new, "Same path for file1");
    assert_eq!(lib2, lib2_new, "Same path for file2");
    assert!(lib1_mtime_new > lib1_mtime_old, "File1 should be rebuilt");
    assert_eq!(lib2_mtime_old, lib2_mtime_new, "File2 should use cache");

    println!("   Only modified QIR file was rebuilt, other used cache");
}

/// Helper to get the runtime library path
fn get_runtime_lib_path() -> PathBuf {
    let base_dir = if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        PathBuf::from(cargo_home)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cargo")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
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
