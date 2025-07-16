//! Unified engine builder for LLVM that integrates with the common simulation API
//!
//! This module provides the engine builder that implements the `ClassicalControlEngineBuilder`
//! trait from pecos-engines, enabling the unified simulation API.

use crate::source::LlvmSource;
use hugr_core::Hugr;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_llvm_runtime::{LlvmEngine, LlvmEngineConfig};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Builder for LLVM engines that integrates with the unified simulation API
#[derive(Debug, Clone, Default)]
pub struct LlvmEngineBuilder {
    /// The source of LLVM IR or HUGR
    source: Option<LlvmSource>,
    /// Maximum number of qubits allowed
    max_qubits: Option<usize>,
    /// Verbose output
    verbose: bool,
}

impl LlvmEngineBuilder {
    /// Create a new LLVM engine builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source to LLVM IR text (human-readable format)
    pub fn llvm_ir(mut self, ir: impl Into<String>) -> Self {
        self.source = Some(LlvmSource::LlvmIr(ir.into()));
        self
    }
    
    /// Set the source to LLVM bitcode (binary format)
    pub fn llvm_bitcode(mut self, bitcode: impl Into<Vec<u8>>) -> Self {
        self.source = Some(LlvmSource::LlvmBitcode(bitcode.into()));
        self
    }

    /// Set the source to LLVM file (auto-detects .ll or .bc extension)
    pub fn llvm_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the source to LLVM IR text file (.ll)
    pub fn llvm_ir_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmIrFile(path.as_ref().to_path_buf()));
        self
    }
    
    /// Set the source to LLVM bitcode file (.bc)
    pub fn llvm_bitcode_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::LlvmBitcodeFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set the source to HUGR
    pub fn hugr(mut self, hugr: Hugr) -> Self {
        self.source = Some(LlvmSource::Hugr(Box::new(hugr)));
        self
    }

    /// Set the source to HUGR bytes
    pub fn hugr_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.source = Some(LlvmSource::HugrBytes(bytes));
        self
    }

    /// Set the source to HUGR file
    pub fn hugr_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(LlvmSource::HugrFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set maximum number of qubits allowed for allocation
    pub fn max_qubits(mut self, max_qubits: usize) -> Self {
        self.max_qubits = Some(max_qubits);
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

impl ClassicalControlEngineBuilder for LlvmEngineBuilder {
    type Engine = LlvmEngine;

    fn build(self) -> Result<Self::Engine, PecosError> {
        // Get source or error
        let source = self.source.ok_or_else(|| {
            PecosError::Input(
                "No source specified. Use .llvm_ir(), .llvm_bitcode(), .hugr(), or similar method.".to_string(),
            )
        })?;

        // Convert source to LLVM IR
        let llvm_ir = source.to_llvm_ir()?;

        // Create temporary file for LLVM IR
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| PecosError::with_context(e, "Failed to create temp file for LLVM IR"))?;

        std::io::Write::write_all(&mut temp_file, llvm_ir.as_bytes())
            .map_err(|e| PecosError::with_context(e, "Failed to write LLVM IR to temp file"))?;

        temp_file
            .flush()
            .map_err(|e| PecosError::with_context(e, "Failed to flush LLVM IR to temp file"))?;

        // We need to keep the temp file from being deleted, so we use persist
        let (_, path) = temp_file
            .keep()
            .map_err(|e| PecosError::with_context(e, "Failed to persist temp file"))?;

        // Create LLVM engine with configuration
        let engine_config = LlvmEngineConfig {
            assigned_shots: 0,
            verbose: self.verbose,
            max_qubits: self.max_qubits,
        };

        let engine = LlvmEngine::with_config(path, engine_config);

        Ok(engine)
    }
}

/// Create a new LLVM engine builder
///
/// This is the entry point for the unified simulation API.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use pecos_llvm_sim::engine_builder::llvm_engine;
/// use pecos_engines::{ClassicalControlEngineBuilder, DepolarizingNoise};
///
/// let results = llvm_engine()
///     .llvm_ir("define void @main() { ret void }")
///     .to_sim()
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(1000)?;
/// # Ok(())
/// # }
/// ```
pub fn llvm_engine() -> LlvmEngineBuilder {
    LlvmEngineBuilder::new()
}