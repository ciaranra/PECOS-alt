//! LLVM Engine Module
//!
//! This module provides the LLVM Engine for executing quantum programs compiled to LLVM IR.
use crate::library::LlvmLibrary;
use crate::linker::LlvmLinker;
use crate::utils::{LLVM_LOG, log_error, retry_with_backoff};
use log::{debug, error, trace, warn};
use pecos_core::errors::PecosError;
use pecos_engines::Engine;
use pecos_engines::byte_message::ByteMessage;
use pecos_engines::engine_system::{ClassicalEngine, ControlEngine, EngineStage};
use pecos_engines::shot_results::Shot;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

/// Helper function to get the current thread ID as a string
///
/// This function returns the current thread ID formatted as a string.
/// It's used for logging and debugging purposes.
///
/// # Returns
///
/// A string representation of the current thread ID
#[must_use]
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Configuration options for the LLVM engine
#[derive(Debug, Clone, Default)]
pub struct LlvmEngineConfig {
    /// Number of shots assigned to this engine
    pub assigned_shots: usize,
    /// Whether to show verbose command logs
    pub verbose: bool,
    /// Maximum number of qubits allowed for allocation
    pub max_qubits: Option<usize>,
}

/// LLVM Engine for executing quantum programs in LLVM IR format
///
/// This engine loads and executes quantum programs that have been compiled to LLVM IR.
/// It supports any LLVM IR that follows the quantum runtime conventions for gate calls
/// and measurement operations.
pub struct LlvmEngine {
    /// The loaded LLVM library for executing quantum programs
    library: Option<Box<LlvmLibrary>>,

    /// Map of measurement results by `result_id`
    measurement_results: HashMap<usize, i64>,

    /// Path to the LLVM IR file to execute
    llvm_file: PathBuf,

    /// Path to the compiled library file
    library_path: Option<PathBuf>,

    /// Flag indicating whether commands have been generated for the current shot
    commands_generated: bool,

    /// Number of shots processed so far
    shot_count: usize,

    /// Configuration options for the engine
    config: LlvmEngineConfig,

    /// Entry point function name (detected from LLVM IR file)
    entry_point: Option<String>,
    
    /// Track if measurements have been processed via interactive execution
    measurements_processed_interactively: bool,
}

impl LlvmEngine {
    /// Create a new LLVM engine with default configuration
    ///
    /// # Arguments
    ///
    /// * `llvm_file` - Path to the LLVM IR file to execute
    ///
    /// # Returns
    ///
    /// A new LLVM engine instance with default configuration
    #[must_use]
    pub fn new(llvm_file: PathBuf) -> Self {
        debug!("LLVM: Creating new engine with program path: {llvm_file:?}");
        Self {
            library: None,
            measurement_results: HashMap::new(),
            llvm_file,
            library_path: None,
            commands_generated: false,
            shot_count: 0,
            config: LlvmEngineConfig::default(),
            entry_point: None,
            measurements_processed_interactively: false,
        }
    }

    /// Create a new LLVM engine with custom configuration
    ///
    /// # Arguments
    ///
    /// * `llvm_file` - Path to the LLVM IR file to execute
    /// * `config` - Configuration options for the engine
    ///
    /// # Returns
    ///
    /// A new LLVM engine instance with the specified configuration
    #[must_use]
    pub fn with_config(llvm_file: PathBuf, config: LlvmEngineConfig) -> Self {
        debug!("LLVM: Creating new engine with program path: {llvm_file:?} and custom config");
        Self {
            library: None,
            measurement_results: HashMap::new(),
            llvm_file,
            library_path: None,
            commands_generated: false,
            shot_count: 0,
            config,
            entry_point: None,
            measurements_processed_interactively: false,
        }
    }

    /// Set the number of shots assigned to this engine
    pub fn set_assigned_shots(&mut self, shots: usize) {
        debug!("LLVM: Setting assigned shots to {shots}");
        self.config.assigned_shots = shots;
    }

