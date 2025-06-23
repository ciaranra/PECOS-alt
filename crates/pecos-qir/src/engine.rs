//! QIR Engine Module
//!
//! This module provides the QIR Engine for executing quantum programs compiled to QIR.
use crate::library::QirLibrary;
use crate::linker::QirLinker;
use log::{debug, trace, warn};
use pecos_core::errors::PecosError;
use pecos_engines::Engine;
use pecos_engines::byte_message::ByteMessage;
use pecos_engines::engine_system::{ClassicalEngine, ControlEngine, EngineStage};
use pecos_engines::shot_results::{Data, Shot};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// Program complexity analysis for compilation strategy decisions
#[derive(Debug, Clone)]
struct ProgramComplexity {
    is_simple: bool,
    has_control_flow: bool,
    has_classical_compute: bool,
    gate_count: usize,
    qubit_count: usize,
}

/// Helper function to get the current thread ID as a string
///
/// This function returns the current thread ID formatted as a string.
/// It's used for logging and debugging purposes.
///
/// # Returns
///
/// A string representation of the current thread ID
#[must_use]
pub fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Configuration options for the QIR engine
#[derive(Debug, Clone, Default)]
pub struct QirEngineConfig {
    /// Number of shots assigned to this engine
    pub assigned_shots: usize,
    /// Whether to show verbose command logs
    pub verbose: bool,
}

/// QIR Engine for executing quantum programs compiled to QIR
///
/// The engine loads and executes QIR programs, handling the interaction between
/// the QIR runtime and the quantum system.
pub struct QirEngine {
    /// The loaded QIR library for executing quantum programs
    library: Option<Box<QirLibrary>>,

    /// Map of measurement results by `result_id`
    measurement_results: HashMap<usize, i64>,

    /// Path to the QIR file to execute
    qir_file: PathBuf,

    /// Path to the compiled library file
    library_path: Option<PathBuf>,

    /// Flag indicating whether commands have been generated for the current shot
    commands_generated: bool,

    /// Number of shots processed so far
    shot_count: usize,

    /// Configuration options for the engine
    config: QirEngineConfig,

    /// Entry point function name (detected from QIR file)
    entry_point: Option<String>,
}

impl QirEngine {
    /// Helper function to log errors
    fn log_error<E: std::fmt::Display>(context: &str, error: E) -> PecosError {
        warn!("QIR Engine: {}: {}", context, error);
        PecosError::Processing(format!("QIR operation failed - {context}: {error}"))
    }

    /// Create a new QIR engine with default configuration
    ///
    /// # Arguments
    ///
    /// * `qir_file` - Path to the QIR file to execute
    ///
    /// # Returns
    ///
    /// A new QIR engine instance with default configuration
    #[must_use]
    pub fn new(qir_file: PathBuf) -> Self {
        debug!("QIR: Creating new engine with program path: {:?}", qir_file);
        Self {
            library: None,
            measurement_results: HashMap::new(),
            qir_file,
            library_path: None,
            commands_generated: false,
            shot_count: 0,
            config: QirEngineConfig::default(),
            entry_point: None,
        }
    }

    /// Create a new QIR engine with custom configuration
    ///
    /// # Arguments
    ///
    /// * `qir_file` - Path to the QIR file to execute
    /// * `config` - Configuration options for the engine
    ///
    /// # Returns
    ///
    /// A new QIR engine instance with the specified configuration
    #[must_use]
    pub fn with_config(qir_file: PathBuf, config: QirEngineConfig) -> Self {
        debug!(
            "QIR: Creating new engine with program path: {:?} and custom config",
            qir_file
        );
        Self {
            library: None,
            measurement_results: HashMap::new(),
            qir_file,
            library_path: None,
            commands_generated: false,
            shot_count: 0,
            config,
            entry_point: None,
        }
    }

    /// Set the number of shots assigned to this engine
    pub fn set_assigned_shots(&mut self, shots: usize) {
        debug!("QIR: Setting assigned shots to {}", shots);
        self.config.assigned_shots = shots;
    }

    /// Set whether to show verbose command logs
    pub fn set_verbose(&mut self, verbose: bool) {
        self.config.verbose = verbose;
    }

