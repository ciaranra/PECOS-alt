//! `SeleneExecutableEngine` - A `ClassicalControlEngine` that runs Selene instances directly
//!
//! This engine uses Selene's `build()` API to create `SeleneInstance` objects and runs them
//! in-process with the `PecosSeleneBridgeSimulator` to communicate with PECOS via `ByteMessages`.

use pecos_core::prelude::PecosError;
use pecos_engines::{
    ByteMessage, ByteMessageBuilder, ClassicalEngine, ControlEngine, Data, Engine, EngineStage,
    GateType, Shot,
};
// MessageType is not exported, we'll match on the actual values
use crate::SeleneError;
use pecos_programs::{LlvmProgram, SeleneInterfaceProgram};
use std::{any::Any, collections::BTreeMap};

// Import the bridge interface from our bridge simulator
use crate::selene_ffi_to_bytemessage::{
    EngineInterface as FFIEngineInterface, initialize_engine_interface as initialize_ffi_interface,
};
use crate::selene_runtime_init::{SeleneRuntime, clear_current_instance, set_current_instance};
use pecos_selene_bridge::EngineInterface;

use std::io::{BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};

/// Represents a running Selene instance
pub struct SeleneInstance {
    /// Path to the Selene executable
    pub executable: std::path::PathBuf,
    /// Path to the artifacts directory
    pub artifacts: std::path::PathBuf,
    /// Number of qubits for this instance
    pub num_qubits: usize,
    /// The running process (if started)
    process: Option<Child>,
    /// Process stdin for sending commands
    stdin: Option<BufWriter<std::process::ChildStdin>>,
    /// Process stdout for reading results
    stdout: Option<BufReader<std::process::ChildStdout>>,
}

impl SeleneInstance {
    /// Create a new `SeleneInstance` from paths
    #[must_use]
    pub fn new(
        executable: std::path::PathBuf,
        artifacts: std::path::PathBuf,
        num_qubits: usize,
    ) -> Self {
        Self {
            executable,
            artifacts,
            num_qubits,
            process: None,
            stdin: None,
            stdout: None,
        }
    }

    /// Create a configuration file for Selene executable
    fn create_selene_config(&self) -> Result<std::path::PathBuf, PecosError> {
        use std::fs::File;
        use tempfile::tempdir;

        log::debug!("create_selene_config() called");

        // Create a temporary directory for the config
        let temp_dir = tempdir()
            .map_err(|e| PecosError::Processing(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("selene_config.json");

        // Find the bridge plugin - check multiple locations
        // Use absolute paths for the config file
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let bridge_plugin = if cwd
            .join("target/release/libpecos_selene_bridge.so")
            .exists()
        {
            cwd.join("target/release/libpecos_selene_bridge.so")
        } else if cwd.join("target/debug/libpecos_selene_bridge.so").exists() {
            cwd.join("target/debug/libpecos_selene_bridge.so")
        } else if cwd
            .join("target/release/deps/libpecos_selene_bridge.so")
            .exists()
        {
            cwd.join("target/release/deps/libpecos_selene_bridge.so")
        } else if cwd
            .join("target/debug/deps/libpecos_selene_bridge.so")
            .exists()
        {
            cwd.join("target/debug/deps/libpecos_selene_bridge.so")
        } else {
            return Err(PecosError::Processing(
                "Bridge plugin not found in any expected location".to_string(),
            ));
        };

        log::info!("Using Bridge plugin at: {}", bridge_plugin.display());

        // Find the ideal error model plugin - check multiple locations
        let ideal_plugin = if std::path::Path::new(
            "/home/ciaranra/Repos/cl_projects/gup/selene/target/release/libselene_ideal_plugin.so",
        )
        .exists()
        {
            std::path::PathBuf::from(
                "/home/ciaranra/Repos/cl_projects/gup/selene/target/release/libselene_ideal_plugin.so",
            )
        } else {
            std::path::PathBuf::from(
                "/home/ciaranra/Repos/cl_projects/gup/PECOS/.venv/lib/python3.12/site-packages/selene_ideal_error_model_plugin/_dist/lib/libselene_ideal_plugin.so",
            )
        };

        let runtime_plugin = if std::path::Path::new("/home/ciaranra/Repos/cl_projects/gup/selene/target/release/libselene_simple_runtime.so").exists() {
            std::path::PathBuf::from("/home/ciaranra/Repos/cl_projects/gup/selene/target/release/libselene_simple_runtime.so")
        } else {
            std::path::PathBuf::from("/home/ciaranra/Repos/cl_projects/gup/PECOS/.venv/lib/python3.12/site-packages/selene_simple_runtime_plugin/_dist/lib/libselene_simple_runtime.so")
        };

        // Create the configuration JSON
        let config_json = serde_json::json!({
            "simulator": {
                "name": "pecos_selene_bridge",
                "file": bridge_plugin.to_string_lossy(),
                "args": []
            },
            "shots": {
                "count": 1,
                "offset": 0,
                "increment": 1
            },
            "n_qubits": self.num_qubits,
            "output_stream": "stderr",
            "artifact_dir": self.artifacts.to_string_lossy(),
            "error_model": {
                "name": "selene_ideal_error_model_plugin.IdealErrorModel",
                "file": ideal_plugin.to_string_lossy(),
                "args": []
            },
            "runtime": {
                "name": "selene_simple_runtime_plugin.SimpleRuntime",
                "file": runtime_plugin.to_string_lossy(),
                "args": []
            },
            "event_hooks": {
                "shot_start": [],
                "shot_end": [],
                "shot_fail": []
            }
        });

        // Write the configuration to file
        // log::debug!("*** ENGINE: Writing Selene config with n_qubits={} ***", self.num_qubits);
        // log::debug!("*** ENGINE: Config JSON: {} ***", config_json.to_string());
        let mut file = File::create(&config_path)
            .map_err(|e| PecosError::Processing(format!("Failed to create config file: {e}")))?;
        file.write_all(config_json.to_string().as_bytes())
            .map_err(|e| PecosError::Processing(format!("Failed to write config: {e}")))?;
        file.sync_all()
            .map_err(|e| PecosError::Processing(format!("Failed to sync config file: {e}")))?;
        drop(file);

        log::debug!(
            "Created config file at: {} (exists: {})",
            config_path.display(),
            config_path.exists()
        );

        // Leak the temp_dir to keep it alive
        std::mem::forget(temp_dir);

        Ok(config_path)
    }

    /// Start the Selene executable process
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create configuration file
    /// - Failed to spawn the Selene process
    /// - Failed to establish IPC connection
    pub fn start(&mut self) -> Result<(), PecosError> {
        // log::debug!("*** ENGINE: SeleneInstance.start() called ***");
        if self.process.is_some() {
            // log::debug!("*** ENGINE: Process already running, skipping start ***");
            log::debug!("Process already started, reusing existing process");
            return Ok(()); // Already started
        }

        // log::debug!("*** ENGINE: Starting new Selene process ***");
        log::info!("Starting Selene executable: {}", self.executable.display());

        // Create the runtime configuration for Selene with Bridge plugin
        let config = self.create_selene_config()?;

        log::debug!(
            "Runtime config file created at: {} (size: {} bytes)",
            config.display(),
            std::fs::metadata(&config).map(|m| m.len()).unwrap_or(0)
        );

        // Check if selene.yaml exists in the parent directory (where it should be)
        let parent_dir = self.artifacts.parent().ok_or_else(|| {
            PecosError::Processing("No parent directory for artifacts".to_string())
        })?;
        let selene_yaml = parent_dir.join("selene.yaml");

        if selene_yaml.exists() {
            log::info!(
                "Found selene.yaml at: {} (size: {} bytes)",
                selene_yaml.display(),
                std::fs::metadata(&selene_yaml)
                    .map(|m| m.len())
                    .unwrap_or(0)
            );

            // Selene executable expects the HUGR program to be available
            // The executable was built with this HUGR program compiled in
            log::debug!(
                "Selene executable should have HUGR program compiled in from: {}",
                selene_yaml.display()
            );
        } else {
            log::warn!("No selene.yaml found at: {}", selene_yaml.display());
        }

        // Start the Selene executable with configuration
        let mut cmd = Command::new(&self.executable);
        cmd.arg("--configuration").arg(&config);

        // log::debug!("*** ENGINE: Starting Selene with config: {:?} ***", config);
        log::debug!(
            "Executing command: {} --configuration {}",
            self.executable.display(),
            config.display()
        );

        // Create IPC marker file BEFORE starting the process to signal Bridge plugin to use IPC mode
        let ipc_marker = self.artifacts.join("pecos_ipc_mode");
        std::fs::write(&ipc_marker, "1")
            .map_err(|e| PecosError::Processing(format!("Failed to create IPC marker: {e}")))?;

        // log::debug!("*** ENGINE: Created IPC marker at {:?} ***", ipc_marker);
        log::info!(
            "IPC mode enabled: created marker at {}",
            ipc_marker.display()
        );

        // Write a config file with the correct number of qubits for Bridge to read
        let config_path = self.artifacts.join("pecos_config.json");
        let config_json = serde_json::json!({
            "n_qubits": self.num_qubits,
        });
        std::fs::write(&config_path, config_json.to_string()).map_err(|e| {
            PecosError::Processing(format!("Failed to write pecos_config.json: {e}"))
        })?;
        // log::debug!("*** ENGINE: Created config with n_qubits={} at {:?} ***", self.num_qubits, config_path);

        // Verify the marker file was created
        if !ipc_marker.exists() {
            return Err(PecosError::Processing(format!(
                "IPC marker file not found after creation: {}",
                ipc_marker.display()
            )));
        }

        // Pass artifact directory to Bridge via environment
        // This tells the Bridge plugin where to find the IPC marker
        let artifacts_str = self.artifacts.to_string_lossy().to_string();
        cmd.env("SELENE_ARTIFACTS_DIR", &artifacts_str);

        // CRITICAL: Set SELENE_IPC to enable IPC mode in the Bridge plugin
        cmd.env("SELENE_IPC", "1");

        // log::debug!("*** ENGINE: Set SELENE_ARTIFACTS_DIR='{}' for Bridge plugin ***", artifacts_str);
        // log::debug!("*** ENGINE: Set SELENE_IPC='1' to enable IPC mode ***");
        log::info!("Set SELENE_ARTIFACTS_DIR='{artifacts_str}' for Bridge plugin");
        log::info!("Set SELENE_IPC='1' to enable IPC mode");

        // Configure stdio for IPC communication
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Start the process
        let mut child = cmd.spawn().map_err(|e| {
            PecosError::Processing(format!("Failed to start Selene executable: {e}"))
        })?;

        // Get handles for IPC communication
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PecosError::Processing("Failed to get stdin handle".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PecosError::Processing("Failed to get stdout handle".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PecosError::Processing("Failed to get stderr handle".to_string()))?;

        // Monitor stderr in a thread
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                log::info!("SELENE: {line}");
            }
        });

        self.stdin = Some(BufWriter::new(stdin));
        self.stdout = Some(BufReader::new(stdout));
        self.process = Some(child);

        // Process should be ready immediately with proper IPC setup
        // No sleep needed here

        log::info!("Selene executable started successfully");
        Ok(())
    }

