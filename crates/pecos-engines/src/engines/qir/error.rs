use crate::errors::QueueError;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Error type for QIR engine operations
///
/// This enum represents the various errors that can occur during QIR engine operations.
/// It provides more specific error types than the generic `QueueError`, making error
/// handling more explicit and self-documenting.
#[derive(Debug)]
pub enum QirError {
    /// The QIR file was not found at the specified path
    FileNotFound(PathBuf),

    /// The QIR file exists but is empty
    EmptyFile(PathBuf),

    /// Failed to read the QIR file
    FileReadError {
        /// Path to the QIR file
        path: PathBuf,
        /// The underlying IO error
        error: std::io::Error,
    },

    /// Failed to compile the QIR program
    CompilationFailed(String),

    /// Failed to load the QIR library
    LibraryLoadFailed(String),

    /// Failed to call a function in the QIR library
    LibraryCallFailed(String),

    /// No qubit allocations were found in the QIR file
    NoQubitAllocationsFound(PathBuf),

    /// Failed to create a temporary directory
    TempDirCreationFailed(std::io::Error),

    /// Failed to copy the library to a thread-specific path
    LibraryCopyFailed {
        /// Source path
        source: PathBuf,
        /// Destination path
        destination: PathBuf,
        /// The underlying IO error
        error: std::io::Error,
    },

    /// Failed to get commands from the QIR library
    GetCommandsFailed(String),

    /// No QIR library is loaded
    NoLibraryLoaded,

    /// Failed to process measurements
    MeasurementProcessingFailed(String),

    /// Failed to generate commands
    CommandGenerationFailed(String),

    /// Other unspecified error
    Other(String),
}

impl fmt::Display for QirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "QIR file not found: {}", path.display()),
            Self::EmptyFile(path) => write!(f, "QIR file is empty: {}", path.display()),
            Self::FileReadError { path, error } => {
                write!(f, "Failed to read QIR file {}: {}", path.display(), error)
            }
            Self::CompilationFailed(msg) => write!(f, "QIR compilation failed: {msg}"),
            Self::LibraryLoadFailed(msg) => write!(f, "Failed to load QIR library: {msg}"),
            Self::LibraryCallFailed(msg) => {
                write!(f, "Failed to call function in QIR library: {msg}")
            }
            Self::NoQubitAllocationsFound(path) => write!(
                f,
                "No qubit allocations found in QIR file: {}",
                path.display()
            ),
            Self::TempDirCreationFailed(error) => {
                write!(f, "Failed to create temporary directory: {error}")
            }
            Self::LibraryCopyFailed {
                source,
                destination,
                error,
            } => {
                write!(
                    f,
                    "Failed to copy library from {} to {}: {}",
                    source.display(),
                    destination.display(),
                    error
                )
            }
            Self::GetCommandsFailed(msg) => {
                write!(f, "Failed to get commands from QIR library: {msg}")
            }
            Self::NoLibraryLoaded => write!(f, "No QIR library loaded"),
            Self::MeasurementProcessingFailed(msg) => {
                write!(f, "Failed to process measurements: {msg}")
            }
            Self::CommandGenerationFailed(msg) => {
                write!(f, "Failed to generate commands: {msg}")
            }
            Self::Other(msg) => write!(f, "QIR error: {msg}"),
        }
    }
}

impl Error for QirError {}

impl From<QirError> for QueueError {
    fn from(error: QirError) -> Self {
        QueueError::OperationError(error.to_string())
    }
}

/// Helper function to create a file not found error
///
/// # Arguments
///
/// * `path` - The path to the file that was not found
///
/// # Returns
///
/// A `QirError::FileNotFound` error
#[must_use]
pub fn file_not_found(path: PathBuf) -> QirError {
    QirError::FileNotFound(path)
}

/// Helper function to create an empty file error
///
/// # Arguments
///
/// * `path` - The path to the empty file
///
/// # Returns
///
/// A `QirError::EmptyFile` error
#[must_use]
pub fn empty_file(path: PathBuf) -> QirError {
    QirError::EmptyFile(path)
}

/// Helper function to create a file read error
///
/// # Arguments
///
/// * `path` - The path to the file that could not be read
/// * `error` - The underlying IO error
///
/// # Returns
///
/// A `QirError::FileReadError` error
#[must_use]
pub fn file_read_error(path: PathBuf, error: std::io::Error) -> QirError {
    QirError::FileReadError { path, error }
}

/// Helper function to create a library load failed error
///
/// # Arguments
///
/// * `msg` - The error message
///
/// # Returns
///
/// A `QirError::LibraryLoadFailed` error
pub fn library_load_failed<S: Into<String>>(msg: S) -> QirError {
    QirError::LibraryLoadFailed(msg.into())
}

/// Helper function to create a library call failed error
///
/// # Arguments
///
/// * `msg` - The error message
///
/// # Returns
///
/// A `QirError::LibraryCallFailed` error
pub fn library_call_failed<S: Into<String>>(msg: S) -> QirError {
    QirError::LibraryCallFailed(msg.into())
}

/// Helper function to create a no qubit allocations found error
///
/// # Arguments
///
/// * `path` - The path to the QIR file
///
/// # Returns
///
/// A `QirError::NoQubitAllocationsFound` error
#[must_use]
pub fn no_qubit_allocations_found(path: PathBuf) -> QirError {
    QirError::NoQubitAllocationsFound(path)
}

/// Helper function to create a get commands failed error
///
/// # Arguments
///
/// * `msg` - The error message
///
/// # Returns
///
/// A `QirError::GetCommandsFailed` error
pub fn get_commands_failed<S: Into<String>>(msg: S) -> QirError {
    QirError::GetCommandsFailed(msg.into())
}