    /// Set whether to show verbose command logs
    pub fn set_verbose(&mut self, verbose: bool) {
        self.config.verbose = verbose;
    }

    /// Get the path to the LLVM IR file
    #[must_use]
    pub fn get_llvm_file(&self) -> &Path {
        &self.llvm_file
    }

    /// Reset the internal state of the engine
    fn reset_internal_state(&mut self) {
        debug!("LLVM: Resetting internal state");
        self.shot_count = 0;
        self.measurement_results.clear();
        self.measurements_processed_interactively = false;
        self.commands_generated = false;

        // Reset the LLVM runtime state through the library if it exists
        if let Some(ref library) = self.library {
            // Check if reset function exists (might not for empty circuits)
            if library.has_function(b"llvm_runtime_reset").unwrap_or(false) {
                if let Err(e) = library.reset() {
                    debug!("LLVM: Failed to reset LLVM runtime: {e}");
                }
            }
        }
    }

    /// Set up the LLVM library
    fn setup_library(&mut self) -> Result<(), PecosError> {
        // If the library is already set up, don't recompile
        if self.library.is_some() {
            trace!("LLVM: Library already set up, skipping compilation");
            return Ok(());
        }

        debug!("LLVM: Setting up library");

        // Clean up any existing library
        self.reset_internal_state();

        // Get or compile the library
        let library_path = if let Some(ref library_path) = self.library_path {
            debug!("LLVM: Using existing library at {library_path:?}");

            // Verify the library still exists
            if library_path.exists() {
                library_path.clone()
            } else {
                // Library was removed, need to recompile
                debug!("LLVM: Library no longer exists, recompiling");
                let output_dir = library_path
                    .parent()
                    .ok_or_else(|| PecosError::Processing("Invalid library path".to_string()))?;
                self.compile_library(output_dir)?
            }
        } else {
            // First time compilation - compile to the build directory
            debug!("LLVM: No existing library, compiling from source");
            let build_dir = self
                .llvm_file
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("build");

            // Ensure build directory exists
            if !build_dir.exists() {
                std::fs::create_dir_all(&build_dir)
                    .map_err(|e| log_error("LLVM Engine", "Failed to create build directory", e))?;
            }

            self.compile_library(&build_dir)?
        };

        // Load the library
        debug!("LLVM: Loading library from {library_path:?}");

        let library = LlvmLibrary::load(&library_path)
            .map_err(|e| log_error("LLVM Engine", "Failed to load LLVM library", e))?;

        // Store the library and path
        self.library = Some(Box::new(library));
        self.library_path = Some(library_path.clone());

        // Try to detect the entry point from the LLVM IR file
        if self.entry_point.is_none() {
            match crate::llvm_utils::find_entry_point(&self.llvm_file) {
                Ok(Some(entry_point)) => {
                    debug!("LLVM: Detected entry point function: {entry_point}");
                    self.entry_point = Some(entry_point);
                }
                Ok(None) => {
                    // No entry point found - log warning but don't fail yet
                    // The error will be caught in run_llvm_program
                    debug!("LLVM: No entry point detected from LLVM IR attributes");
                }
                Err(e) => {
                    // Failed to detect entry point - log warning but don't fail yet
                    debug!("LLVM: Failed to detect entry point: {e}");
                }
            }
        }

        debug!("LLVM: Successfully set up LLVM library");

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
        
        debug!("LLVM: Raw outcomes from quantum engine: {:?}", outcomes);
        debug!("LLVM: Number of outcomes: {}", outcomes.len());
        
        // Check if all measurements have already been processed interactively
        if let Some(library) = &self.library {
            if let Ok(executed_count) = library.get_measurements_executed() {
                if let Ok(all_ids) = library.get_measurement_result_ids() {
                    if executed_count >= all_ids.len() {
                        debug!("LLVM: All {} measurements already processed interactively, skipping", executed_count);
                        return Ok(());
                    }
                }
            }
        }

        // Get the result IDs from the runtime state
        let (result_ids, previously_executed) = if let Some(library) = &self.library {
            // Get the measurement result IDs that were tracked during execution
            if library.has_function(b"llvm_runtime_get_measurement_result_ids").unwrap_or(false) &&
               library.has_function(b"llvm_runtime_get_measurements_executed").unwrap_or(false) {
                match (library.get_measurement_result_ids(), library.get_measurements_executed()) {
                    (Ok(all_ids), Ok(executed_count)) => {
                        debug!("LLVM: Got {} result IDs from runtime: {:?}", all_ids.len(), all_ids);
                        debug!("LLVM: Previously executed measurements: {}", executed_count);
                        // Only take the result IDs for the NEW measurements
                        let new_ids: Vec<usize> = all_ids.into_iter()
                            .skip(executed_count)
                            .take(outcomes.len())
                            .collect();
                        debug!("LLVM: Using result IDs for new measurements: {:?}", new_ids);
                        (new_ids, executed_count)
                    },
                    _ => {
                        debug!("LLVM: Failed to get measurement tracking info");
                        // Fallback to sequential IDs
                        ((0..outcomes.len()).collect(), 0)
                    }
                }
            } else {
                // Fallback to sequential IDs if function not available
                ((0..outcomes.len()).collect(), 0)
            }
        } else {
            // No library, use sequential IDs
            ((0..outcomes.len()).collect(), 0)
        };

        // Verify we have the same number of result IDs as outcomes
        if result_ids.len() != outcomes.len() {
            return Err(PecosError::Processing(format!(
                "Mismatch between number of measurement outcomes ({}) and result IDs ({})",
                outcomes.len(),
                result_ids.len()
            )));
        }

        // Create measurements with the correct result IDs
        debug!("LLVM: About to zip result_ids={:?} with outcomes={:?}", result_ids, outcomes);
        debug!("LLVM: Previously executed: {}", previously_executed);
        let measurements: Vec<(usize, u32)> = result_ids
            .into_iter()
            .zip(outcomes.into_iter())
            .collect();
        
        debug!("LLVM: Zipped measurements (result_id, outcome): {:?}", measurements);

        self.measurement_results.clear();
        // Convert u32 measurements to i64 for LLVM standard
        self.measurement_results.extend(
            measurements
                .iter()
                .map(|(id, value)| (*id, i64::from(*value))),
        );

        // Update the runtime with measurement results
        if let Some(library) = &self.library {
            debug!(
                "LLVM: Updating runtime with {} measurement results",
                measurements.len()
            );

            // Check if runtime update functions exist (might not for empty circuits)
            let has_update = library
                .has_function(b"llvm_runtime_update_measurement_results")
                .unwrap_or(false);
            let has_finalize = library
                .has_function(b"llvm_runtime_finalize_shot")
                .unwrap_or(false);

            if has_update && has_finalize {
                // Convert measurements to the format expected by the runtime
                // The runtime expects pairs of (result_id, value)
                let mut results_data = Vec::with_capacity(measurements.len() * 2);
                for (idx, (result_id, value)) in measurements.iter().enumerate() {
                    debug!("LLVM: Measurement[{}] result_id={} value={} ({})", 
                           idx, result_id, value, if *value == 0 { "False" } else { "True" });
                    results_data.push(u32::try_from(*result_id).map_err(|_| {
                        PecosError::Resource(format!(
                            "Result ID {result_id} is too large to fit in u32"
                        ))
                    })?);
                    results_data.push(*value);
                }

                // Call the runtime update function
                library.update_measurement_results(&results_data)?;

                // Now finalize the shot with the measurement results
                library.finalize_shot()?;
            } else {
                debug!("LLVM: Runtime update/finalize functions not found, skipping measurement update");
            }
        }

        self.commands_generated = false;
        self.shot_count += 1;

        debug!("LLVM: Completed shot {}", self.shot_count);
        Ok(())
    }

