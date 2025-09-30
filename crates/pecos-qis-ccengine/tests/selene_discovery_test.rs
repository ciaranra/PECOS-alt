//! Test that we can discover Selene runtimes in the adjacent repository

use pecos_qis_ccengine::{
    find_selene_runtime,
    qis_control_engine,
    selene_runtime,
};
use std::path::PathBuf;

#[test]
fn test_find_selene_runtimes_in_repo() {
    // List of Selene runtime plugins
    let plugins = [
        "simple_runtime",
        "soft_rz_runtime",
    ];

    println!("Searching for Selene runtimes in adjacent repository...");

    let mut found_count = 0;
    for plugin_name in &plugins {
        if let Some(path) = find_selene_runtime(plugin_name) {
            println!("  Found: {} at {}", plugin_name, path.display());
            found_count += 1;

            // Verify it's from the Selene repo
            let path_str = path.to_string_lossy();
            if path_str.contains("selene/") {
                println!("    -> From Selene repository");
            }
        } else {
            println!("  Not found: {}", plugin_name);
        }
    }

    println!("\nFound {}/{} Selene plugins", found_count, plugins.len());

    // If any are found, it shows our discovery is working
    if found_count > 0 {
        println!("Successfully discovering Selene plugins from adjacent repository!");
    }
}

#[test]
fn test_load_selene_runtime_from_repo() {
    // Try to find and load the simple runtime
    if let Some(path) = find_selene_runtime("simple_runtime") {
        println!("Found simple_runtime at: {}", path.display());

        // Try to create a runtime with it
        let runtime = selene_runtime(&path);

        // Try to create an engine with it
        let _builder = qis_control_engine().runtime(runtime);

        // This would fail if the library can't be loaded, but that's OK for this test
        // We're just verifying the discovery and API works
        println!("Successfully created builder with Selene runtime from: {}", path.display());
    } else {
        println!("Simple runtime not found in Selene repository (expected if not built)");
    }
}

#[test]
fn test_selene_repo_paths() {
    // Test that our search includes the Selene repo paths
    let selene_release = PathBuf::from("../selene/target/release");
    let selene_debug = PathBuf::from("../selene/target/debug");
    let compiler_release = PathBuf::from("../selene/selene-compilers/hugr_qis/target/release");

    println!("Checking Selene repository paths:");
    println!("  Release dir exists: {}", selene_release.exists());
    println!("  Debug dir exists: {}", selene_debug.exists());
    println!("  Compiler dir exists: {}", compiler_release.exists());

    if selene_release.exists() {
        // List .so files in the release directory
        println!("\nSelene release directory contents:");
        if let Ok(entries) = std::fs::read_dir(&selene_release) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("libselene_") && name_str.ends_with(".so") {
                        println!("  - {}", name_str);
                    }
                }
            }
        }
    }
}