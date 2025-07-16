//! Unified engine builder for Selene that integrates with the common simulation API
//!
//! This module provides the engine builder that implements the `ClassicalControlEngineBuilder`
//! trait from pecos-engines, enabling the unified simulation API.

use crate::{
    selene_engine::SeleneEngine,
    program::SeleneProgram,
};
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use std::path::Path;

/// Builder for Selene engines that integrates with the unified simulation API
#[derive(Debug, Clone, Default)]
pub struct SeleneEngineBuilder {
    /// The program source
    program: Option<SeleneProgram>,
    /// Number of qubits
    num_qubits: Option<usize>,
    /// Whether to optimize the program
    optimize: bool,
    /// Verbose output
    verbose: bool,
}

impl SeleneEngineBuilder {
    /// Create a new Selene engine builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the program from a HUGR
    #[cfg(feature = "hugr")]
    pub fn hugr(mut self, hugr: hugr::Hugr) -> Self {
        self.program = Some(SeleneProgram::Hugr(hugr));
        self
    }
    
    /// Set the program from LLVM IR text (human-readable format)
    pub fn llvm_ir(mut self, ir: impl Into<String>) -> Self {
        self.program = Some(SeleneProgram::LlvmIr(ir.into()));
        self
    }
    
    /// Set the program from LLVM bitcode (binary format)
    pub fn llvm_bitcode(mut self, bitcode: impl Into<Vec<u8>>) -> Self {
        self.program = Some(SeleneProgram::LlvmBitcode(bitcode.into()));
        self
    }
    
    /// Set the program from an LLVM file (auto-detects .ll or .bc)
    pub fn llvm_file(mut self, path: impl AsRef<Path>) -> Self {
        self.program = Some(SeleneProgram::LlvmFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the program from an LLVM IR text file (.ll)
    pub fn llvm_ir_file(mut self, path: impl AsRef<Path>) -> Self {
        self.program = Some(SeleneProgram::LlvmIrFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the program from an LLVM bitcode file (.bc)
    pub fn llvm_bitcode_file(mut self, path: impl AsRef<Path>) -> Self {
        self.program = Some(SeleneProgram::LlvmBitcodeFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the program from a HUGR file
    #[cfg(feature = "hugr")]
    pub fn hugr_file(mut self, path: impl AsRef<Path>) -> Self {
        self.program = Some(SeleneProgram::HugrFile(path.as_ref().to_path_buf()));
        self
    }
    
    
    /// Set the number of qubits to allocate
    pub fn qubits(mut self, n: usize) -> Self {
        self.num_qubits = Some(n);
        self
    }
    
    /// Enable optimization
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }
    
    /// Enable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

impl ClassicalControlEngineBuilder for SeleneEngineBuilder {
    type Engine = SeleneEngine;

    fn build(self) -> Result<Self::Engine, PecosError> {
        let program = self.program.ok_or_else(|| {
            PecosError::Input(
                "No program specified. Use .llvm_ir(), .hugr(), .selene(), or similar method.".to_string(),
            )
        })?;

        let num_qubits = self.num_qubits.ok_or_else(|| {
            PecosError::Input(
                "Number of qubits not specified. Use .qubits() to set the number of qubits.".to_string(),
            )
        })?;

        // Build the Selene engine
        Ok(SeleneEngine::new(program, num_qubits, self.optimize))
    }
}

/// Create a new Selene engine builder
///
/// This is the entry point for the unified simulation API.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use pecos_selene_ceng::engine_builder::selene_engine;
/// use pecos_engines::{ClassicalControlEngineBuilder, DepolarizingNoise};
///
/// let results = selene_engine()
///     .llvm_ir("define void @main() { ret void }")
///     .qubits(2)
///     .to_sim()
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(1000)?;
/// # Ok(())
/// # }
/// ```
pub fn selene_engine() -> SeleneEngineBuilder {
    SeleneEngineBuilder::new()
}