    /// Get the results of the quantum computation
    ///
    /// # Returns
    ///
    /// * `Shot` - The results of the quantum computation
    ///
    /// # Panics
    ///
    /// * If no library is loaded (engine not properly initialized)
    /// * If the runtime returns no shot results after finalization
    /// * If there's an error getting shot results from the library
    fn get_results_impl(&self) -> Shot {
        // Get shot results from the runtime - this should always work
        if let Some(library) = &self.library {
            // Check if get_shot_results function exists (might not for empty circuits)
            let has_get_shot_results = library
                .has_function(b"llvm_runtime_get_shot_results")
                .unwrap_or(false);

            if has_get_shot_results {
                match library.get_shot_results() {
                    Ok(Some(shot)) => {
                        debug!(
                            "LLVM: Retrieved shot from runtime with {} registers: {:?}",
                            shot.data.len(),
                            shot.data.keys().collect::<Vec<_>>()
                        );
                        shot
                    }
                    Ok(None) => {
                        panic!(
                            "LLVM: Runtime returned no shot results after finalization - this indicates a bug in the LLVM runtime state management"
                        );
                    }
                    Err(e) => {
                        panic!("LLVM: Error getting shot results from library: {e}");
                    }
                }
            } else {
                // No runtime functions - return an empty shot for empty circuits
                debug!("LLVM: No get_shot_results function found, returning empty shot");
                Shot::default()
            }
        } else {
            panic!("LLVM: No library loaded - engine not properly initialized");
        }
    }

