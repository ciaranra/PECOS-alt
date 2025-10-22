//! Cache directory management for build artifacts

use crate::errors::Result;
use std::fs;
use std::path::PathBuf;

/// Get the persistent cache directory for build artifacts
/// Works across Windows, macOS, and Linux
///
/// # Errors
///
/// Returns an error if unable to determine a cache directory on the system
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = if let Ok(dir) = std::env::var("PECOS_CACHE_DIR") {
        // Allow override via environment variable
        PathBuf::from(dir)
    } else if let Some(dir) = dirs::cache_dir() {
        // Use system cache directory
        // - Linux: ~/.cache/pecos-decoders
        // - macOS: ~/Library/Caches/pecos-decoders
        // - Windows: C:\Users\{user}\AppData\Local\pecos-decoders\cache
        dir.join("pecos-decoders")
    } else {
        // Fallback to target directory
        PathBuf::from(std::env::var("OUT_DIR")?).join(".cache")
    };

    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}
