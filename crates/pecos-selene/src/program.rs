//! Selene program format types
//!
//! This module defines the program formats that Selene can accept.

use std::path::PathBuf;

/// Program formats that Selene can accept in pure Rust
/// 
/// Note: Guppy programs require Python compilation (Guppy → HUGR) which
/// should be handled in Python via guppy_selene_sim(), not here.
#[derive(Debug, Clone)]
pub enum SeleneProgram {
    /// HUGR 0.13 (for guppylang compatibility)
    #[cfg(feature = "hugr-013")]
    Hugr(crate::hugr_013_support::Hugr),
    /// HUGR serialized bytes (to be deserialized during build)
    #[cfg(feature = "hugr-013")]
    HugrBytes(Vec<u8>),
    /// LLVM IR text format
    LlvmIr(String),
    /// LLVM bitcode binary format
    LlvmBitcode(Vec<u8>),
    /// LLVM file (auto-detect .ll or .bc)
    LlvmFile(PathBuf),
    /// LLVM IR text file (.ll)
    LlvmIrFile(PathBuf),
    /// LLVM bitcode file (.bc)
    LlvmBitcodeFile(PathBuf),
    /// HUGR file
    HugrFile(PathBuf),
    /// Compiled plugin file (.so)
    Plugin(PathBuf),
}