use log::info;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Initialize logger for build script
    env_logger::init();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Find or build libhelios_selene_interface.a
    find_or_build_helios_lib(&out_dir);

    // Export paths for Selene runtime libraries if they're built as dependencies
    #[cfg(feature = "selene-runtimes")]
    export_selene_runtime_paths();

    // Tell cargo to rerun this build script if pecos-qis-ffi changes
    println!("cargo:rerun-if-changed=../pecos-qis-ffi/src");

    // Then build our PECOS shim with undefined __quantum__* symbols
    // These will be resolved at runtime from libpecos_qis_ffi.so/.dylib/.dll
    let source_file = PathBuf::from("src/c/selene_shim.c");
    let output_file = if cfg!(target_os = "macos") {
        out_dir.join("libpecos_selene.dylib")
    } else if cfg!(target_os = "windows") {
        out_dir.join("pecos_selene.dll")
    } else {
        out_dir.join("libpecos_selene.so")
    };

    // Build the C shim as a shared library with undefined __quantum__* symbols
    // These symbols will be resolved from libpecos_qis_ffi.so at runtime
    // Use system clang (not LLVM clang from /tmp/llvm which lacks standard headers)
    // On Unix-like systems, /usr/bin/clang is the system compiler
    let clang_path = if cfg!(target_os = "windows") {
        "clang"
    } else {
        "/usr/bin/clang"
    };
    let mut cmd = Command::new(clang_path);
    cmd.arg("-shared");

    // -fPIC is not supported (and not needed) on Windows MSVC
    #[cfg(not(target_os = "windows"))]
    cmd.arg("-fPIC");

    cmd.arg("-O2").arg("-o").arg(&output_file).arg(&source_file);

    // -lm is not needed on Windows
    #[cfg(not(target_os = "windows"))]
    cmd.arg("-lm");

    // On macOS, we need to allow undefined symbols
    if cfg!(target_os = "macos") {
        cmd.arg("-undefined");
        cmd.arg("dynamic_lookup");
    }

    // On Windows, allow undefined symbols using linker flags
    if cfg!(target_os = "windows") {
        cmd.arg("-Wl,/FORCE:UNRESOLVED");
    }

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

    // Build from Cargo-downloaded Selene dependency
    #[cfg(feature = "selene-runtimes")]
    match build_helios_from_cargo_dependency(out_dir) {
        Ok(()) => {
            println!("cargo:rustc-env=HELIOS_LIB_PATH={}", helios_lib.display());
        }
        Err(e) => {
            panic!("Failed to build Helios interface from Selene dependency: {e}");
        }
    }

    #[cfg(not(feature = "selene-runtimes"))]
    panic!(
        "Failed to build Helios interface library. The selene-runtimes feature must be enabled."
    );
}

