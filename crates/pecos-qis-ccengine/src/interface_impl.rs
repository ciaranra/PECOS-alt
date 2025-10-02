//! Interface trait for QIS program execution
//!
//! This module defines the QisInterface trait that different implementations
//! (JIT, Helios, etc.) must implement to execute quantum programs.

use pecos_qis_interface::QisInterface as OperationList;  // The old struct
use pecos_core::prelude::PecosError;
use std::collections::HashMap;

/// Trait for QIS interface implementations
///
/// A QisInterface is responsible for executing a quantum program and
/// collecting the quantum operations that need to be performed.
///
/// Different implementations:
/// - `QisJitInterface` - Uses LLVM JIT compilation
/// - `QisSeleneHeliosInterface` - Links with Selene's Helios interface
pub trait QisInterface: Send + Sync {
    /// Load a program into the interface
    ///
    /// The format depends on the implementation:
    /// - JIT: LLVM IR text or bitcode
    /// - Helios: QIS bitcode or HUGR bytes
    fn load_program(&mut self, program_bytes: &[u8], format: ProgramFormat) -> Result<(), PecosError>;

    /// Execute the program to collect operations
    ///
    /// This runs the program in "collection mode" to discover all quantum
    /// operations without actually performing quantum simulation.
    fn collect_operations(&mut self) -> Result<OperationList, PecosError>;

    /// Execute with measurement results
    ///
    /// This runs the program with specific measurement results to handle
    /// conditional execution paths correctly.
    fn execute_with_measurements(
        &mut self,
        measurements: HashMap<usize, bool>,
    ) -> Result<OperationList, PecosError>;

    /// Get metadata about the implementation
    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Get the name of this implementation
    fn name(&self) -> &'static str;

    /// Reset the interface for a new execution
    fn reset(&mut self) -> Result<(), PecosError>;
}

/// Program format for loading
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgramFormat {
    /// LLVM IR text
    LlvmIrText,
    /// LLVM bitcode
    LlvmBitcode,
    /// HUGR bytes
    HugrBytes,
    /// QIS bitcode (Selene format)
    QisBitcode,
}

/// Box type for interface implementations
pub type BoxedInterface = Box<dyn QisInterface>;

/// Simple wrapper for pre-built QisInterface data structures
///
/// This allows pre-built QisInterface instances to be used directly
/// with the QisControlEngine without needing compilation.
pub struct SimpleQisInterface {
    operations: OperationList,
}

impl SimpleQisInterface {
    /// Create a new SimpleQisInterface from a pre-built operations list
    pub fn new(operations: OperationList) -> Self {
        Self { operations }
    }
}

impl QisInterface for SimpleQisInterface {
    fn load_program(&mut self, _program_bytes: &[u8], _format: ProgramFormat) -> Result<(), PecosError> {
        // Pre-built interface doesn't need to load programs
        Ok(())
    }

    fn collect_operations(&mut self) -> Result<OperationList, PecosError> {
        // Return the pre-built operations
        Ok(self.operations.clone())
    }

    fn execute_with_measurements(
        &mut self,
        _measurements: HashMap<usize, bool>,
    ) -> Result<OperationList, PecosError> {
        // For pre-built interfaces, just return the operations as-is
        // since there are no conditional paths
        Ok(self.operations.clone())
    }

    fn name(&self) -> &'static str {
        "Simple (Pre-built)"
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        // Nothing to reset for pre-built interface
        Ok(())
    }
}