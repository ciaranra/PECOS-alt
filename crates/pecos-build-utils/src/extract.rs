//! Archive extraction utilities

use crate::errors::{BuildError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Extract a tar.gz archive and emit rerun-if-changed for all extracted files
pub fn extract_archive(
    data: &[u8],
    out_dir: &Path,
    expected_dir_name: Option<&str>,
) -> Result<PathBuf> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let tar = GzDecoder::new(data);
    let mut archive = Archive::new(tar);

    // Extract to temporary directory first
    let temp_dir = out_dir.join(format!("extract_temp_{}", std::process::id()));
    fs::create_dir_all(&temp_dir)?;
    archive.unpack(&temp_dir)?;

    // Find the extracted directory
    let entries = fs::read_dir(&temp_dir)?;
    let extracted_dir = entries
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
        .ok_or_else(|| BuildError::Archive("No directory found in archive".to_string()))?
        .path();

    // Move to final location
    let final_name = expected_dir_name.unwrap_or("extracted");
    let final_dir = out_dir.join(final_name);

    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)?;
    }

    fs::rename(extracted_dir, &final_dir)?;
    fs::remove_dir_all(&temp_dir)?;

    Ok(final_dir)
}
