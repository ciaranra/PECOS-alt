pub mod engine;
pub mod library;
pub mod linker; // Links QIR programs with runtime library
pub mod platform;
pub mod prelude;
pub mod runtime;

// Internal modules for compilation
pub(crate) mod runtime_builder; // Builds the static runtime library

pub use engine::QirEngine;

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
