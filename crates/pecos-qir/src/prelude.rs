//! Prelude module for pecos-qir
//!
//! This module provides convenient re-exports of commonly used types and functions
//! for working with QIR (Quantum Intermediate Representation) in PECOS.
//!
//! # Example
//!
//! ```no_run
//! use pecos_qir::prelude::*;
//! use pecos_engines::run_sim;
//!
//! fn main() -> Result<(), PecosError> {
//!     // Create a QIR engine
//!     let engine = setup_qir_engine(Path::new("program.qir"), None)?;
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

// Core QIR functionality
pub use crate::{QirEngine, setup_qir_engine};

// HUGR compilation support (when available)
#[cfg(feature = "hugr-llvm-pipeline")]
pub use crate::{
    HugrCompiler, HugrCompilerConfig, QuantumLlvmConvention, compile_hugr_to_qir,
    create_hugr_qir_engine, setup_hugr_qir_engine,
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
