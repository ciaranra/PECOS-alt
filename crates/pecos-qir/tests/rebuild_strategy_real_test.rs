use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use pecos_qir::linker::QirLinker;
use serial_test::serial;

/// Helper to create a test QIR file
fn create_test_qir_file(dir: &Path, name: &str) -> PathBuf {
    let qir_file = dir.join(format!("{name}.ll"));
    fs::write(
        &qir_file,
        r"
%Qubit = type opaque
declare void @__quantum__rt__initialize(i8*)
declare %Qubit* @__quantum__rt__qubit_allocate()

define void @main() {
entry:
    call void @__quantum__rt__initialize(i8* null)
    %q = call %Qubit* @__quantum__rt__qubit_allocate()
    ret void
}
",
    )
    .unwrap();
    qir_file
}

/// Get the runtime library path
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

/// Initialize the runtime library once before tests
fn ensure_runtime_exists() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // Create a dummy QIR file and compile it to ensure runtime exists
        let temp_dir = tempfile::tempdir().unwrap();
        let qir_file = create_test_qir_file(temp_dir.path(), "init");
        let _ = QirLinker::compile(&qir_file, None);
    });
}

#[test]
#[serial]
fn test_cache_really_works() {
    ensure_runtime_exists();

    // This test MUST verify that when nothing changes, we get the EXACT same library
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "cache_test");
    let output_dir = test_dir.path().to_path_buf();

    // First compilation
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let lib1_mtime = fs::metadata(&lib1).unwrap().modified().unwrap();
    println!("First compilation created: {lib1:?}");

    // Wait to ensure any new compilation would have different timestamp
    thread::sleep(Duration::from_millis(1500));

    // Second compilation - should return SAME library
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
    let lib2_mtime = fs::metadata(&lib2).unwrap().modified().unwrap();
    println!("Second compilation returned: {lib2:?}");

    // Verify it's actually the same file
    assert_eq!(lib1, lib2, "Cache broken: got different library paths");
    assert_eq!(
        lib1_mtime, lib2_mtime,
        "Cache broken: library was recreated"
    );
}

#[test]
#[serial]
fn test_qir_change_forces_rebuild() {
    ensure_runtime_exists();

    // This test MUST verify that QIR changes create new libraries
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "rebuild_test");
    let output_dir = test_dir.path().to_path_buf();

    // First compilation
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Wait to ensure different timestamp
    thread::sleep(Duration::from_millis(1500));

    // Make a REAL change to QIR file
    let content = fs::read_to_string(&qir_file).unwrap();
    fs::write(&qir_file, content.replace("ret void", "ret void ; changed")).unwrap();

    // Second compilation - MUST create new library
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Verify new library was created
    assert_ne!(lib1, lib2, "QIR change didn't trigger rebuild");
    assert!(
        lib1.exists() && lib2.exists(),
        "Libraries should both exist"
    );
}

#[test]
#[ignore] // This test modifies global state (deletes the runtime library) and should only be run in isolation
fn test_runtime_missing_forces_build() {
    let runtime_lib = get_runtime_lib_path();

    // Backup existing runtime if it exists
    let backup = runtime_lib.with_extension("backup");
    if runtime_lib.exists() {
        fs::rename(&runtime_lib, &backup).unwrap();
    }

    // Ensure runtime is gone
    assert!(!runtime_lib.exists(), "Runtime should not exist");

    // Create test QIR
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "runtime_test");

    // Compile - should build runtime
    let result = QirLinker::compile(&qir_file, None);

    // Restore backup
    if backup.exists() {
        fs::rename(&backup, &runtime_lib).ok();
    }

    // Verify
    assert!(result.is_ok(), "Compilation failed: {result:?}");
    assert!(runtime_lib.exists(), "Runtime was not built when missing");
}

#[test]
#[serial]
fn test_runtime_newer_forces_qir_rebuild() {
    ensure_runtime_exists();

    // This test verifies our logic that QIR libraries rebuild when runtime is newer
    let runtime_lib = get_runtime_lib_path();
    let test_dir = tempfile::tempdir().unwrap();
    let qir_file = create_test_qir_file(test_dir.path(), "newer_test");
    let output_dir = test_dir.path().to_path_buf();

    // Compile QIR
    let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    // Wait a bit
    thread::sleep(Duration::from_millis(1500));

    // Touch the runtime library to make it newer
    // We'll update its timestamp by copying it to itself
    let runtime_content = fs::read(&runtime_lib).unwrap();
    fs::write(&runtime_lib, &runtime_content).unwrap();

    // Verify runtime is now newer
    let runtime_mtime = fs::metadata(&runtime_lib).unwrap().modified().unwrap();
    let lib1_mtime = fs::metadata(&lib1).unwrap().modified().unwrap();
    assert!(runtime_mtime > lib1_mtime, "Failed to make runtime newer");

    // Compile again - should NOT use cache because runtime is newer
    let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();

    assert_ne!(lib1, lib2, "QIR didn't rebuild when runtime was newer");
}

#[cfg(test)]
mod integration {
    use super::*;

    #[test]
    #[serial]
    fn test_complete_rebuild_flow() {
        ensure_runtime_exists();

        // This test verifies the complete flow without excuses
        let test_dir = tempfile::tempdir().unwrap();
        let output_dir = test_dir.path().to_path_buf();
        let runtime_lib = get_runtime_lib_path();

        // Step 1: Create and compile QIR
        let qir_file = create_test_qir_file(test_dir.path(), "flow_test");
        let lib1 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        println!("Initial compilation: {:?}", lib1.file_name());

        // Step 2: Compile again - should use cache
        thread::sleep(Duration::from_millis(1500));
        let lib2 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        assert_eq!(lib1, lib2, "Cache not working");
        println!("Cache working correctly");

        // Step 3: Change QIR - should rebuild
        thread::sleep(Duration::from_millis(1500));
        let content = fs::read_to_string(&qir_file).unwrap();
        fs::write(&qir_file, format!("{content}\n; Modified")).unwrap();
        let lib3 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
        assert_ne!(lib2, lib3, "QIR change didn't trigger rebuild");
        println!("QIR rebuild working correctly");

        // Step 4: Test runtime dependency
        if runtime_lib.exists() {
            thread::sleep(Duration::from_millis(1500));
            // Make runtime newer by touching it
            let runtime_data = fs::read(&runtime_lib).unwrap();
            fs::write(&runtime_lib, runtime_data).unwrap();

            let lib4 = QirLinker::compile(&qir_file, Some(&output_dir)).unwrap();
            assert_ne!(lib3, lib4, "Runtime update didn't trigger QIR rebuild");
            println!("Runtime dependency working correctly");
        }

        println!("All rebuild scenarios working correctly!");
    }
}
