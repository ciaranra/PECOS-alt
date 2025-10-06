//! Selene Runtime Shim
//!
//! This module provides the C shim library that implements selene_* functions
//! and forwards them to PECOS's thread-local interface.
//!
//! The shim is compiled as a shared library (libpecos_selene_shim.so) that
//! provides the selene_* symbols expected by libhelios.a.

// The actual shim is implemented in C (src/c/selene_shim.c)
// This module just provides Rust-side utilities if needed

/// Get the path to the compiled shim library
///
/// The shim is compiled by build.rs and placed in the output directory
pub fn get_shim_library_path() -> Option<std::path::PathBuf> {
    // The build script will set this environment variable
    std::env::var("PECOS_SELENE_SHIM_PATH")
        .ok()
        .map(std::path::PathBuf::from)
}
