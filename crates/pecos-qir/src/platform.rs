//! Platform-specific implementations for QIR compilation
//!
//! This module contains platform-specific code for compiling QIR programs,
//! separated to improve maintainability and organization.

use std::path::PathBuf;

// Import platform-specific modules
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

// Re-export platform-specific implementations (for backwards compatibility)
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
pub use macos::*;

/// Get standard LLVM installation paths for the current platform
#[must_use]
pub fn standard_llvm_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            // CI environment - GitHub Actions might install LLVM here
            PathBuf::from("D:\\a\\_temp\\llvm\\bin"),
            // Standard installation paths
            PathBuf::from("C:\\Program Files\\LLVM\\bin"),
            PathBuf::from("C:\\Program Files (x86)\\LLVM\\bin"),
            // Common Windows package manager locations
            PathBuf::from("C:\\msys64\\mingw64\\bin"),
            PathBuf::from("C:\\msys64\\usr\\bin"),
        ]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/lib/llvm/bin"),
        ]
    }

    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/opt/llvm/bin"),
        ]
    }
}

/// Get platform-specific executable name
#[must_use]
pub fn executable_name(tool_name: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("{tool_name}.exe")
    }

    #[cfg(not(target_os = "windows"))]
    {
        tool_name.to_string()
    }
}
