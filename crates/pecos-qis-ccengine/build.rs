//! Build script to automatically build Selene runtime plugins if available
//!
//! This build script automatically builds the Selene runtime .so files
//! if the Selene repository is found adjacent to PECOS.

use std::path::PathBuf;
use std::process::Command;

fn main() {

    // Check if Selene repository exists
    // Try multiple possible locations
    let possible_paths = [
        PathBuf::from("../../../selene"),  // From crate directory
        PathBuf::from("../selene"),         // From workspace root
    ];

    let selene_path = possible_paths
        .iter()
        .find(|p| p.exists())
        .cloned();

    let selene_path = match selene_path {
        Some(path) => path,
        None => {
            // Selene not found - this is fine, just skip building
            return;
        }
    };

    println!("cargo:warning=Found Selene repository at {}, building runtime plugins...", selene_path.display());

    // List of runtime crates to build
    let runtimes = [
        ("selene-ext/runtimes/simple", "selene_simple_runtime"),
        ("selene-ext/runtimes/soft_rz", "selene_soft_rz_runtime"),
    ];

    for (crate_path, lib_name) in &runtimes {
        let full_path = selene_path.join(crate_path);

        if !full_path.exists() {
            println!("cargo:warning=Runtime crate not found: {}", full_path.display());
            continue;
        }

        // Check if .so already exists
        let so_path = selene_path.join("target/release").join(format!("lib{}.so", lib_name));
        if so_path.exists() {
            // Already built, skip
            continue;
        }

        println!("cargo:warning=Building Selene runtime: {}", lib_name);

        // Build the runtime using cargo
        let output = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(full_path.join("Cargo.toml"))
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("cargo:warning=Successfully built {}", lib_name);

                // The .so file should be in ../selene/target/release/
                let so_path = selene_path.join("target/release").join(format!("lib{}.so", lib_name));
                if so_path.exists() {
                    println!("cargo:warning=Runtime available at: {}", so_path.display());
                } else {
                    println!("cargo:warning=Warning: Built {} but .so not found at expected location", lib_name);
                }
            }
            Ok(output) => {
                println!("cargo:warning=Failed to build {}: {}",
                    lib_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                println!("cargo:warning=Error running cargo for {}: {}", lib_name, e);
            }
        }
    }
}