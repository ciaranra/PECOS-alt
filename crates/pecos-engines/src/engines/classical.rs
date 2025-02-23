use crate::channels::Message;
use crate::engines::phir::PHIREngine;
use crate::engines::qir::engine::QirClassicalEngine;
use crate::errors::QueueError;
use log::debug;
use pecos_core::types::{CommandBatch, ShotResult};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Classical engine that processes programs and handles measurements
pub trait ClassicalEngine: Send + Sync {
    /// Processes the classical program and generates a batch of quantum commands
    /// to be sent for execution.
    ///
    /// # Returns
    ///
    /// Returns a `CommandBatch` containing the quantum commands to execute if successful.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `QueueError::OperationError`: If the program processing fails or encounters unsupported operations.
    /// - `QueueError::LockError`: If a lock cannot be acquired during the execution process.
    fn process_program(&mut self) -> Result<CommandBatch, QueueError>;
    /// Handles a measurement received from the quantum engine.
    ///
    /// This function takes a `measurement` message and processes it to update
    /// the state or results of the classical engine.
    ///
    /// # Parameters
    ///
    /// - `measurement`: A `Message` containing the measurement data to process.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `QueueError::OperationError`: If the measurement processing fails or encounters
    ///   unsupported operations.
    /// - `QueueError::LockError`: If a lock cannot be acquired during the measurement handling process.
    fn handle_measurement(&mut self, measurement: Message) -> Result<(), QueueError>;
    /// Retrieves the results of the execution process after all measurements are handled.
    ///
    /// # Returns
    ///
    /// Returns a `ShotResult` containing the measurements and results generated
    /// during the execution process.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `QueueError::OperationError`: If result retrieval fails or is unsupported.
    /// - `QueueError::LockError`: If a lock cannot be acquired to access required resources.
    fn get_results(&self) -> Result<ShotResult, QueueError>;
    /// Compiles the classical program into an intermediate representation or directly
    /// into commands that can be executed by the engine.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the compilation is successful, or an `Err` containing
    /// a boxed error if the compilation fails.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `Box<dyn std::error::Error>`: If there is a compilation error due to syntax issues,
    ///   unsupported features, or internal errors in the engine's implementation.
    fn compile(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub fn detect_program_type(path: &Path) -> Result<ProgramType, Box<dyn Error>> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => {
            // Read JSON and verify format
            let content = fs::read_to_string(path)?;
            let json: serde_json::Value = serde_json::from_str(&content)?;

            if let Some("PHIR/JSON") = json.get("format").and_then(|f| f.as_str()) {
                Ok(ProgramType::PHIR)
            } else {
                Err("Invalid JSON format - expected PHIR/JSON".into())
            }
        }
        Some("ll") => Ok(ProgramType::QIR),
        _ => Err("Unsupported file format. Expected .ll or .json".into()),
    }
}

#[allow(clippy::upper_case_acronyms)]
pub enum ProgramType {
    QIR,
    PHIR,
}

pub fn setup_engine(program_path: &Path) -> Result<Box<dyn ClassicalEngine>, Box<dyn Error>> {
    debug!("Program path: {}", program_path.display());
    let build_dir = program_path.parent().unwrap().join("build");
    debug!("Build directory: {}", build_dir.display());
    std::fs::create_dir_all(&build_dir)?;

    match detect_program_type(program_path)? {
        ProgramType::QIR => Ok(Box::new(QirClassicalEngine::new(program_path, &build_dir))),
        ProgramType::PHIR => Ok(Box::new(PHIREngine::new(program_path)?)),
    }
}

pub fn get_program_path(program: &str) -> Result<PathBuf, Box<dyn Error>> {
    debug!("Resolving program path");

    // Get the current directory for relative path resolution
    let current_dir = std::env::current_dir()?;
    debug!("Current directory: {}", current_dir.display());

    // Resolve the path
    let path = if Path::new(program).is_absolute() {
        PathBuf::from(program)
    } else {
        current_dir.join(program)
    };

    // Check if file exists
    if !path.exists() {
        return Err(format!("Program file not found: {}", path.display()).into());
    }

    Ok(path.canonicalize()?)
}
