use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Find or build libhelios_selene_interface.a
    find_or_build_helios_lib(&out_dir);

    // Tell cargo to rerun this build script if pecos-qis-ffi changes
    println!("cargo:rerun-if-changed=../pecos-qis-ffi/src");

    // Then build our PECOS shim with undefined __quantum__* symbols
    // These will be resolved at runtime from libpecos_qis_ffi.so
    let source_file = PathBuf::from("src/c/selene_shim.c");
    let output_file = out_dir.join("libpecos_selene.so");

    // Build the C shim as a shared library with undefined __quantum__* symbols
    // These symbols will be resolved from libpecos_qis_ffi.so at runtime
    let mut cmd = Command::new("clang");
    cmd.arg("-shared")
        .arg("-fPIC")
        .arg("-O2")
        .arg("-o")
        .arg(&output_file)
        .arg(&source_file)
        .arg("-lm");

    // Add include paths if needed
    if let Ok(selene_include) = env::var("SELENE_INCLUDE_PATH") {
        cmd.arg(format!("-I{selene_include}"));
    }

    let output = cmd.output().expect("Failed to execute clang");

    assert!(
        output.status.success(),
        "Failed to compile selene shim:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Set environment variable so Rust code can find the shim
    println!(
        "cargo:rustc-env=PECOS_SELENE_SHIM_PATH={}",
        output_file.display()
    );

    // Tell cargo to recompile if the C source changes
    println!("cargo:rerun-if-changed=src/c/selene_shim.c");
}

fn find_or_build_helios_lib(out_dir: &Path) {
    let helios_lib = out_dir.join("libhelios_selene_interface.a");

    // Check if already exists in our output directory
    if helios_lib.exists() {
        println!("cargo:rustc-env=HELIOS_LIB_PATH={}", helios_lib.display());
        return;
    }

    // Try to find Selene repository
    let possible_paths = [
        PathBuf::from("../../../selene"), // From crate directory
        PathBuf::from("../selene"),       // From workspace root
    ];

    let selene_path = possible_paths.iter().find(|p| p.exists()).cloned();

    if let Some(selene_path) = selene_path {
        // Found Selene, look for pre-built library
        let helios_path = selene_path.join("selene-ext/interfaces/helios_qis");
        let prebuilt_paths = [
            helios_path.join("c/build/libhelios_selene_interface.a"),
            helios_path
                .join("python/selene_helios_qis_plugin/_dist/lib/libhelios_selene_interface.a"),
        ];

        for prebuilt in &prebuilt_paths {
            if prebuilt.exists() {
                // Copy to our output directory
                if std::fs::copy(prebuilt, &helios_lib).is_ok() {
                    println!("cargo:rustc-env=HELIOS_LIB_PATH={}", helios_lib.display());
                    // Also build runtime plugins while we're here
                    build_runtime_plugins(&selene_path);
                    return;
                }
            }
        }
    }

    // If we get here, fall back to building from vendored sources
    build_helios_lib_from_vendor(out_dir);
}

/// Build Selene runtime plugins (.so files) if the Selene repository is available
fn build_runtime_plugins(selene_path: &Path) {
    // List of runtime crates to build
    let runtimes = [
        ("selene-ext/runtimes/simple", "selene_simple_runtime"),
        ("selene-ext/runtimes/soft_rz", "selene_soft_rz_runtime"),
    ];

    for (crate_path, lib_name) in &runtimes {
        let full_path = selene_path.join(crate_path);

        if !full_path.exists() {
            println!(
                "cargo:warning=Runtime crate not found: {}",
                full_path.display()
            );
            continue;
        }

        // Check if .so already exists
        let so_path = selene_path
            .join("target/release")
            .join(format!("lib{lib_name}.so"));
        if so_path.exists() {
            // Already built, skip
            continue;
        }

        println!("cargo:warning=Building Selene runtime: {lib_name}");

        // Build the runtime using cargo
        let output = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--manifest-path")
            .arg(full_path.join("Cargo.toml"))
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("cargo:warning=Successfully built {lib_name}");

                // The .so file should be in ../selene/target/release/
                let so_path = selene_path
                    .join("target/release")
                    .join(format!("lib{lib_name}.so"));
                if so_path.exists() {
                    println!("cargo:warning=Runtime available at: {}", so_path.display());
                } else {
                    println!(
                        "cargo:warning=Warning: Built {lib_name} but .so not found at expected location"
                    );
                }
            }
            Ok(output) => {
                println!(
                    "cargo:warning=Failed to build {}: {}",
                    lib_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                println!("cargo:warning=Error running cargo for {lib_name}: {e}");
            }
        }
    }
}

fn build_helios_lib_from_vendor(out_dir: &Path) {
    let vendor_dir = PathBuf::from("vendor/helios_qis");
    let interface_c = vendor_dir.join("src/interface.c");
    let interface_o = out_dir.join("interface.o");
    let helios_lib = out_dir.join("libhelios_selene_interface.a");

    // Compile interface.c to object file
    let mut compile_cmd = Command::new("clang");
    compile_cmd
        .arg("-c")
        .arg("-fPIC")
        .arg("-O2")
        .arg("-std=c11")
        .arg("-D_USE_MATH_DEFINES") // For M_PI on some platforms
        .arg("-DSELENE_LOG_LEVEL=0")
        .arg("-I")
        .arg(vendor_dir.join("include"))
        .arg("-o")
        .arg(&interface_o)
        .arg(&interface_c);

    let output = compile_cmd
        .output()
        .expect("Failed to execute clang for interface.c");

    assert!(
        output.status.success(),
        "Failed to compile interface.c:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Create static library from object file
    let mut ar_cmd = Command::new("ar");
    ar_cmd.arg("rcs").arg(&helios_lib).arg(&interface_o);

    let output = ar_cmd.output().expect("Failed to execute ar");

    assert!(
        output.status.success(),
        "Failed to create libhelios_selene_interface.a:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Export the path for use in tests
    println!("cargo:rustc-env=HELIOS_LIB_PATH={}", helios_lib.display());

    // Tell cargo to recompile if vendored files change
    println!("cargo:rerun-if-changed=vendor/helios_qis/src/interface.c");
    println!("cargo:rerun-if-changed=vendor/helios_qis/include/helios_qis/interface.h");
    println!("cargo:rerun-if-changed=vendor/helios_qis/include/selene/selene.h");
}
