pub mod engine;
pub mod hugr_python_api;
pub mod library;
pub mod linker; // Links LLVM IR programs with runtime library
pub mod llvm_utils; // LLVM utilities for entry point detection
pub mod platform;
pub mod prelude; // Convenient re-exports for common usage
pub mod runtime; // LLVM runtime implementation with submodules
pub mod utils; // Common utilities for error handling, logging, etc.

// HUGR-LLVM pipeline functionality
pub mod hugr; // HUGR frontend (compiler, engine, etc.) - contains stubs when feature disabled

// PMIR (PECOS MLIR) - Alternative compilation pipeline via MLIR
// Using external pecos-pmir crate

pub use engine::LlvmEngine;

// HUGR-LLVM pipeline re-exports
pub use hugr::compiler::{HugrCompiler, HugrCompilerConfig};
pub use hugr::engine_utils::{
    compile_hugr_to_llvm, create_hugr_llvm_engine, setup_hugr_llvm_engine,
};

// PMIR pipeline re-exports (only available with pmir-pipeline feature)
// Users should depend on pecos-pmir directly if they need PMIR functionality
#[cfg(feature = "pmir-pipeline")]
pub use pecos_pmir::{
    PmirConfig, compile_hugr_via_pmir, hugr_to_past_ron, hugr_to_pmir_mlir, past_ron_to_pmir_mlir,
};

use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::path::Path;

/// Sets up a basic LLVM engine.
///
/// This function creates an LLVM engine from the provided path.
///
/// # Parameters
///
/// - `program_path`: A reference to the path of the LLVM IR program file
/// - `shots`: Optional number of shots to assign to the engine
///
/// # Returns
///
/// Returns a `Box<dyn ClassicalEngine>` containing the LLVM engine
///
/// # Errors
///
/// This function may return the following errors:
/// - `PecosError::Compilation`: If the LLVM IR file cannot be compiled
/// - `PecosError::Processing`: If the LLVM engine fails to process commands
pub fn setup_llvm_engine(
    program_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    debug!("Setting up LLVM engine for: {}", program_path.display());

    // Create an LlvmEngine from the path
    let mut engine = LlvmEngine::new(program_path.to_path_buf());

    // Set the number of shots assigned to this engine if specified
    if let Some(num_shots) = shots {
        engine.set_assigned_shots(num_shots);
    }

    // Pre-compile the LLVM library for efficient cloning
    engine.pre_compile()?;

    Ok(Box::new(engine))
}
