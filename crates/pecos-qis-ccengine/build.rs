//! Build script to automatically build Selene runtime plugins if available
//!
//! This build script automatically builds the Selene runtime .so files
//! if the Selene repository is found adjacent to PECOS.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Build the Helios interface library (.a file) using Selene
fn build_helios_interface(selene_path: &Path) {
    println!("cargo:warning=Building Selene Helios interface library...");

    // The Helios interface is in selene-ext/interfaces/helios_qis
    let helios_path = selene_path.join("selene-ext/interfaces/helios_qis");

    // Check if the path exists
    if !helios_path.exists() {
        println!("cargo:warning=Helios interface not found at {}, skipping build", helios_path.display());
        return;
    }

    // Output location for the .a file (in our target directory)
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let lib_path = PathBuf::from(&out_dir).join("libhelios.a");

    // Check if already copied
    if lib_path.exists() {
        println!("cargo:warning=Helios interface already available at {}", lib_path.display());
        // Tell cargo where to find it
        println!("cargo:rustc-env=HELIOS_LIB_PATH={}", lib_path.display());
        // Don't link the Helios library at build time to avoid symbol conflicts
        // The Helios interface will use it only when creating executables
        // println!("cargo:rustc-link-search=native={}", out_dir);
        // println!("cargo:rustc-link-lib=static=helios");
        return;
    }

    // Look for pre-built .a file
    let prebuilt_paths = [
        helios_path.join("c/build/libhelios_selene_interface.a"),
        helios_path.join("python/selene_helios_qis_plugin/_dist/lib/libhelios_selene_interface.a"),
    ];

    for prebuilt in &prebuilt_paths {
        if prebuilt.exists() {
            println!("cargo:warning=Found pre-built Helios interface at {}", prebuilt.display());
            // Copy to our output directory
            if let Err(e) = std::fs::copy(prebuilt, &lib_path) {
                println!("cargo:warning=Failed to copy Helios interface: {}", e);
            } else {
                println!("cargo:warning=Copied Helios interface to {}", lib_path.display());
                println!("cargo:rustc-env=HELIOS_LIB_PATH={}", lib_path.display());
                // Don't link the Helios library at build time to avoid symbol conflicts
                // println!("cargo:rustc-link-search=native={}", out_dir);
                // println!("cargo:rustc-link-lib=static=helios");
                return;
            }
        }
    }

    // If no pre-built found, try to build it with CMake
    let cmake_lists = helios_path.join("c/CMakeLists.txt");
    if cmake_lists.exists() {
        println!("cargo:warning=No pre-built Helios interface found, would need to build with CMake");
        println!("cargo:warning=To build manually: cd {} && mkdir -p build && cd build && cmake .. && make",
                 helios_path.join("c").display());
    } else {
        println!("cargo:warning=No pre-built Helios interface found and no CMakeLists.txt");
    }
}

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

    // First, build the Helios interface library
    build_helios_interface(&selene_path);

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