    /// Stop the Selene executable process
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to terminate
    pub fn stop(&mut self) -> Result<(), PecosError> {
        if let Some(mut process) = self.process.take() {
            log::info!("Stopping Selene executable");

            // Try to terminate gracefully first
            if let Some(mut stdin) = self.stdin.take() {
                let _ = stdin.write_all(b"exit\n");
                let _ = stdin.flush();
            }

            // Give process a small chance to exit gracefully
            // but don't wait too long
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Force kill if still running
            let _ = process.kill();
            let _ = process.wait();

            self.stdout = None;
            log::info!("Selene executable stopped");
        }
        Ok(())
    }

    /// Run a shot on the Selene instance
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Shot execution fails
    /// - IPC communication fails
    ///
    /// # Panics
    ///
    /// Panics if stderr flush fails
    pub fn run_shot(&mut self, shot_id: u64) -> Result<(), PecosError> {
        // log::debug!("*** ENGINE: SeleneInstance.run_shot({}) called ***", shot_id);
        std::io::stderr().flush().unwrap(); // Force flush
        // panic!("DEBUG: run_shot was called!");  // Uncomment to verify
        log::debug!("SeleneInstance::run_shot({shot_id}) called");

        // For proper isolation, always stop any existing process and start fresh
        // This ensures each shot gets a clean subprocess
        if self.process.is_some() {
            // log::debug!("*** ENGINE: Stopping existing process before new shot ***");
            log::debug!("Stopping existing process before starting new shot");
            self.stop()?;
        }

        // Start a fresh process for this shot
        // Note: The Selene executable starts running immediately when spawned,
        // it doesn't wait for a "shot" command. The Bridge plugin will begin
        // sending ByteMessages on stdout as soon as it starts.
        // log::debug!("*** ENGINE: Calling start() to launch Selene process ***");
        self.start()?;

        // Don't send a shot command - the executable is already running
        log::debug!("Selene process started, Bridge plugin should be sending operations");

        Ok(())
    }

    /// Try to read `ByteMessages` from the subprocess stdout (IPC)
    fn try_read_ipc_messages(&mut self) -> Result<Vec<ByteMessage>, PecosError> {
        use std::io::Read;

        let mut messages = Vec::new();

        // First check if the process is still running
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited
                    if status.success() {
                        // Process exited successfully - this is expected after shot completion
                        log::info!("Selene subprocess completed successfully");
                        self.process = None; // Clear the process handle
                        return Ok(messages); // Return any messages we've collected (empty)
                    }
                    return Err(PecosError::Processing(format!(
                        "Selene subprocess failed with status: {status}"
                    )));
                }
                Ok(None) => {
                    // Process is still running - good
                }
                Err(e) => {
                    return Err(PecosError::Processing(format!(
                        "Failed to check subprocess status: {e}"
                    )));
                }
            }
        }

        if let Some(ref mut stdout) = self.stdout {
            log::trace!("Trying to read IPC messages from subprocess stdout");

            // Try to read one length-prefixed message (simplified protocol)
            // Try to read the length prefix (4 bytes) with a timeout check
            let mut len_bytes = [0u8; 4];

            // Use non-blocking read to avoid infinite waits
            match stdout.read_exact(&mut len_bytes) {
                Ok(()) => {
                    let msg_len = usize::try_from(u32::from_le_bytes(len_bytes))
                        .expect("Message length should fit in usize");
                    log::trace!("Message length: {msg_len} bytes");

                    // Read the message data
                    let mut msg_bytes = vec![0u8; msg_len];
                    match stdout.read_exact(&mut msg_bytes) {
                        Ok(()) => {
                            log::trace!("Read {} bytes of message data", msg_bytes.len());

                            // Create ByteMessage from the data
                            let message = ByteMessage::new(&msg_bytes);
                            messages.push(message);

                            // For now, only read one message at a time to avoid blocking
                            log::trace!("Successfully read one message, returning");
                        }
                        Err(e) => {
                            log::debug!("Failed to read message data: {e}");
                        }
                    }
                }
                Err(e) => {
                    if messages.is_empty() {
                        log::trace!("No data available from subprocess: {e}");
                    } else {
                        log::trace!("No more messages available: {e}");
                    }
                }
            }
        } else {
            log::warn!("No stdout available for reading IPC messages");
        }

        log::trace!("Read {} IPC messages", messages.len());
        Ok(messages)
    }

    /// Send a `ByteMessage` to the subprocess via stdin (IPC)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Process is not running
    /// - Failed to serialize message
    /// - Failed to write to stdin
    pub fn send_ipc_message(&mut self, message: &ByteMessage) -> Result<(), PecosError> {
        // Check if process is still running before sending
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(Some(status)) => {
                    return Err(PecosError::Processing(format!(
                        "Cannot send message - subprocess exited with status: {status}"
                    )));
                }
                Ok(None) => {
                    // Process is running
                }
                Err(e) => {
                    return Err(PecosError::Processing(format!(
                        "Failed to check subprocess status: {e}"
                    )));
                }
            }
        }

        if let Some(ref mut stdin) = self.stdin {
            log::trace!("Sending ByteMessage to subprocess via IPC");

            let bytes = message.as_bytes();

            // Send message with simple length prefix (matching Bridge simulator protocol)
            // Write length as 4 bytes
            let len_bytes = u32::try_from(bytes.len())
                .map_err(|_| PecosError::Processing("Message too large".to_string()))?
                .to_le_bytes();
            stdin
                .write_all(&len_bytes)
                .map_err(|e| PecosError::Processing(format!("Failed to write IPC length: {e}")))?;

            // Write the actual message bytes
            stdin
                .write_all(bytes)
                .map_err(|e| PecosError::Processing(format!("Failed to write IPC message: {e}")))?;

            stdin
                .flush()
                .map_err(|e| PecosError::Processing(format!("Failed to flush IPC message: {e}")))?;

            log::trace!("Sent {} bytes via IPC (length-prefixed)", bytes.len());
            Ok(())
        } else {
            Err(PecosError::Processing(
                "No stdin available for IPC communication".to_string(),
            ))
        }
    }
}