    /// Reset the internal state of the engine
    fn reset_internal_state(&mut self) {
        debug!("QIR: Resetting internal state");
        self.shot_count = 0;
        self.measurement_results.clear();
        self.commands_generated = false;

        if let Some(ref library) = self.library {
            if let Err(e) = library.reset() {
                debug!("QIR: Failed to reset QIR runtime: {}", e);
            }
        }
    }

    /// Set up the QIR library
    fn setup_library(&mut self) -> Result<(), PecosError> {
        // If the library is already set up, don't recompile
        if self.library.is_some() {
            trace!("QIR: Library already set up, skipping compilation");
            return Ok(());
        }

        debug!("QIR: Setting up library");

        // Clean up any existing library
        self.reset_internal_state();

        // Create a unique temporary directory for this thread with more randomness
        let thread_id = get_thread_id();
        // Add timestamp for additional uniqueness across multiple test runs
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        // Use timestamp as a unique identifier - no external dependencies needed
        let temp_dir = std::env::temp_dir().join(format!(
            "qir_{}_{}_{}",
            std::process::id(),
            thread_id,
            timestamp
        ));

        debug!("QIR: Creating unique temporary directory at {:?}", temp_dir);

        // Ensure the directory is clean by removing it if it exists
        if temp_dir.exists() {
            debug!("QIR: Temporary directory already exists, removing it first");
            std::fs::remove_dir_all(&temp_dir)
                .map_err(|e| Self::log_error("Failed to clean existing temp directory", e))?;
        }

        // Create the directory
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| Self::log_error("Failed to create temp directory", e))?;

        // Check if we already have a library path from a previous compilation
        let library_path = if let Some(ref library_path) = self.library_path {
            debug!(
                "QIR: Using existing library at {:?} as template",
                library_path
            );

            // Create a thread-specific copy of the library with platform-specific extension
            let extension = if cfg!(target_os = "windows") {
                "dll"
            } else if cfg!(target_os = "macos") {
                "dylib"
            } else {
                "so"
            };

            let thread_specific_path = temp_dir.join(format!("lib_thread_{thread_id}.{extension}"));

            debug!(
                "QIR: Thread-specific library path: {:?}",
                thread_specific_path
            );

            // Copy the library to the thread-specific path with verification
            if library_path.exists() {
                // Verify source file is valid before copying
                let metadata = std::fs::metadata(library_path)
                    .map_err(|e| Self::log_error("Failed to get metadata for source library", e))?;

                if !metadata.is_file() {
                    return Err(Self::log_error(
                        "Source library is not a regular file",
                        format!("Path: {}", library_path.display()),
                    ));
                }

                let file_size = metadata.len();
                if file_size < 1024 {
                    return Err(Self::log_error(
                        "Source library file is too small to be valid",
                        format!(
                            "Path: {} (size: {} bytes)",
                            library_path.display(),
                            file_size
                        ),
                    ));
                }

                // Copy the file
                debug!(
                    "QIR: Copying library from {:?} to {:?}",
                    library_path, thread_specific_path
                );
                std::fs::copy(library_path, &thread_specific_path).map_err(|e| {
                    Self::log_error("Failed to copy library to thread-specific path", e)
                })?;

                // Verify the copied file
                let copied_metadata = std::fs::metadata(&thread_specific_path)
                    .map_err(|e| Self::log_error("Failed to get metadata for copied library", e))?;

                let copied_size = copied_metadata.len();
                if copied_size != file_size {
                    return Err(Self::log_error(
                        "Copied library file size mismatch",
                        format!("Expected: {file_size} bytes, Got: {copied_size} bytes"),
                    ));
                }

                debug!("QIR: Successfully copied library ({} bytes)", copied_size);
                thread_specific_path
            } else {
                // If the library doesn't exist, compile it
                debug!("QIR: Library template doesn't exist, compiling from source");
                self.compile_library(&temp_dir)?
            }
        } else {
            // If we don't have a library path, compile the QIR file
            debug!("QIR: No existing library, compiling from source");
            self.compile_library(&temp_dir)?
        };

        // Load the library
        debug!("QIR: Loading library from {:?}", library_path);

        let library = QirLibrary::load(&library_path)
            .map_err(|e| Self::log_error("Failed to load QIR library", e))?;

