//! Unified engine builder for LLVM that integrates with the common simulation API
//!
//! This module provides the engine builder that implements the `ClassicalControlEngineBuilder`
//! trait from pecos-engines, enabling the unified simulation API.

use crate::source::QisSource;
use tket::hugr::Hugr;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_qis_runtime::{QisEngine, QisEngineConfig};
use pecos_programs::{HugrProgram, QisProgram};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Program source types that can be converted to LLVM engine source
pub enum ProgramSource {
    Qis(QisProgram),
    Hugr(HugrProgram),
}

impl From<QisProgram> for ProgramSource {
    fn from(program: QisProgram) -> Self {
        ProgramSource::Qis(program)
    }
}

impl From<HugrProgram> for ProgramSource {
    fn from(program: HugrProgram) -> Self {
        ProgramSource::Hugr(program)
    }
}

/// Builder for LLVM engines that integrates with the unified simulation API
#[derive(Debug, Clone, Default)]
pub struct QisEngineBuilder {
    /// The source of LLVM IR or HUGR
    source: Option<QisSource>,
    /// Number of qubits (used as both initial allocation and hard limit)
    num_qubits: Option<usize>,
    /// Verbose output
    verbose: bool,
}

impl QisEngineBuilder {
    /// Create a new LLVM engine builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source to LLVM IR text (human-readable format)
    #[must_use]
    pub fn llvm_ir(mut self, ir: impl Into<String>) -> Self {
        self.source = Some(QisSource::LlvmIr(ir.into()));
        self
    }

    /// Set the source to LLVM bitcode (binary format)
    #[must_use]
    pub fn llvm_bitcode(mut self, bitcode: impl Into<Vec<u8>>) -> Self {
        self.source = Some(QisSource::LlvmBitcode(bitcode.into()));
        self
    }

    /// Set the source to LLVM file (auto-detects .ll or .bc extension)
    #[must_use]
    pub fn llvm_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(QisSource::LlvmFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set the source to LLVM IR text file (.ll)
    #[must_use]
    pub fn llvm_ir_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(QisSource::LlvmIrFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set the source to LLVM bitcode file (.bc)
    #[must_use]
    pub fn llvm_bitcode_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(QisSource::LlvmBitcodeFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set the source to HUGR
    #[must_use]
    pub fn hugr(mut self, hugr: Hugr) -> Self {
        self.source = Some(QisSource::Hugr(Box::new(hugr)));
        self
    }

    /// Set the source to HUGR bytes
    #[must_use]
    pub fn hugr_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.source = Some(QisSource::HugrBytes(bytes));
        self
    }

    /// Set the source to HUGR file
    #[must_use]
    pub fn hugr_file(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(QisSource::HugrFile(path.as_ref().to_path_buf()));
        self
    }

    /// Set number of qubits (used as both initial allocation and hard limit)
    #[must_use]
    pub fn qubits(mut self, num_qubits: usize) -> Self {
        self.num_qubits = Some(num_qubits);
        self
    }

    /// Enable verbose output
    #[must_use]
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set the source from a `QisProgram`
    #[must_use]
    pub fn program(mut self, program: impl Into<ProgramSource>) -> Self {
        match program.into() {
            ProgramSource::Qis(p) => match p.content {
                pecos_programs::QisContent::Ir(ir) => {
                    self.source = Some(QisSource::LlvmIr(ir));
                }
                pecos_programs::QisContent::Bitcode(bc) => {
                    self.source = Some(QisSource::LlvmBitcode(bc));
                }
            },
            ProgramSource::Hugr(p) => {
                self.source = Some(QisSource::HugrBytes(p.hugr));
            }
        }
        self
    }
}

impl ClassicalControlEngineBuilder for QisEngineBuilder {
    type Engine = QisEngine;

    fn build(self) -> Result<Self::Engine, PecosError> {
        // Get source or error
        let source = self.source.ok_or_else(|| {
            PecosError::Input(
                "No source specified. Use .llvm_ir(), .llvm_bitcode(), .hugr(), or similar method."
                    .to_string(),
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
        let engine_config = QisEngineConfig {
            assigned_shots: 0,
            verbose: self.verbose,
            max_qubits: self.num_qubits,
        };

        let engine = QisEngine::with_config(path, engine_config);

        Ok(engine)
    }
}

impl From<QisProgram> for QisEngineBuilder {
    fn from(program: QisProgram) -> Self {
        Self::new().program(program)
    }
}

impl From<HugrProgram> for QisEngineBuilder {
    fn from(program: HugrProgram) -> Self {
        Self::new().program(program)
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
/// use pecos_qis_sim::engine_builder::qis_engine;
/// use pecos_engines::{ClassicalControlEngineBuilder, DepolarizingNoise};
///
/// let results = qis_engine()
///     .llvm_ir("define void @main() { ret void }")
///     .to_sim()
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .run(1000)?;
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn qis_engine() -> QisEngineBuilder {
    QisEngineBuilder::new()
}
