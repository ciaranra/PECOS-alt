//! Trait definition for QIS interfaces
//!
//! A QisInterface is responsible for executing a quantum program and
//! collecting the quantum operations that need to be performed.
//! Different implementations can use different execution strategies:
//! - JIT compilation and execution
//! - Linking with Helios and executing
//! - Direct interpretation
//! etc.

use crate::{CollectedOperations, Operation};
use std::collections::HashMap;

/// Result type for interface operations
pub type Result<T> = std::result::Result<T, InterfaceError>;

/// Errors that can occur during interface execution
#[derive(Debug, Clone)]
pub enum InterfaceError {
    /// Compilation failed
    CompilationError(String),

    /// Linking failed
    LinkingError(String),

    /// Execution failed
    ExecutionError(String),

    /// FFI error
    FfiError(String),

    /// Program not loaded
    NoProgramLoaded,

    /// Other error
    Other(String),
}

impl std::fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilationError(msg) => write!(f, "Compilation error: {}", msg),
            Self::LinkingError(msg) => write!(f, "Linking error: {}", msg),
            Self::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
            Self::FfiError(msg) => write!(f, "FFI error: {}", msg),
            Self::NoProgramLoaded => write!(f, "No program loaded"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for InterfaceError {}

/// Trait for QIS interface implementations
///
/// An interface is responsible for executing a quantum program and
/// collecting the operations that need to be performed by the quantum simulator.
pub trait QisInterface: Send + Sync {
    /// Load a program for execution
    ///
    /// The program format depends on the interface implementation:
    /// - JIT interfaces might take LLVM IR
    /// - Helios interfaces might take bitcode
    /// - etc.
    fn load_program(&mut self, program: &[u8]) -> Result<()>;

    /// Execute the program to collect operations
    ///
    /// This runs the program in "collection mode" to gather all
    /// quantum operations without actually performing them.
    fn collect_operations(&mut self) -> Result<CollectedOperations>;

    /// Execute with measurement results
    ///
    /// This runs the program with actual measurement results to handle
    /// conditional execution paths.
    fn execute_with_measurements(
        &mut self,
        measurements: HashMap<usize, bool>,
    ) -> Result<CollectedOperations>;

    /// Get metadata about the interface
    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Check if a program is loaded
    fn has_program(&self) -> bool;

    /// Reset the interface for a new execution
    fn reset(&mut self) -> Result<()>;
}

/// Builder trait for creating QIS interfaces
///
/// This allows for a fluent API when constructing interfaces
/// with various configuration options.
pub trait QisInterfaceBuilder: Send + Sync {
    /// The type of interface this builder creates
    type Interface: QisInterface;

    /// Build the interface
    fn build(self) -> Result<Self::Interface>;

    /// Get the name of this interface type
    fn name(&self) -> &'static str;
}