        // Store the library and path
        self.library = Some(Box::new(library));
        self.library_path = Some(library_path.clone());

        // Try to detect the entry point from the QIR file
        if self.entry_point.is_none() {
            match crate::qir_utils::find_entry_point(&self.qir_file) {
                Ok(Some(entry_point)) => {
                    debug!("QIR: Detected entry point function: {}", entry_point);
                    self.entry_point = Some(entry_point);
                }
                Ok(None) => {
                    // No entry point found - log warning but don't fail yet
                    // The error will be caught in run_qir_program
                    debug!("QIR: No entry point detected from LLVM IR attributes");
                }
                Err(e) => {
                    // Failed to detect entry point - log warning but don't fail yet
                    debug!("QIR: Failed to detect entry point: {}", e);
                }
            }
        }

        debug!("QIR: Successfully set up QIR library");

        Ok(())
    }

    /// Process measurements from the quantum system
    fn process_measurements(&mut self, message: &ByteMessage) -> Result<(), PecosError> {
        // Extract raw measurement outcomes
        let outcomes = message.outcomes().map_err(|e| {
            PecosError::Input(format!(
                "Failed to extract measurements from ByteMessage: {e}"
            ))
        })?;

        // Convert to indexed format for compatibility with existing code
        let measurements: Vec<(usize, u32)> = outcomes.into_iter().enumerate().collect();

        self.measurement_results.clear();
        // Convert u32 measurements to i64 for QIR standard
        self.measurement_results.extend(
            measurements
                .iter()
                .map(|(id, value)| (*id, i64::from(*value))),
        );

        // Update the runtime with measurement results
        if let Some(library) = &self.library {
            debug!(
                "QIR: Updating runtime with {} measurement results",
                measurements.len()
            );

            // Convert measurements to the format expected by the runtime
            // The runtime expects pairs of (result_id, value)
            let mut results_data = Vec::with_capacity(measurements.len() * 2);
            for (result_id, value) in measurements {
                debug!("QIR: Measurement result_id={} value={}", result_id, value);
                results_data.push(u32::try_from(result_id).map_err(|_| {
                    PecosError::Resource(format!(
                        "Result ID {result_id} is too large to fit in u32"
                    ))
                })?);
                results_data.push(value);
            }

            // Call the runtime update function
            library.update_measurement_results(&results_data)?;

            // Now finalize the shot with the measurement results
            library.finalize_shot()?;
        }

        self.commands_generated = false;
        self.shot_count += 1;

        debug!("QIR: Completed shot {}", self.shot_count);
        Ok(())
    }

    /// Get the results of the quantum computation
    ///
    /// # Returns
    ///
    /// * `Shot` - The results of the quantum computation
    fn get_results_impl(&self) -> Shot {
        // Try to get shot results from the runtime
        if let Some(library) = &self.library {
            if let Ok(Some(shot)) = library.get_shot_results() {
                debug!(
                    "QIR: Retrieved shot from runtime with {} registers",
                    shot.data.len()
                );
                return shot;
            }
        }

        // Fallback: create shot result from raw measurements
        // This should only happen if the runtime doesn't support shot export
        debug!("QIR: Falling back to raw measurement results");
        let mut shot_result = Shot::default();

        for (&result_id, &value) in &self.measurement_results {
            let name = format!("result_{result_id}");
            // Store all values as I64 for consistency with QIR standard
            shot_result.data.insert(name, Data::I64(value));
        }

        shot_result
    }

    /// Pre-compile the QIR library to prepare for cloning
    ///
    /// # Errors
    ///
    /// Returns an error if the QIR library cannot be pre-compiled.
    pub fn pre_compile(&mut self) -> Result<(), PecosError> {
        // Get the current thread ID for logging
        let thread_id = get_thread_id();

        debug!(
            "QIR: [Thread {}] Pre-compiling library for efficient cloning",
            thread_id
        );

        // If the library is already set up, don't recompile
        if self.library.is_some() && self.library_path.is_some() {
            debug!(
                "QIR: [Thread {}] Library already pre-compiled, skipping",
                thread_id
            );
            return Ok(());
        }

        // Compile the QIR program to a library
        let library_path = QirLinker::compile(&self.qir_file, None)
            .map_err(|e| PecosError::Processing(format!("Failed to compile QIR program: {e}")))?;

        // Detect the entry point from the QIR file
        match crate::qir_utils::find_entry_point(&self.qir_file) {
            Ok(Some(entry_point)) => {
                debug!("QIR: Detected entry point function: {}", entry_point);
                self.entry_point = Some(entry_point);
            }
            Ok(None) => {
                // No entry point found - log but don't fail during pre-compile
                // The actual error will be thrown when trying to run the program
                debug!("QIR: No entry point found in QIR file during pre-compile");
                self.entry_point = None;
            }
            Err(e) => {
                // Failed to parse QIR file - log but don't fail during pre-compile
                debug!("QIR: Failed to detect entry point during pre-compile: {}", e);
                self.entry_point = None;
            }
        }

        // Store the library path
        self.library_path = Some(library_path.clone());

        // We don't need to load the library here, as each thread will get its own copy
        debug!(
            "QIR: [Thread {}] Library pre-compiled successfully (path: {:?})",
            thread_id, library_path
        );

        Ok(())
    }

    /// Run the QIR program and get the commands
    ///
    /// This method runs the QIR program by calling the main function in the library
    /// and retrieves the generated quantum commands.
    ///
    /// # Arguments
    ///
    /// * `library` - The QIR library to run
    ///
    /// # Returns
    ///
    /// * `Result<ByteMessage, PecosError>` - The binary message generated by the QIR program
    ///
    /// # Error Handling
    ///
    /// Errors are propagated through the Result type and logged at their source with
    /// appropriate context, including the thread ID.
    fn run_qir_program(&self, library: &QirLibrary) -> Result<ByteMessage, PecosError> {
        
        // Configure verbosity through environment variable
        if self.config.verbose {
            unsafe {
                std::env::remove_var("QIR_RUNTIME_QUIET");
            }
        } else {
            unsafe {
                std::env::set_var("QIR_RUNTIME_QUIET", "1");
            }
        }

        // Find and call the entry point function
        // First check if we already know the entry point for this program
        let entry_point = if let Some(ref ep) = self.entry_point {
            ep.clone()
        } else {
            // No entry point was detected - this is an error
            return Err(PecosError::Input(
                "No entry point found in QIR program. The program must have a function \
                 marked with the 'EntryPoint' attribute. Example:\n\
                 define void @my_function() #0 {\n\
                   ...\n\
                 }\n\
                 attributes #0 = { \"EntryPoint\" }".to_string()
            ));
        };

        // Check if the entry point function exists in the library
        if !library.has_function(entry_point.as_bytes()).unwrap_or(false) {
            return Err(PecosError::Input(format!(
                "Entry point function '{}' was marked with EntryPoint attribute but was not found \
                 in the compiled library. This may indicate a compilation error or that the function \
                 was optimized away. Ensure the function has a body and is not marked as internal.",
                entry_point
            )));
        }

        debug!("QIR: Calling entry point function: {}", entry_point);
        library.call_function(entry_point.as_bytes()).map_err(|e| {
            // Special case for removed library files
            if e.to_string().contains("No such file or directory") {
                debug!("QIR: Library file was already removed, continuing");
                PecosError::Processing("Library file was already removed".to_string())
            } else {
                Self::log_error(&format!("Failed to call {entry_point} function"), e)
            }
        })?;

        // Get the binary message generated by the QIR runtime
        let runtime_message = library
            .get_binary_commands()
            .map_err(|e| Self::log_error("Failed to get binary commands from QIR runtime", e))?;

        // Log message details for debugging
        debug!(
            "QIR: Binary message from runtime: {} bytes",
            runtime_message.as_bytes().len()
        );

        // Try to parse and log quantum operations for debugging
        if let Ok(operations) = runtime_message.quantum_ops() {
            debug!("QIR: Parsed {} quantum operations:", operations.len());
            for (i, op) in operations.iter().enumerate().take(10) {
                debug!("QIR:   [{}] {:?}", i, op);
            }
            if operations.len() > 10 {
                debug!("QIR:   ... and {} more operations", operations.len() - 10);
            }
        }

        Ok(runtime_message)
    }

    fn generate_commands_impl(&mut self) -> Result<Option<ByteMessage>, PecosError> {
        // Only log at trace level to reduce verbosity
        trace!("QIR: Generating commands (shot {})", self.shot_count + 1);

        // If we've already generated commands for this shot, return None
        if self.commands_generated {
            trace!("QIR: Commands already generated for this shot, returning None");
            return Ok(None);
        }

        // If we've already processed a shot in this run_shot call, return None
        if self.shot_count > 0 {
            debug!("QIR: Already processed one shot in this run_shot call, returning None");
            return Ok(None);
        }

        // Set up library if not already done
        if self.library.is_none() {
            debug!(
                "QIR: Setting up library before generating commands for shot {}",
                self.shot_count + 1
            );

            // Try to set up the library, handling "Text file busy" error with a retry
            if let Err(e) = self.setup_library() {
                if e.to_string().contains("Text file busy") {
                    debug!("QIR: Got 'Text file busy' error, trying to recover");
                    // Sleep a bit longer to allow the file to be released
                    thread::sleep(Duration::from_millis(500));
                    // Try to set up the library again
                    self.setup_library().map_err(|e| {
                        warn!("QIR: Failed to set up library after retry: {}", e);
                        e
                    })?;
                } else {
                    warn!("QIR: Failed to set up library: {}", e);
                    return Err(e);
                }
            }
        }

        // Run the QIR program
        if let Some(library) = &self.library {
            // Run the QIR program and get the ByteMessage directly
            let runtime_message = self.run_qir_program(library)?;

            debug!(
                "QIR: Got ByteMessage for shot {} with {} bytes",
                self.shot_count + 1,
                runtime_message.as_bytes().len()
            );

            // Mark that we've generated commands for this shot
            self.commands_generated = true;

            // Return the ByteMessage
            Ok(Some(runtime_message))
        } else {
            warn!("QIR: No QIR library loaded");
            Err(PecosError::Processing(
                "Cannot generate quantum commands: No QIR library loaded. Call compile() or setup_library() first.".to_string(),
            ))
        }
    }

    /// Helper method to find qubit allocations in QIR content using regex patterns
    fn find_qubit_allocations(content: &str) -> (usize, bool) {
        let mut max_qubit_index = 0;
        let mut found_allocation = false;

        // Pattern 1: Direct qubit references like "inttoptr (i64 N to %Qubit*)"
        // These patterns are static and validated at development time, so we use expect()
        // instead of unwrap() to provide more context in case of a programming error
        let direct_pattern = Regex::new(r"inttoptr\s*\(\s*i64\s+(\d+)\s+to\s+%Qubit\*\)")
            .expect("Invalid regex pattern for direct qubit references");
        for cap in direct_pattern.captures_iter(content) {
            if let Some(index_match) = cap.get(1) {
                if let Ok(index) = index_match.as_str().parse::<usize>() {
                    max_qubit_index = max_qubit_index.max(index);
                    found_allocation = true;
                }
            }
        }

        // Pattern 2: Qubit allocations like "__quantum__rt__qubit_allocate()"
        let alloc_pattern = Regex::new(r"__quantum__rt__qubit_allocate\(\)")
            .expect("Invalid regex pattern for qubit allocations");
        let alloc_count = alloc_pattern.find_iter(content).count();
        if alloc_count > 0 {
            max_qubit_index = max_qubit_index.max(alloc_count - 1);
            found_allocation = true;
        }

        // Pattern 3: Array allocations like "__quantum__rt__array_create_1d(i64 8, i64 N)"
        let array_pattern =
            Regex::new(r"__quantum__rt__array_create_1d\s*\(\s*i64\s+\d+\s*,\s*i64\s+(\d+)\s*\)")
                .expect("Invalid regex pattern for array allocations");
        for cap in array_pattern.captures_iter(content) {
            if let Some(size_match) = cap.get(1) {
                if let Ok(size) = size_match.as_str().parse::<usize>() {
                    max_qubit_index = max_qubit_index.max(size - 1);
                    found_allocation = true;
                }
            }
        }

        // Pattern 4: Integer-based qubit references in PECOS QIR calls like "i64 N"
        // This pattern looks for quantum gate calls with integer qubit arguments
        // Handles both __body and __body_i64 variants
        let int_qubit_pattern =
            Regex::new(r"__quantum__qis__[a-z_]+__body[a-z0-9_]*\s*\([^)]*?i64\s+(\d+)")
                .expect("Invalid regex pattern for integer qubit references");
        for cap in int_qubit_pattern.captures_iter(content) {
            if let Some(index_match) = cap.get(1) {
                if let Ok(index) = index_match.as_str().parse::<usize>() {
                    max_qubit_index = max_qubit_index.max(index);
                    found_allocation = true;
                }
            }
        }

        // Pattern 5: Pointer-based qubit references like "call void @__quantum__qis__h__body(%Qubit* null)"
        // This pattern looks for standard QIR calls with %Qubit* arguments
        let ptr_qubit_pattern =
            Regex::new(r"__quantum__qis__[a-z_]+__body\s*\([^)]*%Qubit\*[^)]*\)")
                .expect("Invalid regex pattern for pointer qubit references");
        if ptr_qubit_pattern.is_match(content) {
            // For pointer-based QIR, we need to count the highest qubit index from the pointers
            // Look for patterns like "inttoptr (i64 N to %Qubit*)" which we already handle in Pattern 1
            // Also look for null pointers which represent qubit 0
            let null_qubit_pattern = Regex::new(r"%Qubit\*\s+null")
                .expect("Invalid regex pattern for null qubit references");
            if null_qubit_pattern.is_match(content) {
                max_qubit_index = max_qubit_index.max(0);
                found_allocation = true;
            }
        }

        (max_qubit_index, found_allocation)
    }

    fn analyze_qir_file(&self) -> Result<usize, PecosError> {
        debug!("QIR Engine: Analyzing QIR file: {:?}", self.qir_file);

        // Check if the file exists
        if !self.qir_file.exists() {
            return Err(PecosError::Resource(format!(
                "Unable to analyze QIR file: File not found at path '{}'",
                self.qir_file.display()
            )));
        }

        // Read the file content - using IO error directly
        let content = fs::read_to_string(&self.qir_file)?;

        // Check if the file is empty
        if content.is_empty() {
            return Err(PecosError::Resource(format!(
                "Unable to analyze QIR file: File is empty at path '{}'",
                self.qir_file.display()
            )));
        }

        // Analyze program complexity for future compilation strategy decisions
        let complexity = self.analyze_program_complexity(&content);
        debug!("QIR Engine: Program complexity - simple: {}, has_control_flow: {}", 
               complexity.is_simple, complexity.has_control_flow);

        // Find qubit allocations in the QIR file
        let (max_qubit_index, found_allocation) = Self::find_qubit_allocations(&content);

        if found_allocation {
            // The number of qubits is the maximum index + 1
            let num_qubits = max_qubit_index + 1;
            debug!("QIR Engine: Found {} qubits in QIR file", num_qubits);
            Ok(num_qubits)
        } else {
            Err(PecosError::Input(format!(
                "Invalid QIR program: No qubit allocations found in file '{}'. The program must contain at least one qubit allocation.",
                self.qir_file.display()
            )))
        }
    }

    /// Analyze program complexity to determine optimal compilation strategy
    fn analyze_program_complexity(&self, content: &str) -> ProgramComplexity {
        let has_control_flow = content.contains("br i1") || 
                              content.contains("switch") || 
                              content.contains("conditional");
        
        let has_classical_compute = content.contains("add") || 
                                   content.contains("mul") || 
                                   content.contains("div") ||
                                   content.contains("icmp") ||
                                   content.contains("fcmp");
        
        let gate_count = content.matches("__quantum__qis__").count();
        let qubit_count = content.matches("qubit").count();
        
        // Simple heuristic: Bell states and similar simple circuits
        let is_simple = !has_control_flow && 
                       !has_classical_compute && 
                       gate_count <= 10 && 
                       qubit_count <= 4;
        
        ProgramComplexity {
            is_simple,
            has_control_flow,
            has_classical_compute,
            gate_count,
            qubit_count,
        }
    }

    /// Helper method to compile the QIR file to a library
    fn compile_library(&self, output_dir: &Path) -> Result<PathBuf, PecosError> {
        debug!("QIR: Compiling QIR program to library in {:?}", output_dir);

        let output_dir_path = output_dir.to_path_buf();
        QirLinker::compile(&self.qir_file, Some(&output_dir_path))
            .map_err(|e| PecosError::Processing(format!("Failed to compile QIR program: {e}")))
    }
}

