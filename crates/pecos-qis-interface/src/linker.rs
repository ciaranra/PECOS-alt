//! QIS Linker for compiling and linking QIR programs with the QIS interface
//!
//! This module provides functionality to compile QIR (Quantum Intermediate Representation)
//! programs and link them with the QIS interface FFI exports to create loadable modules.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result type for linker operations
pub type Result<T> = std::result::Result<T, LinkerError>;

/// Errors that can occur during linking
#[derive(Debug)]
pub enum LinkerError {
    /// LLVM tools not found
    LlvmNotFound(String),

    /// Compilation failed
    CompilationFailed(String),

    /// Linking failed
    LinkingFailed(String),

    /// IO error
    IoError(std::io::Error),

    /// Invalid input file
    InvalidInput(String),
}

impl std::fmt::Display for LinkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LlvmNotFound(msg) => write!(f, "LLVM tools not found: {}", msg),
            Self::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            Self::LinkingFailed(msg) => write!(f, "Linking failed: {}", msg),
            Self::IoError(err) => write!(f, "IO error: {}", err),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for LinkerError {}

impl From<std::io::Error> for LinkerError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

/// QIS Linker for compiling QIR programs
pub struct QisLinker {
    /// Path to the QIS interface library
    interface_lib_path: Option<PathBuf>,

    /// Cache directory for compiled programs
    cache_dir: PathBuf,
}

impl QisLinker {
    /// Create a new QIS linker
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pecos-qis-cache");

        Self {
            interface_lib_path: None,
            cache_dir,
        }
    }

    /// Set the cache directory
    pub fn with_cache_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.cache_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Set the interface library path
    pub fn with_interface_lib(mut self, path: impl AsRef<Path>) -> Self {
        self.interface_lib_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Compile a QIR program to a shared library
    ///
    /// # Arguments
    /// * `qir_path` - Path to the QIR (.ll) file
    /// * `output_name` - Optional name for the output library (without extension)
    ///
    /// # Returns
    /// Path to the compiled shared library
    pub fn compile(
        &self,
        qir_path: impl AsRef<Path>,
        output_name: Option<&str>,
    ) -> Result<PathBuf> {
        let qir_path = qir_path.as_ref();

        // Validate input
        if !qir_path.exists() {
            return Err(LinkerError::InvalidInput(format!(
                "QIR file does not exist: {}",
                qir_path.display()
            )));
        }

        if !qir_path.extension().map_or(false, |e| e == "ll") {
            return Err(LinkerError::InvalidInput(
                "Input file must be a .ll (LLVM IR) file".to_string()
            ));
        }

        // Determine output name
        let output_name = output_name.unwrap_or_else(|| {
            qir_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("qis_program")
        });

        // Create cache directory
        fs::create_dir_all(&self.cache_dir)?;

        // Check if we have a cached version
        let output_path = self.get_output_path(output_name);

        if self.is_cache_valid(&output_path, qir_path)? {
            log::debug!("Using cached library: {}", output_path.display());
            return Ok(output_path);
        }

        // Compile QIR to object file
        log::info!("Compiling QIR: {}", qir_path.display());
        let object_path = self.compile_to_object(qir_path)?;

        // Link with interface library
        log::info!("Linking with QIS interface");
        self.link_to_library(&object_path, &output_path)?;

        // Clean up object file
        let _ = fs::remove_file(&object_path);

        Ok(output_path)
    }

    /// Get the output path for a given program name
    fn get_output_path(&self, name: &str) -> PathBuf {
        let extension = Self::platform_extension();
        self.cache_dir.join(format!("{}.{}", name, extension))
    }

    /// Check if cached library is valid
    fn is_cache_valid(&self, output: &Path, source: &Path) -> Result<bool> {
        if !output.exists() {
            return Ok(false);
        }

        let output_time = fs::metadata(output)?.modified()?;
        let source_time = fs::metadata(source)?.modified()?;

        // Check if source is newer than output
        if source_time > output_time {
            log::debug!("Source is newer than cached library");
            return Ok(false);
        }

        // If we have an interface library, check if it's newer
        if let Some(lib_path) = &self.interface_lib_path {
            if lib_path.exists() {
                let lib_time = fs::metadata(lib_path)?.modified()?;
                if lib_time > output_time {
                    log::debug!("Interface library is newer than cached library");
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Compile QIR to object file
    fn compile_to_object(&self, qir_path: &Path) -> Result<PathBuf> {
        let object_path = self.cache_dir.join(format!(
            "{}.o",
            qir_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("temp")
        ));

        // Find LLVM compiler
        let llc = Self::find_llvm_tool("llc")?;

        // Compile to object file
        let output = Command::new(&llc)
            .arg("-filetype=obj")
            .arg("-o")
            .arg(&object_path)
            .arg(qir_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LinkerError::CompilationFailed(format!(
                "llc failed: {}",
                stderr
            )));
        }

        Ok(object_path)
    }

    /// Link object file with interface library to create shared library
    fn link_to_library(&self, object_path: &Path, output_path: &Path) -> Result<()> {
        let mut cmd = if cfg!(target_os = "macos") {
            let mut cmd = Command::new("clang");
            cmd.arg("-shared")
               .arg("-undefined")
               .arg("dynamic_lookup");
            cmd
        } else if cfg!(target_os = "windows") {
            let mut cmd = Command::new("link");
            cmd.arg("/DLL");
            cmd
        } else {
            // Linux
            let mut cmd = Command::new("clang");
            cmd.arg("-shared")
               .arg("-fPIC");
            cmd
        };

        // Add object file
        cmd.arg(object_path);

        // Add interface library if provided
        if let Some(lib_path) = &self.interface_lib_path {
            cmd.arg(lib_path);
        }

        // Set output
        if cfg!(target_os = "windows") {
            cmd.arg(format!("/OUT:{}", output_path.display()));
        } else {
            cmd.arg("-o").arg(output_path);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LinkerError::LinkingFailed(format!(
                "Linker failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Find an LLVM tool in the system
    fn find_llvm_tool(tool: &str) -> Result<PathBuf> {
        // Try versioned tools first (llc-18, llc-17, etc.)
        for version in [18, 17, 16, 15, 14].iter() {
            let versioned = format!("{}-{}", tool, version);
            if let Ok(output) = Command::new("which").arg(&versioned).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        log::debug!("Found LLVM tool: {}", path);
                        return Ok(PathBuf::from(path));
                    }
                }
            }
        }

        // Try unversioned tool
        if let Ok(output) = Command::new("which").arg(tool).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    log::debug!("Found LLVM tool: {}", path);
                    return Ok(PathBuf::from(path));
                }
            }
        }

        Err(LinkerError::LlvmNotFound(format!(
            "Could not find LLVM tool: {}",
            tool
        )))
    }

    /// Get platform-specific shared library extension
    fn platform_extension() -> &'static str {
        if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        }
    }
}

impl Default for QisLinker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linker_creation() {
        let linker = QisLinker::new();
        assert!(linker.cache_dir.to_string_lossy().contains("pecos-qis-cache"));
    }

    #[test]
    fn test_platform_extension() {
        let ext = QisLinker::platform_extension();
        assert!(["so", "dylib", "dll"].contains(&ext));
    }
}