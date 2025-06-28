//! Tests for concurrent QIR compilation scenarios
//!
//! This test suite verifies that multiple QIR compilations can happen
//! safely in parallel without race conditions.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use pecos_qir::linker::LlvmLinker;

/// Create a simple test QIR file
fn create_test_qir_file(dir: &Path, name: &str) -> PathBuf {
    let qir_file = dir.join(format!("{name}.ll"));
    fs::write(
        &qir_file,
        format!(
            r"; Test HUGR LLVM-IR file: {name}

declare void @__quantum__rt__initialize(i8*)
declare void @__quantum__qis__h__body__hugr(i64)
declare void @__quantum__qis__m__body__hugr(i64, i64)
declare void @__quantum__rt__result_record_output__hugr(i64, i64)

define void @main() {{
entry:
    call void @__quantum__rt__initialize(i8* null)
    call void @__quantum__qis__h__body__hugr(i64 0)
    call void @__quantum__qis__m__body__hugr(i64 0, i64 0)
    call void @__quantum__rt__result_record_output__hugr(i64 0, i64 0)
    ret void
}}
"
        ),
    )
    .unwrap();
    qir_file
}

#[test]
fn test_concurrent_same_file_compilation() {
    println!("\n=== Testing concurrent compilation of same QIR file ===");

    let test_dir = Arc::new(tempfile::tempdir().unwrap());
    let qir_file = Arc::new(create_test_qir_file(test_dir.path(), "concurrent_same"));

    // Spawn multiple threads to compile the same file
    let mut handles = vec![];

    for i in 0..3 {
        let test_dir = Arc::clone(&test_dir);
        let qir_file = Arc::clone(&qir_file);

        let handle = thread::spawn(move || {
            println!("Thread {i} starting compilation...");
            let output_dir = test_dir.path().join(format!("build_{i}"));
            let result = LlvmLinker::compile(qir_file.as_ref(), Some(&output_dir));
            println!("Thread {} finished: {:?}", i, result.is_ok());
            result
        });

        handles.push(handle);
    }

    // Wait for all threads and collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.join().unwrap());
    }

    // All compilations should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Thread {i} compilation failed: {result:?}");
    }

    println!("   All concurrent compilations succeeded");
}

#[test]
fn test_concurrent_different_files_compilation() {
    println!("\n=== Testing concurrent compilation of different QIR files ===");

    let test_dir = Arc::new(tempfile::tempdir().unwrap());

    // Create multiple QIR files
    let qir_files: Vec<_> = (0..4)
        .map(|i| Arc::new(create_test_qir_file(test_dir.path(), &format!("file_{i}"))))
        .collect();

    // Spawn threads to compile different files
    let mut handles = vec![];

    for (i, qir_file) in qir_files.into_iter().enumerate() {
        let test_dir = Arc::clone(&test_dir);

        let handle = thread::spawn(move || {
            println!("Thread {i} compiling file_{i}.ll...");
            let output_dir = test_dir.path().join("build");
            let result = LlvmLinker::compile(qir_file.as_ref(), Some(&output_dir));
            println!("Thread {} finished: {:?}", i, result.is_ok());
            (i, result)
        });

        handles.push(handle);
    }

    // Wait for all threads and collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.join().unwrap());
    }

    // All compilations should succeed
    for (thread_id, result) in &results {
        assert!(
            result.is_ok(),
            "Thread {thread_id} compilation failed: {result:?}"
        );

        // Verify the compiled library exists
        if let Ok(lib_path) = result {
            assert!(
                lib_path.exists(),
                "Library for thread {thread_id} doesn't exist"
            );
        }
    }

    println!("   All files compiled successfully in parallel");
}

#[test]
fn test_runtime_library_concurrent_access() {
    use std::sync::Barrier;

    println!("\n=== Testing concurrent runtime library access ===");

    // This test verifies that multiple threads can safely call RuntimeBuilder
    // at the same time (through LlvmLinker::compile)

    let test_dir = Arc::new(tempfile::tempdir().unwrap());

    // Create a barrier to synchronize thread starts
    let barrier = Arc::new(Barrier::new(3));

    let mut handles = vec![];

    for i in 0..3 {
        let test_dir = Arc::clone(&test_dir);
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            // Create a unique QIR file for this thread
            let qir_file = create_test_qir_file(test_dir.path(), &format!("runtime_test_{i}"));

            // Wait for all threads to be ready
            barrier.wait();

            // Now all threads will try to access RuntimeBuilder simultaneously
            println!("Thread {i} accessing runtime library...");
            let output_dir = test_dir.path().join(format!("build_{i}"));
            let result = LlvmLinker::compile(&qir_file, Some(&output_dir));

            println!("Thread {} completed: {:?}", i, result.is_ok());
            result
        });

        handles.push(handle);
    }

    // Wait for all threads
    let mut all_succeeded = true;
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.join().unwrap() {
            Ok(_) => println!("   Thread {i} succeeded"),
            Err(e) => {
                println!("   Thread {i} failed: {e:?}");
                all_succeeded = false;
            }
        }
    }

    assert!(all_succeeded, "Not all threads succeeded");
    println!("   Runtime library handled concurrent access correctly");
}