impl ClassicalEngine for QirEngine {
    /// Returns the number of qubits used in the quantum program
    ///
    /// Returns 0 if the qubit count cannot be determined.
    fn num_qubits(&self) -> usize {
        // Always analyze the QIR file to determine qubit count
        // Don't rely on measurement results from previous executions as they could be stale
        match self.analyze_qir_file() {
            Ok(num_qubits) => {
                debug!(
                    "QIR Engine: Determined {} qubits from QIR file analysis",
                    num_qubits
                );
                num_qubits
            }
            Err(e) => {
                warn!("QIR Engine: Could not determine qubit count: {}", e);
                // Fallback: check if we have measurement results from current execution
                if !self.measurement_results.is_empty() {
                    let max_result_id = self.measurement_results.keys().max().unwrap_or(&0);
                    let num_qubits = max_result_id + 1;
                    debug!(
                        "QIR Engine: Fallback to {} qubits from measurement results",
                        num_qubits
                    );
                    return num_qubits;
                }
                // Return 0 to indicate unknown qubit count
                warn!("QIR Engine: Returning 0 to indicate unknown qubit count");
                0
            }
        }
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        // When no commands are left to generate, create an empty message
        // instead of returning an error, to be consistent with other engines
        Ok(self
            .generate_commands_impl()?
            .unwrap_or_else(ByteMessage::create_empty))
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        self.process_measurements(&message)
    }

