//! Error types for build scripts

use thiserror::Error;

/// Build script error type
#[derive(Error, Debug)]
pub enum BuildError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    /// Download error
    #[error("Download error: {0}")]
    Download(String),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Archive extraction error
    #[error("Archive extraction error: {0}")]
    Archive(String),

    /// SHA256 verification error
    #[error("SHA256 mismatch: expected {expected}, got {actual}")]
    Sha256Mismatch { expected: String, actual: String },
}

/// Result type alias for build scripts
pub type Result<T> = std::result::Result<T, BuildError>;
