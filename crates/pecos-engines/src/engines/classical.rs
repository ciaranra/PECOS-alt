use crate::byte_message::ByteMessage;
use crate::engines::{ControlEngine, Engine, EngineStage, phir, qir};
use crate::errors::QueueError;
use crate::shot_results::ShotResult;
use dyn_clone::DynClone;
use log::debug;
use std::any::Any;
use std::error::Error;
use std::path::{Path, PathBuf};

/// Classical engine that processes programs and handles measurements
pub trait ClassicalEngine:
    Engine<Input = (), Output = ShotResult> + DynClone + Send + Sync
{
    fn num_qubits(&self) -> usize;

    /// Generate a `ByteMessage` containing the next batch of quantum commands to execute
    ///
    /// # Returns
    ///
    /// Returns a `ByteMessage` containing the quantum commands to execute if successful.
    /// An empty message indicates no more commands are available.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `QueueError::OperationError`: If the program processing fails or encounters unsupported operations.
    /// - `QueueError::LockError`: If a lock cannot be acquired during the execution process.
    fn generate_commands(&mut self) -> Result<ByteMessage, QueueError>;

    /// Handles a `ByteMessage` containing measurements from the quantum engine
    ///
    /// # Parameters
    ///
    /// - `message`: A `ByteMessage` containing the measurement data to process.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `QueueError::OperationError`: If the measurement processing fails.
    /// - `QueueError::LockError`: If a lock cannot be acquired during the measurement handling process.
    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), QueueError>;

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

    /// Sets a specific seed for the classical engine
    ///
    /// # Arguments
    /// * `seed` - Seed value for the random number generator
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns a `QueueError` if setting the seed fails
    fn set_seed(&mut self, _seed: u64) -> Result<(), QueueError> {
        // Default implementation just succeeds without doing anything
        Ok(())
    }

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

    /// Resets the state of the classical engine to its initial configuration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the reset operation completes successfully.
    ///
    /// # Errors
    ///
    /// This function may return the following errors:
    /// - `QueueError::OperationError`: If the reset operation encounters unsupported actions or fails.
    /// - `QueueError::LockError`: If a lock cannot be acquired during the reset process.
    fn reset(&mut self) -> Result<(), QueueError> {
        Ok(())
    }

    /// Returns a reference to self as Any
    ///
    /// This allows for type-checking and downcasting without requiring
    /// experimental trait upcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns a mutable reference to self as Any
    ///
    /// This allows for type-checking and downcasting without requiring
    /// experimental trait upcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Register the ClassicalEngine trait with dyn_clone
dyn_clone::clone_trait_object!(ClassicalEngine);

impl ControlEngine for Box<dyn ClassicalEngine> {
    type Input = ();
    type Output = ShotResult;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
        // Build up first batch of commands until measurement needed
        let commands = self.generate_commands()?;

        // Check if we have an empty message (no more commands)
        if let Ok(is_empty) = commands.is_empty() {
            if is_empty {
                // No more commands, return results
                let results = self.get_results()?;
                return Ok(EngineStage::Complete(results));
            }
        }

        // Need to process these commands
        Ok(EngineStage::NeedsProcessing(commands))
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
        // Handle measurements from quantum engine
        self.handle_measurements(measurements)?;

        // Generate next batch of commands
        let commands = self.generate_commands()?;

        // Check if we have an empty message (no more commands)
        if let Ok(is_empty) = commands.is_empty() {
            if is_empty {
                // No more commands, return results
                let results = self.get_results()?;
                return Ok(EngineStage::Complete(results));
            }
        }

        Ok(EngineStage::NeedsProcessing(commands))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // Use fully qualified path to disambiguate
        ClassicalEngine::reset(&mut **self)
    }
}

impl Engine for Box<dyn ClassicalEngine> {
    type Input = ();
    type Output = ShotResult;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        let mut stage = self.start(input)?;

