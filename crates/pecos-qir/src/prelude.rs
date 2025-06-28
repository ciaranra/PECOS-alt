//! Prelude module for pecos-qir
//!
//! This module provides convenient re-exports of commonly used types and functions
//! for working with LLVM IR execution in PECOS.
//!
//! # Example
//!
//! ```no_run
//! use pecos_qir::prelude::*;
//! use pecos_engines::run_sim;
//!
//! fn main() -> Result<(), PecosError> {
//!     // Create an LLVM engine
//!     let engine = setup_llvm_engine(Path::new("program.ll"), None)?;
//!     
//!     // Run the simulation with 1000 shots
//!     let results = run_sim(engine, 1000, None, None, None, None)?;
//!     
//!     // Work with shot results
//!     println!("Got {} shots", results.len());
//!     for (i, shot) in results.shots.iter().take(5).enumerate() {
//!         println!("Shot {}: {:?}", i, shot);
//!     }
//!     
//!     Ok(())
//! }
//! ```

// Core LLVM functionality
pub use crate::{LlvmEngine, setup_llvm_engine};

// HUGR compilation support (when available)
#[cfg(feature = "hugr-llvm-pipeline")]
pub use crate::{
    HugrCompiler, HugrCompilerConfig, compile_hugr_to_llvm, create_hugr_llvm_engine,
    setup_hugr_llvm_engine,
};

// Common types from pecos-engines for working with results
pub use pecos_engines::{
    BitVecDisplayFormat, ByteMessage, ClassicalEngine, Shot, ShotMap, ShotMapDisplayExt,
    ShotMapDisplayOptions, ShotVec,
};

// Error types
pub use pecos_core::errors::PecosError;

// Common standard library imports for path handling
pub use std::path::{Path, PathBuf};