    /// Pre-compile the LLVM library to prepare for cloning
    ///
    /// # Errors
    ///
    /// Returns an error if the LLVM library cannot be pre-compiled.
    pub fn pre_compile(&mut self) -> Result<(), PecosError> {
        // Get the current thread ID for logging
        let thread_id = get_thread_id();

        debug!("LLVM: [Thread {thread_id}] Pre-compiling library for efficient cloning");

        // If the library is already set up, don't recompile
        if self.library.is_some() && self.library_path.is_some() {
            debug!("LLVM: [Thread {thread_id}] Library already pre-compiled, skipping");
            return Ok(());
        }

        // Compile the LLVM IR program to a library
        let library_path = LlvmLinker::compile(&self.llvm_file, None).map_err(|e| {
            PecosError::Processing(format!("Failed to compile LLVM IR program: {e}"))
        })?;

        // Detect the entry point from the LLVM IR file
        match crate::llvm_utils::find_entry_point(&self.llvm_file) {
            Ok(Some(entry_point)) => {
                debug!("LLVM: Detected entry point function: {entry_point}");
                self.entry_point = Some(entry_point);
            }
            Ok(None) => {
                // No entry point found - log but don't fail during pre-compile
                // The actual error will be thrown when trying to run the program
                debug!("LLVM: No entry point found in LLVM IR file during pre-compile");
                self.entry_point = None;
            }
            Err(e) => {
                // Failed to parse LLVM IR file - log but don't fail during pre-compile
                debug!("LLVM: Failed to detect entry point during pre-compile: {e}");
                self.entry_point = None;
            }
        }

        // Store the library path
        self.library_path = Some(library_path.clone());

        // We don't need to load the library here, as each thread will get its own copy
        debug!(
            "LLVM: [Thread {thread_id}] Library pre-compiled successfully (path: {library_path:?})"
        );

        Ok(())
    }

