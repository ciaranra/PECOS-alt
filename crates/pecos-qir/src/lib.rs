pub mod engine;
pub mod library;
pub mod linker; // Links QIR programs with runtime library
pub mod platform;
pub mod prelude;
pub mod runtime;
pub mod qir_compat; // QIR compatibility layer (alternative naming conventions)
pub mod qir_utils; // QIR utilities for entry point detection
pub mod python_api; // Python-friendly API functions

// HUGR-specific functionality
pub mod hugr; // HUGR frontend (compiler, engine, etc.)

// Quantum extension modules for quantum IR→LLVM compilation
#[cfg(feature = "hugr-support")]
pub mod quantum_extension; // Quantum operation extensions

// Internal modules for compilation
pub(crate) mod runtime_builder; // Builds the static runtime library

pub use engine::QirEngine;

// HUGR re-exports (only available with hugr-support feature)
#[cfg(feature = "hugr-support")]
pub use hugr::{Compiler as HugrCompiler, CompilerConfig as HugrCompilerConfig, QuantumNamingConvention};
#[cfg(feature = "hugr-support")]
pub use hugr::{create_hugr_qir_engine, setup_hugr_qir_engine, compile_hugr_to_qir};

// Provide stubs when hugr-support is not enabled
#[cfg(not(feature = "hugr-support"))]
pub use hugr::compiler::{HugrCompiler, HugrCompilerConfig, QuantumNamingConvention};
#[cfg(not(feature = "hugr-support"))]
pub use hugr::engine::{create_hugr_qir_engine, setup_hugr_qir_engine, compile_hugr_to_qir};

use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::path::Path;

/// Sets up a basic QIR engine.
///
/// This function creates a QIR engine from the provided path.
///
/// # Parameters
///
/// - `program_path`: A reference to the path of the QIR program file
/// - `shots`: Optional number of shots to assign to the engine
///
/// # Returns
///
/// Returns a `Box<dyn ClassicalEngine>` containing the QIR engine
///
/// # Errors
///
/// This function may return the following errors:
/// - `PecosError::Compilation`: If the QIR file cannot be compiled
/// - `PecosError::Processing`: If the QIR engine fails to process commands
pub fn setup_qir_engine(
    program_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    debug!("Setting up QIR engine for: {}", program_path.display());

    // Create a QirEngine from the path
    let mut engine = QirEngine::new(program_path.to_path_buf());

    // Set the number of shots assigned to this engine if specified
    if let Some(num_shots) = shots {
        engine.set_assigned_shots(num_shots);
    }

    // Pre-compile the QIR library for efficient cloning
    engine.pre_compile()?;

    Ok(Box::new(engine))
}
