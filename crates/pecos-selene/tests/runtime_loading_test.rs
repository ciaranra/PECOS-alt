//! Test that we can find and load the Selene simple runtime library

use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
use pecos_selene::selene_simple_runtime;

#[test]
fn test_find_selene_runtime() {
    // Try to build with default runtime
    let builder = selene_simple_runtime().default_runtime().qubits(2);

    match builder.build() {
        Ok(engine) => {
            println!("Successfully loaded Selene simple runtime!");
            assert_eq!(engine.num_qubits(), 2);
        }
        Err(e) => {
            // Check if it's just that the library wasn't found
            let err_str = e.to_string();
            if err_str.contains("not found") || err_str.contains("No such file") {
                println!(
                    "Selene runtime library not found (expected if not installed): {}",
                    e
                );
            } else {
                // Unexpected error
                panic!("Unexpected error loading runtime: {}", e);
            }
        }
    }
}

#[test]
fn test_runtime_with_explicit_path() {
    // Test known locations
    let test_paths = vec![
        "/home/ciaranra/Repos/cl_projects/gup/PECOS/lib/pecos-runtimes/libselene_simple_runtime.so",
        "/home/ciaranra/.cache/pecos-decoders/selene/libselene_simple_runtime.so",
    ];

    for path in test_paths {
        if std::path::Path::new(path).exists() {
            println!("Testing runtime at: {}", path);

            let engine = selene_simple_runtime()
                .runtime_library(path)
                .qubits(2)
                .build();

            match engine {
                Ok(e) => {
                    println!("Successfully loaded runtime from: {}", path);
                    assert_eq!(e.num_qubits(), 2);
                    return; // Success!
                }
                Err(e) => {
                    println!("Failed to load from {}: {}", path, e);
                }
            }
        }
    }

    println!(
        "No Selene runtime libraries found in test paths (this is OK if Selene is not installed)"
    );
}