/// Configuration for Selene instance creation
#[derive(Clone)]
pub struct SeleneExecutableConfig {
    /// Number of qubits
    pub num_qubits: usize,

    /// Working directory for temporary files
    pub working_dir: Option<std::path::PathBuf>,

    /// Whether to enable verbose output
    pub verbose: bool,

    /// Path to the bridge simulator plugin (auto-detected if not specified)
    pub plugin_path: Option<std::path::PathBuf>,

    /// Path to pre-compiled Selene executable
    pub executable_path: Option<std::path::PathBuf>,

    /// Path to Selene artifacts directory
    pub artifacts_path: Option<std::path::PathBuf>,
}

/// A `ClassicalControlEngine` that runs Selene instances directly with `PecosSeleneBridgeSimulator`
pub struct SeleneExecutableEngine {
    /// Configuration for the Selene instance
    config: SeleneExecutableConfig,

    /// The loaded program (compiled from HUGR)
    program: Option<SeleneInterfaceProgram>,

    /// LLVM program (for backward compatibility)
    llvm_program: Option<LlvmProgram>,

    /// Built Selene instance (created from HUGR via `build()` API)
    selene_instance: Option<SeleneInstance>,

    /// The initialized Selene runtime (when using real libselene.so)
    selene_runtime: Option<SeleneRuntime>,

    /// `ByteMessage` queue for operations sent from bridge simulator
    operation_queue: Vec<ByteMessage>,

    /// Current measurement results
    measurement_results: BTreeMap<String, Data>,

    /// Shot counter
    shot_count: u64,

    /// Reusable message builder
    _message_builder: ByteMessageBuilder,

    /// Whether the Interface Plugin has been executed for this shot
    plugin_executed: bool,

    /// Flag to indicate if we're in `ControlEngine` mode (with `QuantumSystem`)
    control_engine_mode: bool,

    /// Quantum simulator for standalone mode (when not using `QuantumSystem`)
    quantum_sim: Option<Box<dyn pecos_engines::quantum::QuantumEngine>>,

    /// Counter for tracking total measurements across IPC calls
    total_measurement_count: usize,
}

/// Execute quantum operations and return measurement outcomes (standalone helper)
fn execute_quantum_ops(
    ops: &[pecos_engines::Gate],
    num_qubits: usize,
) -> Result<Vec<usize>, PecosError> {
    use pecos_engines::quantum::StateVecEngine;

    // Create a quantum simulator
    let mut quantum_sim = StateVecEngine::new(num_qubits);

    // Build a ByteMessage with all the operations
    let mut msg_builder = ByteMessage::builder();
    let _ = msg_builder.for_quantum_operations();
    for op in ops {
        msg_builder.add_gate_command(op);
    }
    let ops_message = msg_builder.build();

    // Execute all operations and get measurement results
    let result_msg = quantum_sim.process(ops_message)?;
    let mut outcomes = Vec::new();

    if let Ok(meas_outcomes) = result_msg.outcomes() {
        // Convert u32 outcomes to usize
        outcomes.extend(
            meas_outcomes
                .iter()
                .map(|&x| usize::try_from(x).expect("Measurement outcome should fit in usize")),
        );
    }

    log::debug!(
        "Got {} real measurement outcomes from quantum simulator",
        outcomes.len()
    );
    Ok(outcomes)
}

impl SeleneExecutableEngine {
    /// Create a new engine
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Number of qubits is 0 or exceeds maximum supported (64)
    /// - Failed to locate Selene executable
    pub fn new(num_qubits: usize) -> Result<Self, PecosError> {
        log::debug!("SeleneExecutableEngine::new({num_qubits}) called");

        // Validate num_qubits
        if num_qubits == 0 {
            return Err(PecosError::Input(
                "Number of qubits must be greater than 0".to_string(),
            ));
        }

        let config = SeleneExecutableConfig {
            num_qubits,
            working_dir: None,
            verbose: false,
            plugin_path: None,
            executable_path: None,
            artifacts_path: None,
        };

        Ok(Self {
            config,
            program: None,
            llvm_program: None,
            selene_instance: None,
            selene_runtime: None,
            operation_queue: Vec::new(),
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            _message_builder: ByteMessageBuilder::new(),
            plugin_executed: false,
            control_engine_mode: false,
            quantum_sim: None,
            total_measurement_count: 0,
        })
    }

    /// Set the program to execute
    #[must_use]
    pub fn with_program(mut self, program: SeleneInterfaceProgram) -> Self {
        // If the program contains executable paths, store them in config
        if let Some(exec_path) = &program.executable_path {
            self.config.executable_path = Some(std::path::PathBuf::from(exec_path));
        }
        if let Some(artifacts_path) = &program.artifacts_path {
            self.config.artifacts_path = Some(std::path::PathBuf::from(artifacts_path));
        }
        self.program = Some(program);
        self
    }

    /// Set an LLVM program (for backward compatibility)
    #[must_use]
    pub fn with_llvm_program(mut self, program: LlvmProgram) -> Self {
        self.llvm_program = Some(program);
        self
    }

