/// File validation utilities
use pecos_core::errors::PecosError;
use std::fs;
use std::path::Path;

/// Minimum size for a valid library file (1KB)
const MIN_LIBRARY_SIZE: u64 = 1024;

/// Validate a library file for loading
///
/// # Errors
///
/// Returns `PecosError::Resource` if:
/// - The file does not exist
/// - Failed to get file metadata
/// - The path is not a regular file
/// - The file is smaller than the minimum library size (1KB)
pub fn validate_library_file(path: &Path) -> Result<u64, PecosError> {
    // Check if file exists
    if !path.exists() {
        return Err(PecosError::Resource(format!(
            "Library file not found: {}",
            path.display()
        )));
    }

    // Get and validate metadata
    let metadata = fs::metadata(path).map_err(|e| {
        PecosError::Resource(format!(
            "Failed to get file metadata for {}: {}",
            path.display(),
            e
        ))
    })?;

    // Check if it's a regular file
    if !metadata.is_file() {
        return Err(PecosError::Resource(format!(
            "Not a regular file: {}",
            path.display()
        )));
    }

    // Check file size
    let file_size = metadata.len();
    if file_size < MIN_LIBRARY_SIZE {
        return Err(PecosError::Resource(format!(
            "File too small to be a valid library: {} (size: {} bytes, minimum: {} bytes)",
            path.display(),
            file_size,
            MIN_LIBRARY_SIZE
        )));
    }

    Ok(file_size)
}
