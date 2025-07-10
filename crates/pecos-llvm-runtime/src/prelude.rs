//! Prelude module for pecos-llvm-runtime
//!
//! This module provides convenient re-exports of commonly used types and functions
//! for working with LLVM IR execution in PECOS.
//!
//! # Example
//!
//! ```no_run
//! use pecos_llvm_runtime::prelude::*;
//! use pecos_engines::run_sim;
//!
//! fn main() -> Result<(), PecosError> {
//!     // Create an LLVM engine
//!     let engine = LlvmEngine::new(PathBuf::from("program.ll"));
//!     
//!     // Option 1: Run simulation with run_sim (general purpose)
//!     let results = run_sim(
//!         Box::new(engine.clone()),
//!         1000,  // shots
//!         Some(42),  // seed
//!         None,  // workers (defaults to 1)
//!         None,  // noise model (defaults to no noise)
//!         None,  // quantum engine (defaults to StateVecEngine)
//!     )?;
//!     
//!     // Work with shot results
//!     println!("Got {} shots", results.len());
//!     for (i, shot) in results.shots.iter().take(5).enumerate() {
//!         println!("Shot {}: {:?}", i, shot);
//!     }
//!     
//!     // Option 2: Run a single shot directly
//!     let mut engine_single = engine;
//!     let shot = engine_single.process(())?;
//!     println!("Single shot result: {:?}", shot);
//!     
//!     // Note: For more advanced LLVM simulation features (e.g., compiling from HUGR,
//!     // managing temporary files, etc.), consider using the `pecos-llvm-sim` crate
//!     // which provides a builder pattern through `LlvmSim`.
//!     
//!     Ok(())
//! }
//! ```

// Core LLVM functionality
pub use crate::LlvmEngine;

// Common types from pecos-engines for working with results
pub use pecos_engines::{
    BitVecDisplayFormat, ByteMessage, ClassicalEngine, Engine, MonteCarloEngine, 
    Shot, ShotMap, ShotMapDisplayExt, ShotMapDisplayOptions, ShotVec,
};

// Error types
pub use pecos_core::errors::PecosError;

// Common standard library imports for path handling
pub use std::path::{Path, PathBuf};