    /// Run the LLVM IR program and get the commands
    ///
    /// This method runs the LLVM IR program by calling the main function in the library
    /// and retrieves the generated quantum commands.
    ///
    /// # Arguments
    ///
    /// * `library` - The LLVM library to run
    ///
    /// # Returns
    ///
    /// * `Result<ByteMessage, PecosError>` - The binary message generated by the LLVM IR program
    ///
    /// # Error Handling
    ///
    /// Errors are propagated through the Result type and logged at their source with
    /// appropriate context, including the thread ID.
    fn run_llvm_program(&self, library: &LlvmLibrary) -> Result<ByteMessage, PecosError> {
        // Handle deferred measurements - some LLVM IR may call __quantum__rt__result_get_one()
        // to get measurement results, but those results aren't available until after
        // the quantum simulation runs. For now, we'll let it return 0s and rely on the
        // MonteCarloEngine to provide the actual measurement results through the Shot data structure.

        // Configure verbosity through environment variable
        if self.config.verbose {
            unsafe {
                std::env::remove_var("LLVM_RUNTIME_QUIET");
            }
        } else {
            unsafe {
                std::env::set_var("LLVM_RUNTIME_QUIET", "1");
            }
        }

        // Note: max_qubits is now set earlier in generate_commands_impl
        // to ensure it's configured before any LLVM code runs

        // Find and call the entry point function
        // First check if we already know the entry point for this program
        let entry_point = if let Some(ref ep) = self.entry_point {
            ep.clone()
        } else {
            // No entry point was detected - this is an error
            return Err(PecosError::Input(
                "No entry point found in LLVM IR program. The program must have a function \
                 marked with the 'EntryPoint' attribute. Example:\n\
                 define void @my_function() #0 {\n\
                   ...\n\
                 }\n\
                 attributes #0 = { \"EntryPoint\" }"
                    .to_string(),
            ));
        };

        // Check if the entry point function exists in the library
        if !library
            .has_function(entry_point.as_bytes())
            .unwrap_or(false)
        {
            return Err(PecosError::Input(format!(
                "Entry point function '{entry_point}' was marked with EntryPoint attribute but was not found \
                 in the compiled library. This may indicate a compilation error or that the function \
                 was optimized away. Ensure the function has a body and is not marked as internal."
            )));
        }

        debug!("LLVM: Calling entry point function: {entry_point}");
        library.call_function(entry_point.as_bytes()).map_err(|e| {
            // Special case for removed library files
            if e.to_string().contains("No such file or directory") {
                debug!("LLVM: Library file was already removed, continuing");
                PecosError::Processing("Library file was already removed".to_string())
            } else {
                log_error(
                    "LLVM Engine",
                    &format!("Failed to call {entry_point} function"),
                    e,
                )
            }
        })?;

        // Check if the LLVM runtime symbols are available
        // For empty circuits, these symbols might not be linked in
        let has_runtime_symbols = library
            .has_function(b"llvm_runtime_get_binary_commands")
            .unwrap_or(false);

        let runtime_message = if has_runtime_symbols {
            // Get the binary message generated by the LLVM runtime
            library.get_binary_commands().map_err(|e| {
                log_error(
                    "LLVM Engine",
                    "Failed to get binary commands from LLVM runtime",
                    e,
                )
            })?
        } else {
            // No runtime symbols available - this is likely an empty circuit
            debug!("LLVM: No runtime symbols found, assuming empty circuit");
            ByteMessage::create_empty()
        };

        // Log message details for debugging
        debug!(
            "LLVM: Binary message from runtime: {} bytes",
            runtime_message.as_bytes().len()
        );

        // Try to parse and log quantum operations for debugging
        if let Ok(operations) = runtime_message.quantum_ops() {
            debug!("LLVM: Parsed {} quantum operations:", operations.len());
            for (i, op) in operations.iter().enumerate() {
                debug!("LLVM:   [{i}] {op:?}");
            }
            if operations.len() > 10 {
                debug!("LLVM:   ... and {} more operations", operations.len() - 10);
            }
        }

        Ok(runtime_message)
    }

