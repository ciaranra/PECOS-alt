//! Build script for pecos-llvm-runtime
//!
//! This build script is part of a sophisticated rebuild strategy for managing
//! two types of artifacts: the static runtime library and LLVM executables.
//!
//! # Complete Rebuild Strategy Overview
//!
//! The system manages two types of artifacts:
//!
//! ## 1. Static Runtime Library (`~/.cargo/pecos-llvm-runtime/libpecos_llvm_runtime.a`)
//!
//! A static library containing all pecos-llvm-runtime symbols needed by LLVM IR programs.
//! This is built once and cached, only rebuilding when source changes.
//!
//! ## 2. LLVM Executables (in user-specified directories)
//!
//! Compiled LLVM IR programs linked with the runtime library. Each LLVM IR file
//! gets its own cached executable that's rebuilt when either the LLVM IR
//! source or runtime library changes.
//!
//! # The Three-Phase Approach
//!
//! ## Phase 1: Detection (this build.rs script)
//!
//! Runs during `cargo build/test/check` to detect if runtime rebuild is needed:
//! - Checks if the static library exists at `~/.cargo/pecos-llvm-runtime/libpecos_llvm_runtime.a`
//! - Compares library timestamp against source files in `src/`
//! - Creates marker file (`~/.cargo/pecos-llvm-runtime/.needs_rebuild`) if outdated
//! - Removes marker if everything is current
//!
//! ## Phase 2: Runtime Building (`RuntimeBuilder`)
//!
//! When LLVM IR compilation is requested:
//! - Checks for missing library OR marker file
//! - Builds static library if needed using a wrapper crate
//! - Removes marker file after successful build
//!
//! ## Phase 3: LLVM IR Compilation (`LlvmLinker`)
//!
//! The main compilation flow:
//! 1. Check for cached LLVM executable
//! 2. Ensure runtime library is built (calls `RuntimeBuilder`)
//! 3. Compare timestamps: executable vs LLVM IR source and runtime
//! 4. Rebuild executable if any dependency is newer
//!
//! # Why This Complex Approach?
//!
//! We can't use simpler approaches due to Rust/Cargo limitations:
//!
//! ## Why Not Build Static Library in Cargo.toml?
//!
//! Adding `crate-type = ["rlib", "staticlib"]` to generate both library types
//! causes doc tests to fail. Cargo has known issues with multiple crate types,
//! especially when one is `staticlib`. This makes the straightforward approach
//! unusable for a library that needs documentation.
//!
//! ## Why Not Build in build.rs Directly?
//!
//! Building the static library directly in build.rs would require:
//! 1. Creating a wrapper crate that depends on pecos-llvm-runtime
//! 2. Building that crate from within pecos-llvm-runtime's build.rs
//! 3. This creates a circular dependency: pecos-llvm-runtime -> build.rs -> wrapper -> pecos-llvm-runtime
//!
//! Even with careful dependency management, this leads to deadlocks and
//! infinite recursion in Cargo's dependency resolver.
//!
//! ## The Marker File Solution
//!
//! By deferring the actual build to runtime (when QIR compilation happens):
//! 1. build.rs only creates a marker file (no circular deps)
//! 2. The runtime library is built only when actually needed
//! 3. Normal `cargo build/test` works without issues
//! 4. Doc tests work normally
//! 5. Users get automatic rebuilds without manual intervention
//!
//! This approach leverages Cargo's change detection while avoiding its
//! limitations around static library generation.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Track dependencies for rebuild detection
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let lib_path = get_lib_path();
    let marker_path = get_marker_path();

    // Check if rebuild is needed
    let needs_rebuild = !lib_path.exists() || is_library_outdated(&lib_path);

    if needs_rebuild {
        // Create marker file to signal rebuild is needed
        if let Some(parent) = marker_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&marker_path, "rebuild");
    } else {
        // Remove marker if library is up to date
        let _ = fs::remove_file(&marker_path);
    }

    // Track the library so we rebuild if it's deleted
    if lib_path.exists() {
        println!("cargo:rerun-if-changed={}", lib_path.display());
    }
}

fn is_library_outdated(lib_path: &Path) -> bool {
    let Ok(lib_metadata) = fs::metadata(lib_path) else {
        return true;
    };

    let Ok(lib_modified) = lib_metadata.modified() else {
        return true;
    };

    // Check if any source file is newer than the library
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    is_dir_newer_than(&src_dir, lib_modified)
}

fn is_dir_newer_than(dir: &Path, time: std::time::SystemTime) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified > time {
                        return true;
                    }
                }

                // Recursively check subdirectories
                if metadata.is_dir() && is_dir_newer_than(&entry.path(), time) {
                    return true;
                }
            }
        }
    }
    false
}

fn get_lib_path() -> PathBuf {
    let base_dir = env::var("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|_| env::var("HOME").map(|h| PathBuf::from(h).join(".cargo")))
        .or_else(|_| env::var("USERPROFILE").map(|h| PathBuf::from(h).join(".cargo")))
        .unwrap_or_else(|_| PathBuf::from(".cargo"));

    let lib_name = if cfg!(target_os = "windows") {
        "pecos_llvm_runtime.lib"
    } else {
        "libpecos_llvm_runtime.a"
    };

    base_dir.join("pecos-llvm-runtime").join(lib_name)
}

fn get_marker_path() -> PathBuf {
    let base_dir = env::var("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|_| env::var("HOME").map(|h| PathBuf::from(h).join(".cargo")))
        .or_else(|_| env::var("USERPROFILE").map(|h| PathBuf::from(h).join(".cargo")))
        .unwrap_or_else(|_| PathBuf::from(".cargo"));

    base_dir.join("pecos-llvm-runtime").join(".needs_rebuild")
}