/// Build Helios interface library from Cargo-downloaded Selene dependency
#[cfg(feature = "selene-runtimes")]
fn build_helios_from_cargo_dependency(out_dir: &Path) -> Result<(), String> {
    use cargo_metadata::MetadataCommand;

    info!("Building Helios interface from Selene dependency");

    // Get cargo metadata to find Selene source
    let metadata = MetadataCommand::new()
        .exec()
        .map_err(|e| format!("Failed to get cargo metadata: {e}"))?;

    // Find the selene-simple-runtime package (which depends on selene-core)
    let selene_pkg = metadata
        .packages
        .iter()
        .find(|p| p.name == "selene-simple-runtime")
        .ok_or_else(|| "Could not find selene-simple-runtime in cargo metadata".to_string())?;

    // Get the path to the Selene repository root
    // The manifest path is something like .../selene-ext/runtimes/simple/Cargo.toml
    // We need to go up three levels to get to the Selene root
    let manifest_dir = selene_pkg
        .manifest_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| "Could not determine Selene root from manifest path".to_string())?;

    let selene_root = manifest_dir.as_std_path();

    // Build Helios interface from Selene source
    let helios_path = selene_root.join("selene-ext/interfaces/helios_qis");
    let interface_c = helios_path.join("c/src/interface.c");
    let helios_include_dir = helios_path.join("c/include");
    let selene_include_dir = selene_root.join("selene-sim/c/include");

    if !interface_c.exists() {
        return Err(format!(
            "Helios interface.c not found at: {}",
            interface_c.display()
        ));
    }

    let interface_o = out_dir.join("interface.o");
    let helios_lib = out_dir.join("libhelios_selene_interface.a");

    // Compile interface.c to object file
    // Use system clang (not LLVM clang from /tmp/llvm which lacks standard headers)
    // On Unix-like systems, /usr/bin/clang is the system compiler
    let clang_path = if cfg!(target_os = "windows") {
        "clang"
    } else {
        "/usr/bin/clang"
    };
    let mut compile_cmd = Command::new(clang_path);
    compile_cmd.arg("-c");

    // -fPIC is not supported (and not needed) on Windows MSVC
    #[cfg(not(target_os = "windows"))]
    compile_cmd.arg("-fPIC");

    compile_cmd
        .arg("-O2")
        .arg("-std=c11")
        .arg("-D_USE_MATH_DEFINES")
        .arg("-DM_PI=3.14159265358979323846") // Define M_PI directly
        .arg("-DSELENE_LOG_LEVEL=0")
        .arg("-Wno-macro-redefined") // Suppress the redefinition warning
        .arg("-I")
        .arg(&helios_include_dir)
        .arg("-I")
        .arg(&selene_include_dir)
        .arg("-o")
        .arg(&interface_o)
        .arg(&interface_c);

    let output = compile_cmd
        .output()
        .map_err(|e| format!("Failed to execute clang: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to compile interface.c:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Create static library from object file
    let mut ar_cmd = Command::new("ar");
    ar_cmd.arg("rcs").arg(&helios_lib).arg(&interface_o);

    let output = ar_cmd
        .output()
        .map_err(|e| format!("Failed to execute ar: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create libhelios_selene_interface.a:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    info!("Successfully built Helios interface from Selene dependency");

    // Tell cargo to recompile if Selene files change
    println!("cargo:rerun-if-changed={}", interface_c.display());

    Ok(())
}

/// Export environment variables for Selene runtime library paths
#[cfg(feature = "selene-runtimes")]
fn export_selene_runtime_paths() {
    use cargo_metadata::MetadataCommand;

    // Get workspace metadata
    let metadata = MetadataCommand::new()
        .exec()
        .expect("Failed to get cargo metadata");

    // Find the target directory
    let target_dir = metadata.target_directory.as_std_path();

    // Determine the current build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Look for runtime libraries in the current profile first, then fallback
    let profiles = if profile == "release" {
        vec!["release", "debug"]
    } else {
        vec!["debug", "release"]
    };

    let runtime_names = ["selene_simple_runtime", "selene_soft_rz_runtime"];

    for runtime in &runtime_names {
        let mut found = false;
        for profile in &profiles {
            if found {
                break;
            }

            // Check in deps directory first (where cargo puts cdylib dependencies)
            let deps_path = target_dir.join(profile).join("deps");
            if deps_path.exists() {
                // Look for the library with any hash suffix
                if let Ok(entries) = std::fs::read_dir(&deps_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(filename) = path.file_name().and_then(|f| f.to_str())
                            && (filename.starts_with(&format!("lib{runtime}"))
                                || filename.starts_with(runtime))
                            && path.extension().is_some_and(|ext| {
                                if cfg!(target_os = "macos") {
                                    ext.eq_ignore_ascii_case("dylib")
                                } else if cfg!(target_os = "windows") {
                                    ext.eq_ignore_ascii_case("dll")
                                } else {
                                    ext.eq_ignore_ascii_case("so")
                                }
                            })
                        {
                            // Export the path as an environment variable
                            let env_var =
                                format!("PECOS_{}_PATH", runtime.to_uppercase().replace('-', "_"));
                            println!("cargo:rustc-env={}={}", env_var, path.display());
                            info!("Found {} at {}", runtime, path.display());
                            found = true;
                            break;
                        }
                    }
                }
            }

            // Also check the standard location
            if !found {
                let (lib_prefix, lib_ext) = if cfg!(target_os = "macos") {
                    ("lib", "dylib")
                } else if cfg!(target_os = "windows") {
                    ("", "dll")
                } else {
                    ("lib", "so")
                };
                let lib_path = target_dir
                    .join(profile)
                    .join(format!("{lib_prefix}{runtime}.{lib_ext}"));
                if lib_path.exists() {
                    let env_var =
                        format!("PECOS_{}_PATH", runtime.to_uppercase().replace('-', "_"));
                    println!("cargo:rustc-env={}={}", env_var, lib_path.display());
                    info!("Found {} at {}", runtime, lib_path.display());
                    found = true;
                }
            }
        }
    }
}