    fn generate_commands_impl(&mut self) -> Result<Option<ByteMessage>, PecosError> {
        // Only log at trace level to reduce verbosity
        trace!("LLVM: Generating commands (shot {})", self.shot_count + 1);

        // If we've already generated commands for this shot, return None
        if self.commands_generated {
            trace!("LLVM: Commands already generated for this shot, returning None");
            return Ok(None);
        }

        // If we've already processed a shot in this run_shot call, return None
        if self.shot_count > 0 {
            debug!("LLVM: Already processed one shot in this run_shot call, returning None");
            return Ok(None);
        }
        
        // Reset the runtime state at the beginning of command generation
        // This ensures each shot starts with a clean state
        if let Some(ref library) = self.library {
            // Check if reset function exists (might not for empty circuits)
            if library.has_function(b"llvm_runtime_reset").unwrap_or(false) {
                if let Err(e) = library.reset() {
                    debug!("LLVM: Failed to reset runtime before shot: {e}");
                } else {
                    debug!("LLVM: Reset runtime state for new shot");
                }
            } else {
                debug!("LLVM: No reset function found, skipping runtime reset");
            }
        }

        // Set max_qubits in the runtime AFTER reset but BEFORE any LLVM code runs
        // This ensures worker threads have the limit set before qubit allocation
        if let Some(max_qubits) = self.config.max_qubits {
            crate::runtime::core_runtime::set_max_qubits(max_qubits);
        }

        // Set up library if not already done
        if self.library.is_none() {
            debug!(
                "LLVM: Setting up library before generating commands for shot {}",
                self.shot_count + 1
            );

            // Set up the library with proper retry handling
            retry_with_backoff(
                || self.setup_library(),
                2,   // max attempts
                500, // initial delay in ms for file busy errors
            )
            .map_err(|e| {
                LLVM_LOG.warn(format!("Failed to set up library: {e}"));
                e
            })?;
        }

        // Run the LLVM IR program
        if let Some(library) = &self.library {
            // Run the LLVM IR program and get the ByteMessage directly
            let runtime_message = self.run_llvm_program(library)?;

            debug!(
                "LLVM: Got ByteMessage for shot {} with {} bytes",
                self.shot_count + 1,
                runtime_message.as_bytes().len()
            );

            // Mark that we've generated commands for this shot
            self.commands_generated = true;

            // Return the ByteMessage
            Ok(Some(runtime_message))
        } else {
            warn!("LLVM: No LLVM library loaded");
            Err(PecosError::Processing(
                "Cannot generate quantum commands: No LLVM library loaded. Call compile() or setup_library() first.".to_string(),
            ))
        }
    }

