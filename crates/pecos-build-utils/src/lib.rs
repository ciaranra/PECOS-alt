//! Shared build utilities for pecos-decoders workspace
//!
//! This crate provides common functionality needed by build scripts across
//! the pecos-decoders workspace, including download caching, archive extraction,
//! and dependency management.

use log::{debug, info};

pub mod cache;
pub mod dependencies;
pub mod download;
pub mod errors;
pub mod extract;

// Re-export main types and functions for convenience
pub use cache::get_cache_dir;
pub use dependencies::*;
pub use download::{DownloadInfo, download_all_cached, download_cached};
pub use errors::{BuildError, Result};
pub use extract::extract_archive;

/// Report ccache/sccache configuration for C++ builds
pub fn report_cache_config() {
    info!("Checking C++ compiler cache configuration...");

    // The cc/cxx_build crates respect CC and CXX environment variables
    let cc = std::env::var("CC").unwrap_or_default();
    let cxx = std::env::var("CXX").unwrap_or_default();

    if cc.contains("ccache") || cc.contains("sccache") {
        info!("Using compiler cache via CC: {cc}");
    } else if cxx.contains("ccache") || cxx.contains("sccache") {
        info!("Using compiler cache via CXX: {cxx}");
    } else {
        // Check for RUSTC_WRAPPER which cargo uses for Rust compilation
        if let Ok(wrapper) = std::env::var("RUSTC_WRAPPER") {
            if wrapper.contains("sccache") {
                debug!(
                    "Note: RUSTC_WRAPPER=sccache detected. For C++ caching, also set CC='sccache cc' and CXX='sccache c++'"
                );
            } else if wrapper.contains("ccache") {
                debug!(
                    "Note: RUSTC_WRAPPER=ccache detected. For C++ caching, also set CC='ccache cc' and CXX='ccache c++'"
                );
            }
        }
    }

    // Report parallelism
    if let Ok(num_jobs) = std::env::var("NUM_JOBS") {
        info!("Using {num_jobs} parallel jobs for C++ compilation");
    }
}
