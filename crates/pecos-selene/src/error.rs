//! Error types for the Selene engine integration

use pecos_core::prelude::PecosError;
use thiserror::Error;

/// Errors that can occur when using the Selene engine
#[derive(Error, Debug)]
pub enum SeleneError {
    /// No program was specified
    #[error("No program specified for Selene engine")]
    NoProgramSpecified,
    
    /// Number of qubits not specified
    #[error("Number of qubits not specified")]
    QubitCountNotSpecified,
    
    /// Invalid program format
    #[error("Invalid program format: {0}")]
    InvalidProgramFormat(String),
    
    /// Selene runtime error
    #[error("Selene runtime error: {0}")]
    RuntimeError(String),
    
    /// Simulator error
    #[error("Simulator error: {0}")]
    SimulatorError(String),
    
    /// Compilation error
    #[error("Compilation error: {0}")]
    CompilationError(String),
    
    /// Instance not initialized
    #[error("Selene instance not initialized")]
    InstanceNotInitialized,
    
    /// Unexpected state
    #[error("Unexpected state: {0}")]
    UnexpectedState(String),
    
    /// Empty program provided
    #[error("Empty program provided")]
    EmptyProgram,
    
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(std::path::PathBuf),
    
    /// HUGR-related error
    #[error("HUGR error: {0}")]
    HugrError(String),
    
    /// Unsupported program type
    #[error("Unsupported program type: {0}")]
    UnsupportedProgram(String),
    
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

impl From<SeleneError> for PecosError {
    fn from(err: SeleneError) -> Self {
        PecosError::Processing(err.to_string())
    }
}