    /// Helper method to find qubit allocations in LLVM IR content
    fn find_qubit_allocations(content: &str) -> (usize, bool) {
        let mut max_qubit_index = 0;
        let mut found_allocation = false;

        // Pattern 1: Qubit allocations like "call i64 @__quantum__rt__qubit_allocate()"
        // Note: We must match "call" to avoid counting declarations
        let alloc_pattern = Regex::new(r"call\s+i64\s+@__quantum__rt__qubit_allocate\(\)")
            .expect("Invalid regex pattern for qubit allocations");
        let alloc_count = alloc_pattern.find_iter(content).count();
        if alloc_count > 0 {
            max_qubit_index = max_qubit_index.max(alloc_count - 1);
            found_allocation = true;
            debug!(
                "Pattern 1: Found {alloc_count} allocations, max_qubit_index = {max_qubit_index}"
            );
        }

        // Pattern 2: Integer-based qubit references in LLVM IR calls
        // We need to be more careful here to avoid matching result IDs in measurement calls
        
        // Pattern 2a: Single-qubit gates (h, x, y, z, s, t, etc.)
        let single_qubit_pattern =
            Regex::new(r"__quantum__qis__(?:h|x|y|z|s|t|sdg|tdg)__body\s*\(i64\s+(\d+)\)")
                .expect("Invalid regex for single-qubit gates");
        for cap in single_qubit_pattern.captures_iter(content) {
            if let Some(index_match) = cap.get(1) {
                if let Ok(index) = index_match.as_str().parse::<usize>() {
                    debug!("Pattern 2a: Found single-qubit gate on qubit {}", index);
                    max_qubit_index = max_qubit_index.max(index);
                    found_allocation = true;
                }
            }
        }

        // Pattern 2b: Two-qubit gates (cx/cnot, cz, etc.)
        let two_qubit_pattern =
            Regex::new(r"__quantum__qis__(?:cx|cnot|cz)__body\s*\(i64\s+(\d+),\s*i64\s+(\d+)\)")
                .expect("Invalid regex for two-qubit gates");
        for cap in two_qubit_pattern.captures_iter(content) {
            if let Some(control_match) = cap.get(1) {
                if let Ok(control) = control_match.as_str().parse::<usize>() {
                    debug!("Pattern 2b: Found two-qubit gate control qubit {}", control);
                    max_qubit_index = max_qubit_index.max(control);
                    found_allocation = true;
                }
            }
            if let Some(target_match) = cap.get(2) {
                if let Ok(target) = target_match.as_str().parse::<usize>() {
                    debug!("Pattern 2b: Found two-qubit gate target qubit {}", target);
                    max_qubit_index = max_qubit_index.max(target);
                    found_allocation = true;
                }
            }
        }

        // Pattern 2c: Measurement operations - only match the first parameter (qubit)
        let measurement_pattern =
            Regex::new(r"__quantum__qis__m__body\s*\(i64\s+(\d+),\s*i64\s+\d+\)")
                .expect("Invalid regex for measurements");
        for cap in measurement_pattern.captures_iter(content) {
            if let Some(qubit_match) = cap.get(1) {
                if let Ok(qubit) = qubit_match.as_str().parse::<usize>() {
                    debug!("Pattern 2c: Found measurement on qubit {}", qubit);
                    max_qubit_index = max_qubit_index.max(qubit);
                    found_allocation = true;
                }
            }
        }

        // Pattern 2d: Rotation gates with angle parameter
        let rotation_pattern =
            Regex::new(r"__quantum__qis__(?:rx|ry|rz)__body\s*\(double\s+[^,]+,\s*i64\s+(\d+)\)")
                .expect("Invalid regex for rotation gates");
        for cap in rotation_pattern.captures_iter(content) {
            if let Some(qubit_match) = cap.get(1) {
                if let Ok(qubit) = qubit_match.as_str().parse::<usize>() {
                    debug!("Pattern 2d: Found rotation gate on qubit {}", qubit);
                    max_qubit_index = max_qubit_index.max(qubit);
                    found_allocation = true;
                }
            }
        }

        (max_qubit_index, found_allocation)
    }

    /// Analyze the LLVM IR file to determine the number of qubits
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The LLVM IR file does not exist
    /// - The LLVM IR cannot be parsed
    /// - No qubit allocations are found in the file
    pub fn analyze_llvm_file(&self) -> Result<usize, PecosError> {
        debug!("LLVM Engine: Analyzing LLVM IR file: {:?}", self.llvm_file);

        // Check if the file exists
        if !self.llvm_file.exists() {
            return Err(PecosError::Resource(format!(
                "Unable to analyze LLVM IR file: File not found at path '{}'",
                self.llvm_file.display()
            )));
        }

        // Read the file content - using IO error directly
        let content = fs::read_to_string(&self.llvm_file)?;

        // Check if the file is empty
        if content.is_empty() {
            return Err(PecosError::Resource(format!(
                "Unable to analyze LLVM IR file: File is empty at path '{}'",
                self.llvm_file.display()
            )));
        }

        // Quick analysis for debugging
        let gate_count = content.matches("__quantum__qis__").count();
        let qubit_count = content.matches("qubit").count();
        debug!("LLVM Engine: Program stats - {gate_count} gates, {qubit_count} qubit references");

        // Find qubit allocations in the LLVM IR file
        let (max_qubit_index, found_allocation) = Self::find_qubit_allocations(&content);

        if found_allocation {
            // The number of qubits is the maximum index + 1
            let num_qubits = max_qubit_index + 1;
            debug!("LLVM Engine: Found {num_qubits} qubits in LLVM IR file");
            Ok(num_qubits)
        } else {
            Err(PecosError::Input(format!(
                "Invalid LLVM IR program: No qubit allocations found in file '{}'. The program must contain at least one qubit allocation.",
                self.llvm_file.display()
            )))
        }
    }

