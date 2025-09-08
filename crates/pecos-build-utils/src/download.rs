//! Download utilities with caching and integrity verification

use crate::cache::get_cache_dir;
use crate::errors::{BuildError, Result};
use std::fs;

/// Download info with URL and expected SHA256
pub struct DownloadInfo {
    pub url: String,
    pub sha256: &'static str,
    pub name: String,
}

/// Download a file with caching and integrity verification
///
/// # Errors
///
/// Returns an error if unable to download the file or if verification fails
pub fn download_cached(info: &DownloadInfo) -> Result<Vec<u8>> {
    let cache_dir = get_cache_dir()?;
    let cache_file = cache_dir.join(format!("{}-{}.tar.gz", info.name, &info.sha256[..8]));

    // Check if we have a valid cached file
    if cache_file.exists() {
        // Try to read the cached file
        match fs::read(&cache_file) {
            Ok(data) => {
                // Verify integrity
                if verify_sha256(&data, info.sha256).is_ok() {
                    return Ok(data);
                }
                println!("cargo:warning=Cached file corrupted, re-downloading");
                let _ = fs::remove_file(&cache_file); // Ignore removal errors
            }
            Err(e) => {
                println!("cargo:warning=Failed to read cached file: {e}, re-downloading");
                let _ = fs::remove_file(&cache_file); // Try to remove unreadable file
            }
        }
    }

    // Download fresh
    println!("cargo:warning=Downloading {} (will be cached)", info.name);
    let response =
        reqwest::blocking::get(&info.url).map_err(|e| BuildError::Http(e.to_string()))?;

    if !response.status().is_success() {
        return Err(BuildError::Download(format!(
            "Failed with status: {}",
            response.status()
        )));
    }

    let data = response
        .bytes()
        .map_err(|e| BuildError::Http(e.to_string()))?
        .to_vec();

    // Verify integrity
    verify_sha256(&data, info.sha256)?;

    // Save to cache
    fs::write(&cache_file, &data)?;
    println!("cargo:warning=Cached to {}", cache_file.display());

    Ok(data)
}

/// Verify SHA256 hash of data
fn verify_sha256(data: &[u8], expected: &str) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let actual = format!("{result:x}");

    if actual == expected {
        Ok(actual)
    } else {
        Err(BuildError::Sha256Mismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}

/// Download multiple files concurrently
///
/// # Errors
///
/// Returns an error if any download fails
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn download_all_cached(downloads: Vec<DownloadInfo>) -> Result<Vec<(String, Vec<u8>)>> {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let results = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));

    let handles: Vec<_> = downloads
        .into_iter()
        .map(|info| {
            let results = Arc::clone(&results);
            let errors = Arc::clone(&errors);

            thread::spawn(move || match download_cached(&info) {
                Ok(data) => {
                    results.lock().unwrap().push((info.name.clone(), data));
                }
                Err(e) => {
                    errors.lock().unwrap().push(format!("{}: {}", info.name, e));
                }
            })
        })
        .collect();

    // Wait for all downloads
    for handle in handles {
        handle.join().unwrap();
    }

    // Check for errors
    let errors = errors.lock().unwrap();
    if !errors.is_empty() {
        return Err(BuildError::Download(format!(
            "Download failures:\n{}",
            errors.join("\n")
        )));
    }

    Ok(Arc::try_unwrap(results).unwrap().into_inner().unwrap())
}