    fn get_results(&self) -> Result<Shot, PecosError> {
        Ok(self.get_results_impl())
    }

    fn compile(&self) -> Result<(), PecosError> {
        debug!("QIR: Compiling program");
        QirLinker::compile(&self.qir_file, None)
            .map(|_| debug!("QIR: Compilation successful"))
            .map_err(|e| {
                PecosError::Processing(format!(
                    "QIR compilation failed for '{}': {}",
                    self.qir_file.display(),
                    e
                ))
            })
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.reset_internal_state();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Clone for QirEngine {
    fn clone(&self) -> Self {
        debug!("QIR: Cloning engine");

        // Create a new engine with a fresh state
        Self {
            library: None,                       // Start with no library, will be loaded on demand
            measurement_results: HashMap::new(), // Start with empty measurements
            qir_file: self.qir_file.clone(),
            library_path: self.library_path.clone(),
            commands_generated: false,   // Reset commands_generated flag
            shot_count: 0,               // Reset shot count
            config: self.config.clone(), // Keep the configuration
            entry_point: self.entry_point.clone(), // Keep the detected entry point
        }
    }
}

impl Drop for QirEngine {
    fn drop(&mut self) {
        // Don't call reset_internal_state during drop to avoid segfaults
        // The QIR runtime has known cleanup issues that cause segfaults
        // Just clean up the basic state without touching the library
        debug!("QIR: Dropping engine - skipping library cleanup to avoid segfault");
        self.shot_count = 0;
        self.measurement_results.clear();
        self.commands_generated = false;
        // Note: self.library will be dropped automatically by Rust, which should be safe
    }
}

impl ControlEngine for QirEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        match self.generate_commands_impl()? {
            Some(commands) => Ok(EngineStage::NeedsProcessing(commands)),
            None => Ok(EngineStage::Complete(self.get_results()?)),
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // Handle measurements from quantum engine
        self.handle_measurements(measurements)?;

        // Check if we have more commands to process
        match self.generate_commands_impl()? {
            Some(commands) => Ok(EngineStage::NeedsProcessing(commands)),
            None => Ok(EngineStage::Complete(self.get_results()?)),
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.reset_internal_state();
        Ok(())
    }
}

impl Engine for QirEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, PecosError> {
        // Use the EngineStage pattern for processing
        let mut stage = self.start(input)?;

        while let EngineStage::NeedsProcessing(_commands) = stage {
            // In a real processing scenario, these commands would be sent to a quantum engine
            // Here we're just handling an empty processing case
            let measurements = ByteMessage::builder().build();
            stage = self.continue_processing(measurements)?;
        }

        // Extract the final result
        match stage {
            EngineStage::Complete(output) => Ok(output),
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.reset_internal_state();
        Ok(())
    }
}
