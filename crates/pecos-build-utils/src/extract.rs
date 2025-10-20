//! Archive extraction utilities

use crate::errors::{BuildError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Extract a tar.gz or tar.bz2 archive and emit rerun-if-changed for all extracted files
///
/// # Errors
///
/// Returns an error if extraction fails or the expected directory is not found
pub fn extract_archive(
    data: &[u8],
    out_dir: &Path,
    expected_dir_name: Option<&str>,
) -> Result<PathBuf> {
    use tar::Archive;

    // Try to detect if this is gzip or bzip2 by checking magic bytes
    let mut archive = if data.len() >= 3 && data[0] == 0x1f && data[1] == 0x8b && data[2] == 0x08 {
        // gzip magic bytes
        use flate2::read::GzDecoder;
        let tar = GzDecoder::new(data);
        Archive::new(Box::new(tar) as Box<dyn std::io::Read>)
    } else if data.len() >= 3 && &data[0..3] == b"BZh" {
        // bzip2 magic bytes
        use bzip2::read::BzDecoder;
        let tar = BzDecoder::new(data);
        Archive::new(Box::new(tar) as Box<dyn std::io::Read>)
    } else {
        return Err(BuildError::Archive(
            "Unknown archive format - not gzip or bzip2".to_string(),
        ));
    };

    // Extract to temporary directory first
    // On Windows, use a shorter path to avoid MAX_PATH issues with deeply nested archives like Boost
    let temp_dir = if cfg!(windows) {
        // Use Windows temp directory with a short name to minimize path length
        let temp_root = std::env::temp_dir();
        temp_root.join(format!("p{}", std::process::id()))
    } else {
        out_dir.join(format!("extract_temp_{}", std::process::id()))
    };
    fs::create_dir_all(&temp_dir)?;

    // Configure archive for Windows compatibility
    archive.set_preserve_permissions(false);
    archive.set_unpack_xattrs(false);
    archive.unpack(&temp_dir)?;

    // Find the extracted directory
    let entries = fs::read_dir(&temp_dir)?;
    let extracted_dir = entries
        .filter_map(std::result::Result::ok)
        .find(|e| e.file_type().ok().is_some_and(|t| t.is_dir()))
        .ok_or_else(|| BuildError::Archive("No directory found in archive".to_string()))?
        .path();

    // Move to final location
    let final_name = expected_dir_name.unwrap_or("extracted");
    let final_dir = out_dir.join(final_name);

    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)?;
    }

    // On Windows, use copy instead of rename to avoid path length issues
    // fs::rename can fail when destination path is too long on Windows
    #[cfg(windows)]
    {
        copy_dir_all(&extracted_dir, &final_dir)?;
        fs::remove_dir_all(&temp_dir)?;
    }

    #[cfg(not(windows))]
    {
        fs::rename(extracted_dir, &final_dir)?;
        fs::remove_dir_all(&temp_dir)?;
    }

    Ok(final_dir)
}

/// Recursively copy a directory and all its contents
#[cfg(windows)]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