        loop {
            match stage {
                EngineStage::NeedsProcessing(_engine_input) => {
                    // In a real system, this would process through a quantum engine
                    // For now, we'll just return an empty message
                    let engine_output = ByteMessage::builder().build();
                    stage = self.continue_processing(engine_output)?;
                }
                EngineStage::Complete(output) => return Ok(output),
            }
        }
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // Use fully qualified path to disambiguate
        ClassicalEngine::reset(&mut **self)
    }
}

/// Detects the type of program based on its file extension and content.
///
/// This function examines the file extension and content to determine if the file
/// corresponds to a QIR or PHIR program type.
///
/// # Parameters
///
/// - `path`: A reference to the path of the file to be analyzed.
///
/// # Returns
///
/// Returns a `ProgramType` indicating the detected type if successful, or a boxed error
/// if format detection fails.
///
/// # Errors
///
/// This function may return the following errors:
/// - `std::io::Error`: If the file cannot be opened or read.
/// - `serde_json::Error`: If the JSON content cannot be parsed when detecting a PHIR program.
/// - `Box<dyn std::error::Error>`: If the file does not conform to a supported format
///   (e.g., invalid JSON format for PHIR or unsupported file extension).
pub fn detect_program_type(path: &Path) -> Result<ProgramType, Box<dyn Error>> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => {
            // Read JSON and verify format
            let content = std::fs::read_to_string(path)?;
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

/// Sets up a classical engine based on the type of the provided program file.
///
/// This function detects the type of the program (e.g., QIR or PHIR), creates the necessary
/// build directory, and instantiates the corresponding classical engine.
///
/// # Parameters
///
/// - `program_path`: A reference to the path of the program file to be processed.
/// - `shots`: Optional number of shots to set for the engine. Only used for QIR engines.
///
/// # Returns
///
/// Returns a `Box<dyn ClassicalEngine>` containing the constructed engine if successful,
/// or a boxed error if setup fails.
///
/// # Errors
///
/// This function may return the following errors:
/// - `std::io::Error`: If the build directory cannot be created.
/// - `Box<dyn std::error::Error>`: If the program type cannot be detected, or if there
///   is an error while initializing the engine (e.g., invalid file format or unsupported version).
///
/// # Panics
///
/// This function will panic if the `program_path` does not have a parent directory, as it
/// assumes the existence of a parent directory for creating the build directory.
pub fn setup_engine(
    program_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, Box<dyn Error>> {
    debug!("Program path: {}", program_path.display());
    let build_dir = program_path.parent().unwrap().join("build");
    debug!("Build directory: {}", build_dir.display());
    std::fs::create_dir_all(&build_dir)?;

    match detect_program_type(program_path)? {
        ProgramType::QIR => {
            debug!("Setting up QIR engine and pre-compiling for efficient cloning");
            let mut engine = qir::QirEngine::new(program_path.to_path_buf());

            // Set the number of shots assigned to this engine if specified
            if let Some(num_shots) = shots {
                engine.set_assigned_shots(num_shots)?;
            }

            // Pre-compile the QIR library to prepare for efficient cloning
            engine.pre_compile()?;

            Ok(Box::new(engine))
        }
        ProgramType::PHIR => Ok(Box::new(phir::PHIREngine::new(program_path)?)),
    }
}

/// Resolves the absolute path of the provided program.
///
/// This function takes a program path (either absolute or relative),
/// resolves it to an absolute path, and checks if the file exists.
///
/// # Parameters
///
/// - `program`: A string slice containing the path to the program file.
///
/// # Returns
///
/// Returns a `PathBuf` containing the canonicalized absolute path if successful,
/// or an error if the file cannot be found or resolved.
///
/// # Errors
///
/// This function can return the following errors:
/// - `std::io::Error`: If the current working directory cannot be obtained.
/// - `Box<dyn std::error::Error>`: If the program file does not exist, or if the
///   canonicalization of the file path fails.
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