    /// Set the working directory
    #[must_use]
    pub fn with_working_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.config.working_dir = Some(dir);
        self
    }

    /// Enable verbose output
    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }

    /// Set the plugin path
    #[must_use]
    pub fn with_plugin_path(mut self, path: std::path::PathBuf) -> Self {
        self.config.plugin_path = Some(path);
        self
    }

    /// Build the Selene instance from HUGR and prepare for execution
    fn build_selene_instance(&mut self) -> Result<(), PecosError> {
        use pecos_selene_bridge::initialize_engine_interface;
        use std::sync::{Arc, Mutex};

        log::info!("SeleneExecutableEngine: build_selene_instance() called");

        // If we already have a SeleneInstance, don't recreate it
        if self.selene_instance.is_some() {
            log::info!("SeleneInstance already exists, reusing it");
            log::debug!("SeleneInstance already exists, reusing for next shot");
            return Ok(());
        }

        // Check if we have either a SeleneInterfaceProgram or an LlvmProgram
        if self.program.is_none() && self.llvm_program.is_none() {
            return Err(SeleneError::NoProgramSpecified.into());
        }

        // If we have an LLVM program, we're in test mode - just return OK
        // The actual execution will be handled differently
        if self.llvm_program.is_some() {
            log::info!("LLVM program provided - using test mode execution path");
            return Ok(());
        }

        // Check if we have a pre-compiled executable
        if let (Some(exec_path), Some(artifacts_path)) =
            (&self.config.executable_path, &self.config.artifacts_path)
        {
            log::info!(
                "Using pre-compiled Selene executable at: {}",
                exec_path.display()
            );
            log::info!("  Artifacts at: {}", artifacts_path.display());
            log::debug!("Creating NEW SeleneInstance for executable");

            // Create a real SeleneInstance with the pre-compiled executable
            self.selene_instance = Some(SeleneInstance::new(
                exec_path.clone(),
                artifacts_path.clone(),
                self.config.num_qubits,
            ));

            // Initialize the engine interface for bridge communication
            initialize_engine_interface(Arc::new(Mutex::new(self.clone())));

            return Ok(());
        }

        log::info!("Building SeleneInstance from HUGR with PecosSeleneBridgeSimulator");

        // Get the path to the PecosSeleneBridgeSimulator plugin
        // The plugin is built as a cdylib in the target directory
        let bridge_plugin_path = if let Some(ref custom_path) = self.config.plugin_path {
            // Use custom plugin path if provided
            custom_path.clone()
        } else {
            // Try to find the bridge plugin library
            // Use absolute paths based on the workspace root
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().and_then(|p| p.parent()).map_or_else(
                || std::path::PathBuf::from("."),
                std::path::Path::to_path_buf,
            );

            let possible_paths = vec![
                // Development build
                workspace_root.join("target/debug/libpecos_selene_bridge.so"),
                workspace_root.join("target/debug/libpecos_selene_bridge.dylib"),
                workspace_root.join("target/debug/pecos_selene_bridge.dll"),
                // Release build
                workspace_root.join("target/release/libpecos_selene_bridge.so"),
                workspace_root.join("target/release/libpecos_selene_bridge.dylib"),
                workspace_root.join("target/release/pecos_selene_bridge.dll"),
            ];

            possible_paths.into_iter()
                .find(|p| p.exists())
                .ok_or_else(|| SeleneError::RuntimeError(
                    "Could not find PecosSeleneBridgeSimulator plugin library. Make sure to build with: cargo build --package pecos-selene-engine-bridge".to_string()
                ))?
        };

        log::info!("Using bridge plugin at: {}", bridge_plugin_path.display());

        // Initialize the engine interface so the bridge can communicate with us
        initialize_engine_interface(Arc::new(Mutex::new(self.clone())));

        // Note: The actual Selene build process happens in Python using selene_sim.build()
        // The Python side compiles HUGR to a Selene executable and passes the paths here.
        // Since we're called from build_selene_instance() without a pre-compiled executable,
        // this path should not normally be reached - compilation should happen in Python.

        // If we reach here, it means we're trying to build from HUGR in Rust,
        // which is not the intended flow (Python is more natural for Selene).
        Err(SeleneError::CompilationError(
            "Selene executable compilation should happen in Python before creating the engine. \
             Use sim_wrapper.py to compile Guppy/HUGR to Selene executable."
                .to_string(),
        )
        .into())
    }

    /// Execute Selene Interface Plugin in-process for a shot
    fn execute_selene_shot(&mut self) -> Result<(), PecosError> {
        use std::io::Write;
        std::io::stderr().flush().unwrap();

        log::info!(
            "SeleneExecutableEngine: execute_selene_shot() called for shot {}",
            self.shot_count
        );

        // Check if we have a pre-compiled executable (from selene_sim.build())
        if self.selene_instance.is_some() {
            self.execute_precompiled_instance()?;
        }

        // Clone the program to avoid borrowing issues
        if let Some(program) = self.program.clone() {
            self.execute_program_plugin(&program)?;
        } else if let Some(_llvm_program) = &self.llvm_program {
            // For LLVM programs, we need a different execution path
            // For now, just log that we have an LLVM program
            log::info!("LLVM program execution requested - returning empty shot");
            // In the future, this would compile and execute the LLVM program
        }

        Ok(())
    }

    /// Execute a pre-compiled Selene instance
    fn execute_precompiled_instance(&mut self) -> Result<(), PecosError> {
        if let Some(ref mut instance) = self.selene_instance {
            log::debug!(
                "Running pre-compiled Selene executable for shot {}",
                self.shot_count
            );
            log::info!(
                "Running pre-compiled Selene executable for shot {}",
                self.shot_count
            );

            // Run a shot on the Selene executable
            match instance.run_shot(self.shot_count) {
                Ok(()) => {
                    log::debug!("run_shot succeeded, continuing...");
                }
                Err(e) => {
                    log::warn!("run_shot failed: {e:?}");
                    return Err(e);
                }
            }

            log::debug!("After run_shot, subprocess started");
            log::debug!("Subprocess started, Bridge will communicate via IPC");
            log::trace!("DEBUG MARKER 12345");
            log::info!("Selene subprocess started for IPC communication");

            // Check if we're in control engine mode (with QuantumSystem)
            log::debug!("control_engine_mode = {}", self.control_engine_mode);
            if self.control_engine_mode {
                log::debug!("In ControlEngine mode - IPC handled by start()/continue_processing()");
                self.plugin_executed = true;
                return Ok(());
            }

            // In standalone mode (no QuantumSystem), handle IPC directly
            log::debug!("In standalone mode - handling IPC directly");
            self.handle_standalone_ipc()
        } else {
            Err(PecosError::Processing(
                "No Selene instance available".to_string(),
            ))
        }
    }

    /// Handle IPC communication in standalone mode
    fn handle_standalone_ipc(&mut self) -> Result<(), PecosError> {
        const MAX_TIMEOUT_ITERATIONS: u32 = 500; // 5 seconds total

        if let Some(ref mut instance) = self.selene_instance {
            // Send an empty quantum operations message to signal the Bridge to start
            let start_message = ByteMessage::builder().for_quantum_operations().build();
            instance.send_ipc_message(&start_message)?;

            // Process operations from the bridge until complete
            let mut timeout_counter = 0;

            loop {
                let messages = instance.try_read_ipc_messages()?;

                if messages.is_empty() {
                    // Check if process has exited
                    if instance.process.is_none() {
                        log::debug!("Selene process has exited - shot complete");
                        self.plugin_executed = true;
                        return Ok(());
                    }

                    // No messages yet, brief yield to avoid busy-waiting
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    timeout_counter += 1;

                    if timeout_counter > MAX_TIMEOUT_ITERATIONS * 10 {
                        return Err(PecosError::Processing(
                            "Timeout waiting for Bridge response".to_string(),
                        ));
                    }
                    continue;
                }

                timeout_counter = 0; // Reset timeout when we get a message

                for message in messages {
                    // Store the operations for later processing
                    self.operation_queue.push(message.clone());

                    // Check if this is the completion signal
                    if message.is_empty()? {
                        log::debug!("Received empty message - execution complete");
                        self.plugin_executed = true;
                        return Ok(());
                    }

                    // Process quantum operations - prepare the response first
                    let response_message = match message.quantum_ops() {
                        Ok(ops) => {
                            log::debug!("Received {} quantum operations", ops.len());

                            // Count measurements
                            let measurement_count = ops
                                .iter()
                                .filter(|op| {
                                    matches!(op.gate_type, pecos_engines::GateType::Measure)
                                })
                                .count();

                            log::debug!("Total measurements found: {measurement_count}");

                            if measurement_count > 0 {
                                // Execute operations and prepare response without borrowing instance
                                let outcomes = execute_quantum_ops(&ops, self.config.num_qubits)?;

                                // Store the measurement results
                                for &outcome in &outcomes {
                                    self.total_measurement_count += 1;
                                    let key =
                                        format!("measurement_{}", self.total_measurement_count);
                                    log::trace!("Storing measurement {key} = {outcome}");
                                    self.measurement_results.insert(
                                        key,
                                        Data::U32(u32::try_from(outcome).unwrap_or(u32::MAX)),
                                    );
                                }
                                log::trace!(
                                    "Now have {} stored measurements",
                                    self.measurement_results.len()
                                );

                                let mut builder = ByteMessage::builder();
                                let _ = builder.for_outcomes();
                                builder.add_outcomes(&outcomes);
                                builder.build()
                            } else {
                                ByteMessage::builder().for_outcomes().build()
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse quantum ops: {e}");
                            // Send empty measurements as fallback
                            ByteMessage::builder().for_outcomes().build()
                        }
                    };

                    // Now send the prepared response
                    instance.send_ipc_message(&response_message)?;
                }
            }
        } else {
            Err(PecosError::Processing(
                "No Selene instance available".to_string(),
            ))
        }
    }

    /// Execute a program plugin
    fn execute_program_plugin(
        &mut self,
        program: &SeleneInterfaceProgram,
    ) -> Result<(), PecosError> {
        // Only try to load plugin if we have plugin bytes
        if program.plugin.is_empty() {
            log::warn!("No plugin bytes available - cannot execute plugin");
        } else {
            log::info!(
                "Executing Interface Plugin in-process for shot {}",
                self.shot_count
            );

            // Load and execute the Interface Plugin in-process
            self.execute_interface_plugin_in_process(program)?;

            log::info!(
                "Interface Plugin execution completed for shot {}",
                self.shot_count
            );
        }
        Ok(())
    }

    /// Load and execute Interface Plugin in-process (no subprocess)
    fn execute_interface_plugin_in_process(
        &mut self,
        program: &SeleneInterfaceProgram,
    ) -> Result<(), PecosError> {
        use libloading::{Library, Symbol};
        use std::sync::{Arc, Mutex};

        log::info!(
            "Loading Interface Plugin ({} bytes) in-process",
            program.plugin.len()
        );

        // Initialize the FFI interface so plugin calls create ByteMessages
        initialize_ffi_interface(Arc::new(Mutex::new(self.clone())));

        // Initialize Selene runtime if not already done
        if self.selene_runtime.is_none() {
            log::info!(
                "Initializing Selene runtime with {} qubits",
                self.config.num_qubits
            );
            let runtime = SeleneRuntime::new(self.config.num_qubits, 1) // 1 shot per call
                .map_err(|e| {
                    SeleneError::RuntimeError(format!("Failed to initialize Selene: {e}"))
                })?;
            self.selene_runtime = Some(runtime);
        }

        // Get the runtime and set it as current for this thread
        let runtime = self.selene_runtime.as_mut().unwrap();
        set_current_instance(runtime.instance_ptr());

        // Write plugin bytes to a temporary .o file and convert to .so
        let temp_dir = tempfile::tempdir()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to create temp dir: {e}")))?;
        let temp_o_path = temp_dir.path().join("plugin.o");
        let temp_so_path = temp_dir.path().join("plugin.so");

        // Write the plugin bytes
        std::fs::write(&temp_o_path, &program.plugin)
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to write plugin: {e}")))?;

        // Convert .o to .so using gcc
        let output = std::process::Command::new("gcc")
            .args(["-shared", "-o"])
            .arg(&temp_so_path)
            .arg(&temp_o_path)
            .output()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to run gcc: {e}")))?;

        if !output.status.success() {
            return Err(SeleneError::RuntimeError(format!(
                "gcc failed to convert .o to .so: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
            .into());
        }

        // Load the shared library
        let library = unsafe {
            Library::new(&temp_so_path).map_err(|e| {
                SeleneError::RuntimeError(format!("Failed to load plugin library: {e}"))
            })?
        };

        // Results are now handled by the LLVM runtime registry

        // Get the qmain function
        let qmain_symbol: Symbol<unsafe extern "C" fn(u64) -> u64> = unsafe {
            library
                .get(b"qmain")
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to find qmain: {e}")))?
        };

        // Start the shot in Selene runtime
        if let Some(runtime) = &mut self.selene_runtime {
            runtime
                .start_shot(self.shot_count)
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to start shot: {e}")))?;
        }

        log::info!("Calling Interface Plugin qmain(0)");
        log::debug!("About to call Interface Plugin qmain(0)");

        // Call qmain - this will execute the quantum program and call our bridge simulator
        let result = unsafe { qmain_symbol(0) };

        log::info!("Interface Plugin qmain returned: {result}");
        log::debug!("Interface Plugin qmain returned: {result}");

        // End the shot in Selene runtime
        if let Some(runtime) = &mut self.selene_runtime {
            runtime
                .end_shot()
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to end shot: {e}")))?;
        }

        // Clear the instance from thread-local storage
        clear_current_instance();

        // Results are now handled by the LLVM runtime registry
        // Plugin execution stores results via __quantum__rt__result_record_output calls

        log::info!("Interface Plugin executed - results handled by LLVM runtime registry");

        // Keep the library alive and temp dir leaked to avoid cleanup issues
        std::mem::forget(library);
        std::mem::forget(temp_dir);

        Ok(())
    }

    /// Get the next operation from the bridge simulator queue
    fn receive_operations(&mut self) -> Result<ByteMessage, PecosError> {
        // First check if we have queued operations from a previous execution
        if !self.operation_queue.is_empty() {
            log::debug!(
                "Collecting {} operations from queue",
                self.operation_queue.len()
            );
            return Ok(self.operation_queue.remove(0));
        }

        // Try to read new operations from the subprocess via IPC
        if let Some(ref mut instance) = self.selene_instance {
            let messages = instance.try_read_ipc_messages()?;
            for message in messages {
                log::trace!("Queuing ByteMessage from subprocess IPC");
                self.operation_queue.push(message);
            }

            // Return the first message if any were received
            if !self.operation_queue.is_empty() {
                return Ok(self.operation_queue.remove(0));
            }
        }

        // No operations available
        Ok(ByteMessage::create_empty())
    }

    /// Send measurement results to the bridge simulator via IPC
    fn send_measurements(&mut self, message: &ByteMessage) -> Result<(), PecosError> {
        // Extract and store outcomes locally for later retrieval
        let new_outcomes = message
            .outcomes()
            .map_err(|e| PecosError::Processing(format!("Failed to extract outcomes: {e}")))?;

        log::debug!(
            "*** ENGINE: send_measurements storing {} new outcomes starting at index {} ***",
            new_outcomes.len(),
            self.total_measurement_count + 1
        );

        // Store the new outcomes in our accumulated list
        for value in &new_outcomes {
            self.total_measurement_count += 1;
            let result_key = format!("measurement_{}", self.total_measurement_count);
            log::trace!("Storing {result_key} = {value}");
            self.measurement_results
                .insert(result_key, Data::U32(*value));
        }

        log::debug!(
            "*** ENGINE: Total measurements stored so far: {} ***",
            self.total_measurement_count
        );

        // Build a message with ALL accumulated measurements (not just the new ones)
        // The Bridge expects all measurements up to this point
        let mut all_outcomes: Vec<usize> = Vec::new();
        for i in 1..=self.total_measurement_count {
            let key = format!("measurement_{i}");
            if let Some(Data::U32(value)) = self.measurement_results.get(&key) {
                all_outcomes
                    .push(usize::try_from(*value).expect("Measurement value should fit in usize"));
            }
        }

        log::debug!(
            "*** ENGINE: Sending ALL {} accumulated outcomes to Bridge ***",
            all_outcomes.len()
        );

        // Create a new message with all accumulated outcomes
        let mut builder = ByteMessage::builder();
        let _ = builder.for_outcomes();
        builder.add_outcomes(&all_outcomes);
        let accumulated_message = builder.build();

        // Send the accumulated measurement results to the Bridge simulator subprocess via IPC
        if let Some(ref mut instance) = self.selene_instance {
            log::debug!("Sending all accumulated measurement results to Bridge simulator via IPC");
            instance.send_ipc_message(&accumulated_message)?;
        } else {
            log::debug!("No subprocess available for sending measurements");
        }

        log::debug!("Sent accumulated measurement results to PecosSeleneBridgeSimulator via IPC");
        Ok(())
    }
}

// Implement Engine trait
impl Engine for SeleneExecutableEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        use pecos_engines::quantum::StateVecEngine;
        use std::io::Write;

        log::debug!("SeleneExecutableEngine.process() called");
        // log::debug!("*** ENGINE: process() START - THIS SHOULD NOT BE CALLED WHEN USING QUANTUM SYSTEM ***");
        // log::debug!("*** ENGINE: The ControlEngine methods (start/continue_processing) should be used instead ***");
        std::io::stderr().flush().unwrap();

        // Build the Selene instance (direct approach - no subprocess)
        self.build_selene_instance()?;

        // LLVM programs are not supported directly - must use HUGR
        if self.llvm_program.is_some() {
            return Err(PecosError::Processing(
                "Direct LLVM execution not supported. Please compile from HUGR using Selene."
                    .to_string(),
            ));
        }

        // Execute the Selene instance directly (no subprocess management)
        self.execute_selene_shot()?;

        // In IPC mode, we need to initiate the communication
        // Send an empty "start" message to the Bridge to begin execution
        if let Some(ref mut instance) = self.selene_instance {
            // log::debug!("*** ENGINE: Sending initial start message to Bridge via IPC ***");
            // Send an empty quantum operations message to signal the Bridge to start
            let start_message = ByteMessage::builder().for_quantum_operations().build();
            instance.send_ipc_message(&start_message)?;
            // log::debug!("*** ENGINE: Start message sent, waiting for operations from Bridge ***");
        }

        // Create quantum simulator for executing operations
        log::debug!(
            "*** ENGINE: Creating StateVecEngine quantum simulator for {} qubits ***",
            self.config.num_qubits
        );
        self.quantum_sim = Some(Box::new(StateVecEngine::new(self.config.num_qubits)));

        // Process operations from the bridge until complete
        let mut iteration = 0;
        loop {
            let commands = self.receive_operations()?;
            if commands.is_empty()? {
                log::debug!("No more operations from Bridge, completing");
                break;
            }

            iteration += 1;
            log::debug!("Iteration {iteration}: Received operations from Bridge");

            // Execute operations on the quantum simulator (maintains state across iterations)
            let sim = self.quantum_sim.as_mut().ok_or_else(|| {
                PecosError::Processing("Quantum simulator not initialized".to_string())
            })?;

            log::debug!("Executing quantum operations on simulator");
            let measurements = sim.process(commands)?;

            let num_outcomes = measurements.outcomes().map(|o| o.len()).unwrap_or(0);
            log::debug!("Got {num_outcomes} measurement outcomes from simulator");

            // Send measurements back to Bridge for conditional logic
            self.send_measurements(&measurements)?;
        }

        self.get_results()
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ControlEngine>::reset(self)
    }
}