    /// Helper method to compile the LLVM IR file to a library
    fn compile_library(&self, output_dir: &Path) -> Result<PathBuf, PecosError> {
        debug!("LLVM: Compiling LLVM IR program to library in {output_dir:?}");

        let output_dir_path = output_dir.to_path_buf();
        LlvmLinker::compile(&self.llvm_file, Some(&output_dir_path))
            .map_err(|e| PecosError::Processing(format!("Failed to compile LLVM IR program: {e}")))
    }
}

impl ClassicalEngine for LlvmEngine {
    /// Returns the number of qubits used in the quantum program
    ///
    /// Returns 0 if the qubit count cannot be determined.
    fn num_qubits(&self) -> usize {
        // Always analyze the LLVM IR file to determine qubit count
        // Don't rely on measurement results from previous executions as they could be stale
        match self.analyze_llvm_file() {
            Ok(num_qubits) => {
                debug!("LLVM Engine: Determined {num_qubits} qubits from LLVM IR file analysis");
                num_qubits
            }
            Err(e) => {
                warn!(
                    "LLVM Engine: Could not determine qubit count from file {:?}: {}",
                    self.llvm_file, e
                );
                // Fallback: check if we have measurement results from current execution
                if !self.measurement_results.is_empty() {
                    let max_result_id = self.measurement_results.keys().max().unwrap_or(&0);
                    let num_qubits = max_result_id + 1;
                    debug!("LLVM Engine: Fallback to {num_qubits} qubits from measurement results");
                    return num_qubits;
                }
                // This is likely to cause issues - log an error
                error!(
                    "LLVM Engine: CRITICAL - Returning 0 qubits, this will cause runtime errors!"
                );
                error!("LLVM Engine: File path was: {:?}", self.llvm_file);
                error!("LLVM Engine: Error was: {e}");
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
        debug!("LLVM: Compiling program");
        LlvmLinker::compile(&self.llvm_file, None)
            .map(|_| debug!("LLVM: Compilation successful"))
            .map_err(|e| {
                PecosError::Processing(format!(
                    "LLVM compilation failed for '{}': {}",
                    self.llvm_file.display(),
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

impl Clone for LlvmEngine {
    fn clone(&self) -> Self {
        debug!("LLVM: Cloning engine");

        // Create a new engine with a fresh state like the working version
        Self {
            library: None,                       // Start with no library, will be loaded on demand
            measurement_results: HashMap::new(), // Start with empty measurements
            llvm_file: self.llvm_file.clone(),
            library_path: self.library_path.clone(),
            commands_generated: false,   // Reset commands_generated flag
            shot_count: 0,               // Reset shot count
            config: self.config.clone(), // Keep the configuration
            entry_point: self.entry_point.clone(), // Keep the detected entry point
            measurements_processed_interactively: false, // Reset interactive flag
        }
    }
}

impl Drop for LlvmEngine {
    fn drop(&mut self) {
        // Clean up engine state but not the library itself
        // The library is loaded with RTLD_NODELETE flag which prevents it from being
        // unloaded, ensuring symbols remain valid for other threads that might still
        // be using them. This is the proper solution for thread safety.
        LLVM_LOG.debug("Dropping engine - library remains loaded due to RTLD_NODELETE");
        self.shot_count = 0;
        self.measurement_results.clear();
        self.commands_generated = false;
    }
}

impl ControlEngine for LlvmEngine {
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

impl Engine for LlvmEngine {
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
