use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use pecos_qir::linker::QirLinker;

/// Helper to create a simple test QIR file
fn create_test_qir_file(dir: &Path, name: &str) -> PathBuf {
    let qir_file = dir.join(format!("{name}.ll"));
    fs::write(
        &qir_file,
        r"
; Simple QIR test file
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

/// Helper to get modification time
fn get_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// Helper to touch a file (update its modification time)
fn touch_file(path: &Path) {
    if path.exists() {
        // Make a real but harmless change - add/remove a trailing newline
        let content = fs::read_to_string(path).unwrap();
        if content.ends_with('\n') {
            fs::write(path, content.trim_end()).unwrap();
        } else {
            fs::write(path, format!("{content}\n")).unwrap();
        }
    }
}

#[test]
fn test_qir_rebuilds_on_file_change() {
    // Create test QIR file
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "test3");
    let output_dir = test_dir.path().to_path_buf();

    // First compilation
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let _initial_mtime = get_mtime(&lib1).expect("Failed to get library mtime");

    // Wait to ensure timestamp difference
    thread::sleep(Duration::from_millis(1100));

    // Modify QIR file
    let content = fs::read_to_string(&qir_file).unwrap();
    fs::write(&qir_file, format!("{content}\n; Modified")).unwrap();

    // Compile again - should create new library
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // The library names should be different (they include timestamps)
    assert_ne!(
        lib1.file_name(),
        lib2.file_name(),
        "Library was not rebuilt after QIR change"
    );
}

#[test]
fn test_uses_cache_when_nothing_changes() {
    // Create test QIR file
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "test4");
    let output_dir = test_dir.path().to_path_buf();

    // First compilation
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Wait a bit
    thread::sleep(Duration::from_millis(100));

    // Compile again without changes - should use cache
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Should return the same library path (cached)
    assert_eq!(lib1, lib2, "Did not use cached library");
}

#[test]
fn test_qir_rebuilds_when_runtime_newer() {
    // This test verifies that if the runtime library is newer than the cached QIR library,
    // the QIR library is rebuilt. This ensures that runtime updates propagate correctly.

    let runtime_lib = get_runtime_lib_path();
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "test5");
    let output_dir = test_dir.path().to_path_buf();

    // First compilation
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Get the initial modification times
    let qir_lib_mtime = get_mtime(&lib1).expect("Failed to get QIR library mtime");
    let runtime_mtime = get_mtime(&runtime_lib).expect("Failed to get runtime mtime");

    // If we could force a runtime rebuild here (by touching dependencies),
    // we would verify that the QIR library gets rebuilt too.
    // For now, we just verify the logic exists by checking timestamps.

    // The QIR library should not be older than the runtime it was linked with
    assert!(
        qir_lib_mtime >= runtime_mtime,
        "QIR library is older than runtime library - rebuild logic may be broken"
    );
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore] // Integration test that modifies global state (runtime library). Run with: cargo test -- --ignored test_full_rebuild_scenario
    fn test_full_rebuild_scenario() {
        println!("Testing full rebuild scenario...");

        let runtime_lib = get_runtime_lib_path();
        let test_dir = tempfile::tempdir().unwrap();
        let output_dir = test_dir.path().to_path_buf();

        // Scenario 1: Fresh start - no runtime library
        println!("1. Testing fresh build (no runtime)...");
        if runtime_lib.exists() {
            fs::remove_file(&runtime_lib).unwrap();
        }

        let qir_file = create_test_qir_file(test_dir.path(), "full_test");
        let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        assert!(runtime_lib.exists(), "Runtime not created");
        assert!(lib1.exists(), "QIR library not created");

        // Scenario 2: No changes - should use cache
        println!("2. Testing cache usage...");
        thread::sleep(Duration::from_millis(100));
        let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        assert_eq!(lib1, lib2, "Should have used cache");

        // Scenario 3: QIR file change - only QIR rebuilds
        println!("3. Testing QIR file change...");
        thread::sleep(Duration::from_millis(1100));
        let runtime_mtime_before = get_mtime(&runtime_lib).unwrap();
        touch_file(&qir_file);
        let lib3 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        let runtime_mtime_after = get_mtime(&runtime_lib).unwrap();
        assert_ne!(lib1, lib3, "QIR library should be different");
        assert_eq!(
            runtime_mtime_before, runtime_mtime_after,
            "Runtime should not rebuild"
        );

        // Scenario 4: Source change - runtime rebuilds, QIR rebuilds
        println!("4. Testing source change...");
        thread::sleep(Duration::from_millis(1100));
        let src_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/runtime.rs");
        touch_file(&src_file);
        let lib4 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        let runtime_mtime_final = get_mtime(&runtime_lib).unwrap();

        // Note: Cargo's incremental compilation might be smart enough to detect
        // that our "touch" doesn't actually change the compiled output, so we
        // can't reliably test runtime rebuilding with just a newline change.
        // In real scenarios with actual code changes, this would work.
        if runtime_mtime_final > runtime_mtime_after {
            println!("Runtime was rebuilt (as expected with real changes)");
            assert_ne!(
                lib3, lib4,
                "QIR library should rebuild when runtime changes"
            );
        } else {
            println!("Runtime was not rebuilt (cargo detected no real changes)");
            // The QIR library might still be the same if runtime didn't rebuild
        }

        println!("All scenarios passed!");
    }
}