// Implement ClassicalEngine trait
impl ClassicalEngine for SeleneExecutableEngine {
    fn num_qubits(&self) -> usize {
        self.config.num_qubits
    }

    fn compile(&self) -> Result<(), PecosError> {
        // Check if we have a valid program
        if self.program.is_none() && self.llvm_program.is_none() {
            return Err(PecosError::Processing(
                "No program specified for compilation".to_string(),
            ));
        }

        // For LLVM programs, validate that they're not empty
        if let Some(llvm_program) = &self.llvm_program {
            match &llvm_program.content {
                pecos_programs::LlvmContent::Ir(ir) => {
                    if ir.trim().is_empty() {
                        return Err(PecosError::Processing(
                            "Empty LLVM IR cannot be compiled".to_string(),
                        ));
                    }
                }
                pecos_programs::LlvmContent::Bitcode(bc) => {
                    if bc.is_empty() {
                        return Err(PecosError::Processing(
                            "Empty LLVM bitcode cannot be compiled".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        log::trace!("generate_commands() called");
        log::trace!("generate_commands() - CLASSICAL PATH");

        // First check if we have queued operations from a previous execution
        if !self.operation_queue.is_empty() {
            log::debug!("Returning queued operations from bridge");
            return self.receive_operations();
        }

        // Execute the Interface Plugin if not already executed
        if !self.plugin_executed {
            log::debug!("Executing Interface Plugin in-process");
            // Build and execute the Selene Interface Plugin in-process
            if let Some(program) = self.program.clone() {
                self.execute_interface_plugin_in_process(&program)?;
                self.plugin_executed = true;

                // After execution, check if operations were queued
                if !self.operation_queue.is_empty() {
                    log::debug!("Operations queued by bridge, returning them");
                    return self.receive_operations();
                }
            } else if self.llvm_program.is_some() {
                return Err(PecosError::Processing(
                    "Direct LLVM execution not supported. Please use HUGR compilation with Selene."
                        .to_string(),
                ));
            }
        }

        // Return any queued operations (initially empty since plugin directly calls quantum operations)
        self.receive_operations()
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        // Extract outcomes first before moving the message
        let outcomes = message
            .outcomes()
            .map_err(|e| PecosError::Processing(format!("Failed to extract outcomes: {e}")))?;

        log::debug!(
            "*** ENGINE: handle_measurements received {} outcomes ***",
            outcomes.len()
        );

        // Send measurements to the bridge simulator - this also stores them
        self.send_measurements(&message)?;

        // No need to store again - send_measurements already does this with proper indexing

        log::debug!(
            "*** ENGINE: Total stored measurements: {} ***",
            self.measurement_results.len()
        );

        Ok(())
    }

    fn get_results(&self) -> Result<Shot, PecosError> {
        use pecos_llvm_runtime::runtime::registry::RuntimeRegistry;
        log::debug!(
            "*** ENGINE: get_results() called, have {} stored measurements ***",
            self.measurement_results.len()
        );

        // LLVM programs are not supported
        if self.llvm_program.is_some() {
            return Err(PecosError::Processing(
                "Cannot get results for LLVM program. Use HUGR compilation.".to_string(),
            ));
        }

        // Check if we have measurement results from the Bridge
        if !self.measurement_results.is_empty() {
            log::debug!(
                "*** ENGINE: Returning {} measurement results from Bridge ***",
                self.measurement_results.len()
            );
            let mut shot = Shot::default();

            // Add each measurement result to the shot
            log::debug!(
                "*** ENGINE: measurement_results contains {} entries ***",
                self.measurement_results.len()
            );
            for (key, result) in &self.measurement_results {
                log::debug!("    Adding: {key} = {result:?}");
                shot.data.insert(key.clone(), result.clone());
            }
            log::debug!(
                "*** ENGINE: shot now has {} entries before filtering ***",
                shot.data.len()
            );

            // Don't add a combined measurements array for now - it causes issues with nested vectors
            // The individual measurement_1, measurement_2, etc. are sufficient

            // Check for Data::Vec entries that would cause issues
            let mut filtered_shot = Shot::default();
            for (key, value) in shot.data {
                match &value {
                    Data::Vec(vec_data) => {
                        log::debug!(
                            "*** WARNING: Skipping Data::Vec entry '{}' with {} items ***",
                            key,
                            vec_data.len()
                        );
                        // Data::Vec causes issues with to_dict conversion
                        // For now, we'll flatten single-element vectors
                        if vec_data.len() == 1
                            && let Some(single_value) = vec_data.first()
                        {
                            // Convert single-element vec to its value
                            filtered_shot.data.insert(key, single_value.clone());
                        }
                        // Skip multi-element vectors to avoid nested vector error
                    }
                    _ => {
                        filtered_shot.data.insert(key, value);
                    }
                }
            }

            log::debug!(
                "*** ENGINE: Final shot has {} entries after filtering ***",
                filtered_shot.data.len()
            );
            for (key, value) in &filtered_shot.data {
                match value {
                    Data::Bool(b) => log::trace!("    - {key} = Bool({b})"),
                    Data::U32(u) => log::trace!("    - {key} = U32({u})"),
                    Data::I32(i) => log::debug!("    - {key} = I32({i})"),
                    _ => log::trace!("    - {key} = other type"),
                }
            }

            return Ok(filtered_shot);
        }

        // Otherwise try to get results from the LLVM runtime registry
        // This is for backward compatibility with non-Bridge executions

        let mut final_shot = Shot::default();

        // Try to get results from the current runtime state
        if let Some(shot) = RuntimeRegistry::with_current_runtime(|state| {
            // Finalize the shot to apply all mappings
            state.finalize_shot();
            // Get the finalized shot with named register results
            state.get_last_shot().cloned()
        })
        .flatten()
        {
            log::debug!("SeleneExecutableEngine: Got shot from runtime registry: {shot:?}");
            log::debug!(
                "SELENE ENGINE: Got shot from runtime registry with {} entries",
                shot.data.len()
            );
            log::debug!("Registry shot contents:");
            for (key, value) in &shot.data {
                match value {
                    Data::Vec(vec_data) => {
                        log::trace!("    - {} = Data::Vec with {} items", key, vec_data.len());
                    }
                    Data::Bool(b) => log::trace!("    - {key} = Bool({b})"),
                    Data::U32(u) => log::trace!("    - {key} = U32({u})"),
                    _ => log::trace!("    - {key} = other type"),
                }
            }
            // The registry shot might have Data::Vec entries that cause issues
            // Don't use it directly, only merge in non-Vec entries
            for (key, value) in shot.data {
                if !matches!(value, Data::Vec(_)) {
                    final_shot.data.insert(key, value);
                }
            }
        } else {
            log::warn!("SeleneExecutableEngine: No results available from runtime registry");
            log::debug!("*** SELENE ENGINE: No results available from runtime registry");
        }

        // Also include any measurement results we collected locally from the bridge
        for (name, value) in &self.measurement_results {
            final_shot.data.insert(name.clone(), value.clone());
        }

        Ok(final_shot)
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        log::debug!("Resetting SeleneExecutableEngine for next shot");
        log::debug!(
            "reset() called - clearing {} measurements",
            self.measurement_results.len()
        );

        // Reset shot-specific state
        self.measurement_results.clear();
        self.total_measurement_count = 0; // Reset measurement counter
        self.shot_count += 1;
        self.plugin_executed = false;
        self.operation_queue.clear();

        // Stop the subprocess so it can be restarted for the next shot
        // This is necessary because Selene executable runs once per configuration
        if let Some(ref mut instance) = self.selene_instance {
            log::debug!("Stopping Selene subprocess for reset");
            instance.stop()?;
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SeleneExecutableEngine {
    /// Convert R1XY and RZ gates to Clifford equivalents when possible
    /// This allows programs with Y gate (compiled to R1XY) to run on stabilizer simulators
    fn convert_to_clifford_if_possible(operations: ByteMessage) -> Result<ByteMessage, PecosError> {
        use std::f64::consts::PI;

        // log::debug!("*** CONTROL ENGINE: convert_to_clifford_if_possible called ***");

        // Parse the quantum operations
        let ops = operations.quantum_ops().map_err(|e| {
            // log::debug!("*** CONTROL ENGINE: Failed to parse operations: {} ***", e);
            PecosError::Processing(format!("Failed to parse operations: {e}"))
        })?;

        // log::debug!("*** CONTROL ENGINE: Parsed {} operations ***", ops.len());

        // Check if any operations need conversion
        let mut needs_conversion = false;
        for op in &ops {
            if matches!(op.gate_type, GateType::R1XY | GateType::RZ) {
                needs_conversion = true;
                break;
            }
        }

        if !needs_conversion {
            // No conversion needed, return original message
            return Ok(operations);
        }

        // log::debug!("*** CONTROL ENGINE: Converting rotation gates to Clifford equivalents where possible ***");

        // Build new operations with conversions
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        for op in ops {
            // Convert QubitId to usize
            let qubits: Vec<usize> = op.qubits.iter().map(|q| q.0).collect();

            match op.gate_type {
                GateType::R1XY => {
                    // R1XY(theta, phi) gate
                    // For Y gate: theta = π, phi = π/2
                    if op.params.len() >= 2 {
                        let theta = op.params[0];
                        let phi = op.params[1];

                        // Check if this is a Y gate (theta ≈ π, phi ≈ π/2)
                        let is_y_gate =
                            (theta - PI).abs() < 1e-10 && (phi - PI / 2.0).abs() < 1e-10;

                        if is_y_gate {
                            // log::debug!("*** CONTROL ENGINE: Converting R1XY to Y gate ***");
                            // Add Y gate instead
                            builder.add_y(&qubits);
                        } else if (theta - PI).abs() < 1e-10 && phi.abs() < 1e-10 {
                            // X gate: R1XY(π, 0)
                            // log::debug!("*** CONTROL ENGINE: Converting R1XY to X gate ***");
                            builder.add_x(&qubits);
                        } else if theta.abs() < 1e-10 {
                            // Identity: R1XY(0, _)
                            // log::debug!("*** CONTROL ENGINE: Skipping R1XY as identity ***");
                            // Skip identity operations
                        } else {
                            // Can't convert to Clifford, keep original
                            // log::debug!("*** CONTROL ENGINE: Keeping R1XY as-is: theta={:.6}, phi={:.6} ***", theta, phi);
                            builder.add_r1xy(theta, phi, &qubits);
                        }
                    } else {
                        // No parameters, keep original
                        builder.add_r1xy(0.0, 0.0, &qubits);
                    }
                }
                GateType::RZ => {
                    // RZ(theta) gate
                    if op.params.is_empty() {
                        // No parameters, keep original
                        builder.add_rz(0.0, &qubits);
                    } else {
                        let theta = op.params[0];

                        if theta.abs() < 1e-10 {
                            // Identity: RZ(0)
                            log::trace!("CONTROL ENGINE: Skipping RZ(0) as identity");
                            // Skip identity
                        } else if (theta - PI).abs() < 1e-10 {
                            // Z gate: RZ(π)
                            log::trace!("CONTROL ENGINE: Converting RZ(π) to Z gate");
                            builder.add_z(&qubits);
                        } else if (theta - PI / 2.0).abs() < 1e-10 {
                            // S gate: RZ(π/2)
                            log::trace!("CONTROL ENGINE: Converting RZ(π/2) to S gate");
                            builder.add_sz(&qubits);
                        } else if (theta + PI / 2.0).abs() < 1e-10 {
                            // S† gate: RZ(-π/2)
                            log::trace!("CONTROL ENGINE: Converting RZ(-π/2) to S† gate");
                            builder.add_szdg(&qubits);
                        } else {
                            // Can't convert to Clifford, keep original
                            log::trace!("CONTROL ENGINE: Keeping RZ({theta}) as-is");
                            builder.add_rz(theta, &qubits);
                        }
                    }
                }
                GateType::Prep => {
                    builder.add_prep(&qubits);
                }
                GateType::X => {
                    builder.add_x(&qubits);
                }
                GateType::Y => {
                    builder.add_y(&qubits);
                }
                GateType::Z => {
                    builder.add_z(&qubits);
                }
                GateType::H => {
                    builder.add_h(&qubits);
                }
                GateType::SZ => {
                    builder.add_sz(&qubits);
                }
                GateType::SZdg => {
                    builder.add_szdg(&qubits);
                }
                GateType::CX => {
                    // CX needs controls and targets separated
                    if qubits.len() == 2 {
                        builder.add_cx(&[qubits[0]], &[qubits[1]]);
                    }
                }
                GateType::Measure => {
                    builder.add_measurements(&qubits);
                }
                _ => {
                    // Keep other gates as-is
                    // log::debug!("*** CONTROL ENGINE: Keeping gate {:?} as-is ***", op.gate_type);
                    // We can't directly add arbitrary gates, so we'll need to handle this differently
                    // For now, return the original message if we encounter unsupported gates
                    return Ok(operations);
                }
            }
        }

        Ok(builder.build())
    }
}

// Implement ControlEngine trait
impl ControlEngine for SeleneExecutableEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        log::debug!("CONTROL ENGINE: start() called - PROPER INTEGRATION WITH QUANTUM SYSTEM");
        log::info!("SeleneExecutableEngine: start() called - implementing back-and-forth IPC");
        log::info!("Starting back-and-forth communication with Bridge plugin");

        // Set control engine mode flag
        self.control_engine_mode = true;

        // Reset state for new shot
        self.operation_queue.clear();
        self.measurement_results.clear();
        self.total_measurement_count = 0; // Reset measurement counter

        // Build the Selene instance (creates Bridge subprocess with IPC pipes)
        self.build_selene_instance()?;

        // Start the Bridge plugin execution by running the Selene instance
        // This will execute the quantum program which calls Bridge methods
        self.execute_selene_shot()?;

        // The quantum program should now execute between shot_start() and shot_end()
        // The Bridge will buffer operations and send them at shot_end()

        // Send initial message to trigger Bridge
        if let Some(ref mut instance) = self.selene_instance {
            // log::debug!("*** CONTROL ENGINE: Sending initial trigger to Bridge ***");
            let start_message = ByteMessage::builder().for_quantum_operations().build();
            instance.send_ipc_message(&start_message)?;

            // Wait for operations from Bridge
            // log::debug!("*** CONTROL ENGINE: Waiting for operations from Bridge ***");
            let messages = instance.try_read_ipc_messages()?;
            // log::debug!("*** CONTROL ENGINE: try_read_ipc_messages returned {} messages ***", messages.len());

            if !messages.is_empty() {
                // log::debug!("*** CONTROL ENGINE: Messages not empty, processing {} messages ***", messages.len());
                // log::debug!("*** CONTROL ENGINE: Messages vec length before iter: {} ***", messages.len());
                // Store and return the first message
                let mut msg_iter = messages.into_iter();
                // log::debug!("*** CONTROL ENGINE: Created iterator, getting first message ***");
                if let Some(first) = msg_iter.next() {
                    // log::debug!("*** CONTROL ENGINE: Got first message ***");
                    // Store remaining messages in the queue
                    for msg in msg_iter {
                        self.operation_queue.push(msg);
                    }
                    // log::debug!("*** CONTROL ENGINE: About to call convert_to_clifford_if_possible ***");
                    // Convert rotation gates to Clifford equivalents where possible
                    let converted = Self::convert_to_clifford_if_possible(first)?;
                    // log::debug!("*** CONTROL ENGINE: Conversion completed successfully ***");
                    return Ok(EngineStage::NeedsProcessing(converted));
                }
            }
        }

        log::debug!("Returning empty operations as fallback");

        // Return empty operations as fallback
        let empty_ops = ByteMessage::builder().for_quantum_operations().build();
        Ok(EngineStage::NeedsProcessing(empty_ops))
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // log::debug!("*** CONTROL ENGINE: continue_processing() called with measurements ***");
        log::debug!("continue_processing() called with measurements");

        // Send the real measurement results to Bridge plugin via IPC
        // send_measurements will also store them with proper indexing
        self.send_measurements(&measurements)?;

        // Wait for Bridge plugin to process measurements and send back more operations
        log::debug!("Waiting for Bridge response after sending measurements");
        let next_operations = self.receive_operations()?;

        if next_operations.is_empty()? {
            log::debug!("Bridge sent no more operations - execution complete");
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            log::debug!("Bridge sent more operations - continuing processing");
            // Convert rotation gates to Clifford equivalents where possible
            let converted = Self::convert_to_clifford_if_possible(next_operations)?;
            Ok(EngineStage::NeedsProcessing(converted))
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}

// Implement Clone for thread/worker isolation
impl Clone for SeleneExecutableEngine {
    fn clone(&self) -> Self {
        // Create fully isolated instances for proper thread/worker isolation
        Self {
            config: self.config.clone(),
            program: self.program.clone(),
            llvm_program: self.llvm_program.clone(),
            selene_instance: None,       // Each clone builds its own instance
            selene_runtime: None,        // Each clone gets its own runtime
            operation_queue: Vec::new(), // Each clone gets its own queue
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            _message_builder: ByteMessageBuilder::new(),
            plugin_executed: false,
            control_engine_mode: false,
            quantum_sim: None, // Start in standalone mode by default
            total_measurement_count: 0,
        }
    }
}

// Implement the FFIEngineInterface to handle operations from FFI functions
impl FFIEngineInterface for SeleneExecutableEngine {
    fn queue_operation(&mut self, message: ByteMessage) {
        self.operation_queue.push(message);
    }

    fn get_measurement(&mut self, _qubit: usize) -> bool {
        // For now, return false - in production, this would get actual results
        // from the quantum engine
        false
    }
}

// Implement the EngineInterface trait to handle callbacks from the bridge simulator
impl EngineInterface for SeleneExecutableEngine {
    fn send_operation(&mut self, message: ByteMessage) -> Result<(), anyhow::Error> {
        log::debug!("Bridge simulator sending operation to engine");
        self.operation_queue.push(message);
        Ok(())
    }

    fn receive_measurements(&mut self) -> Result<ByteMessage, anyhow::Error> {
        log::debug!("Bridge simulator requesting measurements from engine");

        // Convert stored measurement results back to ByteMessage
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_outcomes();

        // Extract measurement values in order
        let mut outcomes = Vec::new();
        for i in 0..self.measurement_results.len() {
            let key = format!("measurement_{}", i + 1); // Use 1-based indexing
            if let Some(Data::U32(value)) = self.measurement_results.get(&key) {
                outcomes
                    .push(usize::try_from(*value).expect("Measurement value should fit in usize"));
            }
        }

        builder.add_outcomes(&outcomes);

        Ok(builder.build())
    }

    fn get_named_results(&mut self) -> Result<BTreeMap<String, bool>, anyhow::Error> {
        // Results are now handled by the LLVM runtime registry
        // Return empty map since results are accessed via get_results() from runtime
        Ok(BTreeMap::new())
    }
}

// Implement Send and Sync for threading
unsafe impl Send for SeleneExecutableEngine {}
unsafe impl Sync for SeleneExecutableEngine {}
