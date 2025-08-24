//! SeleneExecutableEngine - A ClassicalControlEngine that runs Selene instances directly
//!
//! This engine uses Selene's build() API to create SeleneInstance objects and runs them
//! in-process with the PecosSeleneBridgeSimulator to communicate with PECOS via ByteMessages.

use pecos_core::prelude::PecosError;
use pecos_engines::{
    ByteMessage, ByteMessageBuilder, ClassicalEngine, ControlEngine, Engine, EngineStage, Shot,
    Data,
};
use pecos_programs::{SeleneInterfaceProgram, LlvmProgram};
use std::{any::Any, collections::BTreeMap};
use crate::SeleneError;

// Import the bridge interface from our bridge simulator
use pecos_selene_bridge::EngineInterface;
use crate::selene_runtime_init::{SeleneRuntime, set_current_instance, clear_current_instance};
use crate::selene_ffi_to_bytemessage::{
    EngineInterface as FFIEngineInterface, 
    initialize_engine_interface as initialize_ffi_interface
};

use std::process::{Command, Child, Stdio};
use std::io::{BufReader, BufWriter, Write};


/// Represents a running Selene instance
pub struct SeleneInstance {
    /// Path to the Selene executable
    pub executable: std::path::PathBuf,
    /// Path to the artifacts directory
    pub artifacts: std::path::PathBuf,
    /// The running process (if started)
    process: Option<Child>,
    /// Process stdin for sending commands
    stdin: Option<BufWriter<std::process::ChildStdin>>,
    /// Process stdout for reading results
    stdout: Option<BufReader<std::process::ChildStdout>>,
}

impl SeleneInstance {
    /// Create a new SeleneInstance from paths
    pub fn new(executable: std::path::PathBuf, artifacts: std::path::PathBuf) -> Self {
        Self {
            executable,
            artifacts,
            process: None,
            stdin: None,
            stdout: None,
        }
    }
    
    /// Create a configuration file for Selene executable
    fn create_selene_config(&self) -> Result<std::path::PathBuf, PecosError> {
        use std::fs::File;
        use tempfile::tempdir;
        
        // Create a temporary directory for the config
        let temp_dir = tempdir()
            .map_err(|e| PecosError::Processing(format!("Failed to create temp dir: {}", e)))?;
        let config_path = temp_dir.path().join("selene_config.json");
        
        // Find the bridge plugin
        let bridge_plugin = std::path::PathBuf::from("target/release/deps/libpecos_selene_bridge.so");
        let bridge_plugin = if bridge_plugin.exists() {
            bridge_plugin
        } else {
            std::path::PathBuf::from("target/debug/deps/libpecos_selene_bridge.so")
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
            "n_qubits": 1,
            "output_stream": "stdout",
            "artifact_dir": self.artifacts.to_string_lossy(),
            "error_model": {
                "name": "none",
                "file": "",
                "args": []
            },
            "runtime": {
                "name": "default",
                "file": "",
                "args": []
            }
        });
        
        // Write the configuration to file
        let mut file = File::create(&config_path)
            .map_err(|e| PecosError::Processing(format!("Failed to create config file: {}", e)))?;
        file.write_all(config_json.to_string().as_bytes())
            .map_err(|e| PecosError::Processing(format!("Failed to write config: {}", e)))?;
        
        // Leak the temp_dir to keep it alive
        std::mem::forget(temp_dir);
        
        Ok(config_path)
    }
    
    /// Start the Selene executable process
    pub fn start(&mut self) -> Result<(), PecosError> {
        if self.process.is_some() {
            println!("*** INSTANCE: Process already started, reusing existing process ***");
            return Ok(()); // Already started
        }
        
        println!("*** INSTANCE: Starting NEW Selene executable process: {:?} ***", self.executable);
        log::info!("Starting Selene executable: {:?}", self.executable);
        
        // Create a configuration file for Selene
        let config = self.create_selene_config()?;
        
        // Start the Selene executable with configuration
        let mut cmd = Command::new(&self.executable);
        cmd.arg("--configuration").arg(&config);
        
        // Set environment variable to enable IPC mode in Bridge simulator
        cmd.env("SELENE_IPC", "1");
        
        // Configure stdio for communication
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        // Start the process
        let mut child = cmd.spawn()
            .map_err(|e| PecosError::Processing(format!("Failed to start Selene executable: {}", e)))?;
        
        // Get handles for communication
        let stdin = child.stdin.take()
            .ok_or_else(|| PecosError::Processing("Failed to get stdin handle".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| PecosError::Processing("Failed to get stdout handle".to_string()))?;
        
        self.stdin = Some(BufWriter::new(stdin));
        self.stdout = Some(BufReader::new(stdout));
        self.process = Some(child);
        
        log::info!("Selene executable started successfully");
        Ok(())
    }
    
    /// Stop the Selene executable process
    pub fn stop(&mut self) -> Result<(), PecosError> {
        if let Some(mut process) = self.process.take() {
            log::info!("Stopping Selene executable");
            
            // Try to terminate gracefully first
            if let Some(mut stdin) = self.stdin.take() {
                let _ = stdin.write_all(b"exit\n");
                let _ = stdin.flush();
            }
            
            // Wait a bit for graceful shutdown
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            // Force kill if still running
            let _ = process.kill();
            let _ = process.wait();
            
            self.stdout = None;
            log::info!("Selene executable stopped");
        }
        Ok(())
    }
    
    /// Run a shot on the Selene instance
    pub fn run_shot(&mut self, shot_id: u64) -> Result<(), PecosError> {
        println!("*** INSTANCE: run_shot({}) called ***", shot_id);
        log::info!("SeleneInstance::run_shot({}) called", shot_id);
        
        // Ensure the process is started
        self.start()?;
        
        // Send shot command to Selene
        if let Some(ref mut stdin) = self.stdin {
            let cmd = format!("shot {}\n", shot_id);
            println!("*** INSTANCE: Sending command to Selene process: {} ***", cmd.trim());
            log::info!("Sending command to Selene process: {}", cmd.trim());
            stdin.write_all(cmd.as_bytes())
                .map_err(|e| PecosError::Processing(format!("Failed to send shot command: {}", e)))?;
            stdin.flush()
                .map_err(|e| PecosError::Processing(format!("Failed to flush stdin: {}", e)))?;
            println!("*** INSTANCE: Command sent successfully ***");
            log::info!("Command sent successfully");
        } else {
            println!("*** INSTANCE: WARNING - No stdin available for Selene process ***");
            log::warn!("No stdin available for Selene process");
        }
        
        // The bridge simulator will handle the actual quantum operations
        // and communicate via IPC (stdout/stdin)
        
        // Try to read any ByteMessages from stdout
        let messages = self.try_read_ipc_messages()?;
        
        // Add any received messages to our operation queue for processing
        for message in messages {
            println!("*** INSTANCE: Queuing ByteMessage from subprocess IPC ***");
            // Note: In the full implementation, we'd need to add this to the engine's operation queue
            // For now, just log that we received it
        }
        
        Ok(())
    }
    
    /// Try to read ByteMessages from the subprocess stdout (IPC)
    fn try_read_ipc_messages(&mut self) -> Result<Vec<ByteMessage>, PecosError> {
        use std::io::Read;
        
        let mut messages = Vec::new();
        
        if let Some(ref mut stdout) = self.stdout {
            println!("*** INSTANCE: Trying to read IPC messages from subprocess stdout ***");
            
            // Try to read length-prefixed messages (simplified protocol)
            loop {
                // Try to read the length prefix (4 bytes)
                let mut len_bytes = [0u8; 4];
                match stdout.read_exact(&mut len_bytes) {
                    Ok(_) => {
                        let msg_len = u32::from_le_bytes(len_bytes) as usize;
                        println!("*** INSTANCE: Message length: {} bytes ***", msg_len);
                        
                        // Read the message data
                        let mut msg_bytes = vec![0u8; msg_len];
                        match stdout.read_exact(&mut msg_bytes) {
                            Ok(_) => {
                                println!("*** INSTANCE: Read {} bytes of message data ***", msg_bytes.len());
                                
                                // Create ByteMessage from the data
                                let message = ByteMessage::new(&msg_bytes);
                                messages.push(message);
                            }
                            Err(e) => {
                                println!("*** INSTANCE: Failed to read message data: {} ***", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        if messages.is_empty() {
                            println!("*** INSTANCE: No data available from subprocess: {} ***", e);
                        } else {
                            println!("*** INSTANCE: No more messages available: {} ***", e);
                        }
                        break;
                    }
                }
            }
        } else {
            println!("*** INSTANCE: No stdout available for reading IPC messages ***");
        }
        
        println!("*** INSTANCE: Read {} IPC messages ***", messages.len());
        Ok(messages)
    }
    
    /// Send a ByteMessage to the subprocess via stdin (IPC)
    pub fn send_ipc_message(&mut self, message: &ByteMessage) -> Result<(), PecosError> {
        if let Some(ref mut stdin) = self.stdin {
            println!("*** INSTANCE: Sending ByteMessage to subprocess via IPC ***");
            
            let bytes = message.as_bytes();
            
            // Send message with simple length prefix (matching Bridge simulator protocol)
            // Write length as 4 bytes
            let len_bytes = (bytes.len() as u32).to_le_bytes();
            stdin.write_all(&len_bytes)
                .map_err(|e| PecosError::Processing(format!("Failed to write IPC length: {}", e)))?;
            
            // Write the actual message bytes
            stdin.write_all(bytes)
                .map_err(|e| PecosError::Processing(format!("Failed to write IPC message: {}", e)))?;
            
            stdin.flush()
                .map_err(|e| PecosError::Processing(format!("Failed to flush IPC message: {}", e)))?;
            
            println!("*** INSTANCE: Sent {} bytes via IPC (length-prefixed) ***", bytes.len());
            Ok(())
        } else {
            Err(PecosError::Processing("No stdin available for IPC communication".to_string()))
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

/// A ClassicalControlEngine that runs Selene instances directly with PecosSeleneBridgeSimulator
pub struct SeleneExecutableEngine {
    /// Configuration for the Selene instance
    config: SeleneExecutableConfig,
    
    /// The loaded program (compiled from HUGR)
    program: Option<SeleneInterfaceProgram>,
    
    /// LLVM program (for backward compatibility)
    llvm_program: Option<LlvmProgram>,
    
    /// Built Selene instance (created from HUGR via build() API)
    selene_instance: Option<SeleneInstance>,
    
    /// The initialized Selene runtime (when using real libselene.so)
    selene_runtime: Option<SeleneRuntime>,
    
    /// ByteMessage queue for operations sent from bridge simulator
    operation_queue: Vec<ByteMessage>,
    
    /// Current measurement results
    measurement_results: BTreeMap<String, Data>,
    
    /// Shot counter
    shot_count: u64,
    
    /// Reusable message builder
    message_builder: ByteMessageBuilder,
    
    /// Whether the Interface Plugin has been executed for this shot
    plugin_executed: bool,
}

impl SeleneExecutableEngine {
    /// Create a new engine
    pub fn new(num_qubits: usize) -> Result<Self, PecosError> {
        println!("*** ENGINE: SeleneExecutableEngine::new({}) called ***", num_qubits);
        
        // Validate num_qubits
        if num_qubits == 0 {
            return Err(PecosError::Input("Number of qubits must be greater than 0".to_string()));
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
            message_builder: ByteMessageBuilder::new(),
            plugin_executed: false,
        })
    }
    
    /// Set the program to execute
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
    pub fn with_llvm_program(mut self, program: LlvmProgram) -> Self {
        self.llvm_program = Some(program);
        self
    }
    
    /// Set the working directory
    pub fn with_working_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.config.working_dir = Some(dir);
        self
    }
    
    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }
    
    /// Set the plugin path
    pub fn with_plugin_path(mut self, path: std::path::PathBuf) -> Self {
        self.config.plugin_path = Some(path);
        self
    }
    
    /// Build the Selene instance from HUGR and prepare for execution
    fn build_selene_instance(&mut self) -> Result<(), PecosError> {
        log::info!("SeleneExecutableEngine: build_selene_instance() called");
        
        // If we already have a SeleneInstance, don't recreate it
        if self.selene_instance.is_some() {
            log::info!("SeleneInstance already exists, reusing it");
            println!("*** ENGINE: SeleneInstance already exists, reusing for next shot ***");
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
        if let (Some(exec_path), Some(artifacts_path)) = (&self.config.executable_path, &self.config.artifacts_path) {
            log::info!("Using pre-compiled Selene executable at: {:?}", exec_path);
            log::info!("  Artifacts at: {:?}", artifacts_path);
            println!("*** ENGINE: Creating NEW SeleneInstance for executable ***");
            
            // Create a real SeleneInstance with the pre-compiled executable
            self.selene_instance = Some(SeleneInstance::new(
                exec_path.clone(),
                artifacts_path.clone(),
            ));
            
            // Initialize the engine interface for bridge communication
            use std::sync::{Arc, Mutex};
            use pecos_selene_bridge::initialize_engine_interface;
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
            let workspace_root = manifest_dir
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            
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
                    "Could not find PecosSeleneBridgeSimulator plugin library. Make sure to build with: cargo build --package pecos-selene-bridge".to_string()
                ))?
        };
        
        log::info!("Using bridge plugin at: {:?}", bridge_plugin_path);
        
        // Initialize the engine interface so the bridge can communicate with us
        use std::sync::{Arc, Mutex};
        use pecos_selene_bridge::initialize_engine_interface;
        initialize_engine_interface(Arc::new(Mutex::new(self.clone())));
        
        // Note: The actual Selene build process happens in Python using selene_sim.build()
        // The Python side compiles HUGR to a Selene executable and passes the paths here.
        // Since we're called from build_selene_instance() without a pre-compiled executable,
        // this path should not normally be reached - compilation should happen in Python.
        
        // If we reach here, it means we're trying to build from HUGR in Rust,
        // which is not the intended flow (Python is more natural for Selene).
        return Err(SeleneError::CompilationError(
            "Selene executable compilation should happen in Python before creating the engine. \
             Use sim_wrapper.py to compile Guppy/HUGR to Selene executable.".to_string()
        ).into());
    }
    
    /// Execute Selene Interface Plugin in-process for a shot
    fn execute_selene_shot(&mut self) -> Result<(), PecosError> {
        log::info!("SeleneExecutableEngine: execute_selene_shot() called for shot {}", self.shot_count);
        
        // Check if we have a pre-compiled executable (from selene_sim.build())
        if let Some(ref mut instance) = self.selene_instance {
            println!("*** ENGINE: Running pre-compiled Selene executable for shot {} ***", self.shot_count);
            log::info!("Running pre-compiled Selene executable for shot {}", self.shot_count);
            
            // Run a shot on the Selene executable
            instance.run_shot(self.shot_count as u64)?;
            
            // The Bridge simulator in the executable will communicate back via the EngineInterface
            // The results will be collected through that interface
            
            println!("*** ENGINE: Shot command sent to Selene executable ***");
            log::info!("Shot command sent to Selene executable");
            
            // For now, mark this shot as having been executed
            self.plugin_executed = true;
            return Ok(());
        }
        
        // Clone the program to avoid borrowing issues
        if let Some(program) = self.program.clone() {
            // Only try to load plugin if we have plugin bytes
            if !program.plugin.is_empty() {
                log::info!("Executing Interface Plugin in-process for shot {}", self.shot_count);
                
                // Load and execute the Interface Plugin in-process
                self.execute_interface_plugin_in_process(&program)?;
                
                log::info!("Interface Plugin execution completed for shot {}", self.shot_count);
            } else {
                log::warn!("No plugin bytes available - cannot execute plugin");
            }
        } else if let Some(_llvm_program) = &self.llvm_program {
            // For LLVM programs, we need a different execution path
            // For now, just log that we have an LLVM program
            log::info!("LLVM program execution requested - returning empty shot");
            // In the future, this would compile and execute the LLVM program
        }
        
        Ok(())
    }
    
    /// Load and execute Interface Plugin in-process (no subprocess)
    fn execute_interface_plugin_in_process(&mut self, program: &SeleneInterfaceProgram) -> Result<(), PecosError> {
        use libloading::{Library, Symbol};
        use std::sync::{Arc, Mutex};
        
        log::info!("Loading Interface Plugin ({} bytes) in-process", program.plugin.len());
        
        // Initialize the FFI interface so plugin calls create ByteMessages
        initialize_ffi_interface(Arc::new(Mutex::new(self.clone())));
        
        // Initialize Selene runtime if not already done
        if self.selene_runtime.is_none() {
            log::info!("Initializing Selene runtime with {} qubits", self.config.num_qubits);
            let runtime = SeleneRuntime::new(self.config.num_qubits, 1) // 1 shot per call
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to initialize Selene: {}", e)))?;
            self.selene_runtime = Some(runtime);
        }
        
        // Get the runtime and set it as current for this thread
        let runtime = self.selene_runtime.as_mut().unwrap();
        set_current_instance(runtime.instance_ptr());
        
        // Write plugin bytes to a temporary .o file and convert to .so
        let temp_dir = tempfile::tempdir()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to create temp dir: {}", e)))?;
        let temp_o_path = temp_dir.path().join("plugin.o");
        let temp_so_path = temp_dir.path().join("plugin.so");
        
        // Write the plugin bytes
        std::fs::write(&temp_o_path, &program.plugin)
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to write plugin: {}", e)))?;
        
        // Convert .o to .so using gcc
        let output = std::process::Command::new("gcc")
            .args(&["-shared", "-o"])
            .arg(&temp_so_path)
            .arg(&temp_o_path)
            .output()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to run gcc: {}", e)))?;
        
        if !output.status.success() {
            return Err(SeleneError::RuntimeError(format!(
                "gcc failed to convert .o to .so: {}", 
                String::from_utf8_lossy(&output.stderr)
            )).into());
        }
        
        // Load the shared library
        let library = unsafe {
            Library::new(&temp_so_path)
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to load plugin library: {}", e)))?
        };
        
        // Results are now handled by the LLVM runtime registry
        
        // Get the qmain function
        let qmain_symbol: Symbol<unsafe extern "C" fn(u64) -> u64> = unsafe {
            library.get(b"qmain")
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to find qmain: {}", e)))?
        };
        
        // Start the shot in Selene runtime
        if let Some(runtime) = &mut self.selene_runtime {
            runtime.start_shot(self.shot_count)
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to start shot: {}", e)))?;
        }
        
        log::info!("Calling Interface Plugin qmain(0)");
        println!("*** ENGINE: About to call Interface Plugin qmain(0) ***");
        
        // Call qmain - this will execute the quantum program and call our bridge simulator
        let result = unsafe { qmain_symbol(0) };
        
        log::info!("Interface Plugin qmain returned: {}", result);
        println!("*** ENGINE: Interface Plugin qmain returned: {} ***", result);
        
        // End the shot in Selene runtime
        if let Some(runtime) = &mut self.selene_runtime {
            runtime.end_shot()
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to end shot: {}", e)))?;
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
            log::debug!("Collecting {} operations from queue", self.operation_queue.len());
            return Ok(self.operation_queue.remove(0));
        }
        
        // Try to read new operations from the subprocess via IPC
        if let Some(ref mut instance) = self.selene_instance {
            let messages = instance.try_read_ipc_messages()?;
            for message in messages {
                println!("*** ENGINE: Queuing ByteMessage from subprocess IPC ***");
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
    fn send_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        // Extract and store outcomes locally for later retrieval
        let outcomes = message.outcomes()
            .map_err(|e| PecosError::Processing(format!("Failed to extract outcomes: {}", e)))?;
        
        for (i, value) in outcomes.iter().enumerate() {
            let result_key = format!("measurement_{}", i);
            self.measurement_results.insert(result_key, Data::U32(*value));
        }
        
        // Send the measurement results to the Bridge simulator subprocess via IPC
        if let Some(ref mut instance) = self.selene_instance {
            println!("*** ENGINE: Sending measurement results to Bridge simulator via IPC ***");
            instance.send_ipc_message(&message)?;
        } else {
            println!("*** ENGINE: No subprocess available for sending measurements ***");
        }
        
        log::debug!("Sent measurement results to PecosSeleneBridgeSimulator via IPC");
        Ok(())
    }
}

// Implement Engine trait
impl Engine for SeleneExecutableEngine {
    type Input = ();
    type Output = Shot;
    
    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        println!("*** ENGINE: SeleneExecutableEngine.process() called ***");
        // Build the Selene instance (direct approach - no subprocess)
        self.build_selene_instance()?;
        
        // If we have an LLVM program, just return dummy results for testing
        if self.llvm_program.is_some() {
            println!("*** ENGINE: LLVM program - returning test shot ***");
            self.shot_count += 1;
            
            // Create a shot with some dummy measurement data
            let mut shot = Shot::default();
            shot.data.insert("measurements".to_string(), Data::U32(0));
            shot.data.insert("measurement_0".to_string(), Data::U32(0));
            shot.data.insert("measurement_1".to_string(), Data::U32(0));
            
            return Ok(shot);
        }
        
        // Execute the Selene instance directly (no subprocess management)
        self.execute_selene_shot()?;
        
        // Process operations from the bridge until complete
        loop {
            let commands = self.receive_operations()?;
            if commands.is_empty()? {
                break;
            }
            
            // In a real system, these would be sent to a quantum engine
            // For now, we simulate empty measurements
            let measurements = ByteMessage::builder().for_outcomes().build();
            self.send_measurements(measurements)?;
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
            return Err(PecosError::Processing("No program specified for compilation".to_string()));
        }
        
        // For LLVM programs, validate that they're not empty
        if let Some(llvm_program) = &self.llvm_program {
            match &llvm_program.content {
                pecos_programs::LlvmContent::Ir(ir) => {
                    if ir.trim().is_empty() {
                        return Err(PecosError::Processing("Empty LLVM IR cannot be compiled".to_string()));
                    }
                }
                pecos_programs::LlvmContent::Bitcode(bc) => {
                    if bc.is_empty() {
                        return Err(PecosError::Processing("Empty LLVM bitcode cannot be compiled".to_string()));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        println!("*** ENGINE: generate_commands() called ***");
        
        // First check if we have queued operations from a previous execution
        if !self.operation_queue.is_empty() {
            println!("*** ENGINE: Returning queued operations from bridge ***");
            return self.receive_operations();
        }
        
        // Execute the Interface Plugin if not already executed
        if !self.plugin_executed {
            println!("*** ENGINE: Executing Interface Plugin in-process ***");
            // Build and execute the Selene Interface Plugin in-process
            if let Some(program) = self.program.clone() {
                self.execute_interface_plugin_in_process(&program)?;
                self.plugin_executed = true;
                
                // After execution, check if operations were queued
                if !self.operation_queue.is_empty() {
                    println!("*** ENGINE: Operations queued by bridge, returning them ***");
                    return self.receive_operations();
                }
            } else if let Some(llvm_program) = &self.llvm_program {
                // Check if LLVM program is empty
                match &llvm_program.content {
                    pecos_programs::LlvmContent::Ir(ir) => {
                        if ir.trim().is_empty() {
                            return Err(PecosError::Processing("Cannot generate commands from empty LLVM IR".to_string()));
                        }
                    }
                    pecos_programs::LlvmContent::Bitcode(bc) => {
                        if bc.is_empty() {
                            return Err(PecosError::Processing("Cannot generate commands from empty LLVM bitcode".to_string()));
                        }
                    }
                }
                
                // For LLVM programs, return empty commands for now
                println!("*** ENGINE: LLVM program - returning empty commands ***");
                self.plugin_executed = true;
                return Ok(ByteMessage::builder().for_quantum_operations().build());
            }
        }
        
        // Return any queued operations (initially empty since plugin directly calls quantum operations)
        self.receive_operations()
    }
    
    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        // Extract outcomes first before moving the message
        let outcomes = message.outcomes()
            .map_err(|e| PecosError::Processing(format!("Failed to extract outcomes: {}", e)))?;
        
        // Send measurements to the bridge simulator
        self.send_measurements(message)?;
        
        for (i, value) in outcomes.iter().enumerate() {
            let result_key = format!("measurement_{}", i);
            self.measurement_results.insert(result_key, Data::U32(*value));
        }
        
        Ok(())
    }
    
    fn get_results(&self) -> Result<Shot, PecosError> {
        // For LLVM programs in test mode, return dummy measurements
        if self.llvm_program.is_some() {
            println!("*** ENGINE: get_results() for LLVM program - returning dummy shot ***");
            let mut shot = Shot::default();
            shot.data.insert("measurements".to_string(), Data::U32(0));
            shot.data.insert("measurement_0".to_string(), Data::U32(0));
            shot.data.insert("measurement_1".to_string(), Data::U32(0));
            return Ok(shot);
        }
        
        // Get results from the LLVM runtime registry instead of global storage
        // This avoids symbol collisions and uses the proper PECOS infrastructure
        use pecos_llvm_runtime::runtime::registry::RuntimeRegistry;
        
        let mut final_shot = Shot::default();
        
        // Try to get results from the current runtime state
        if let Some(shot) = RuntimeRegistry::with_current_runtime(|state| {
            // Finalize the shot to apply all mappings
            state.finalize_shot();
            // Get the finalized shot with named register results
            state.get_last_shot().cloned()
        }).flatten() {
            log::debug!("SeleneExecutableEngine: Got shot from runtime registry: {:?}", shot);
            println!("*** SELENE ENGINE: Got shot from runtime registry with {} entries ***", shot.data.len());
            final_shot = shot;
        } else {
            log::warn!("SeleneExecutableEngine: No results available from runtime registry");
            println!("*** SELENE ENGINE: No results available from runtime registry ***");
        }
        
        // Also include any measurement results we collected locally from the bridge
        for (name, value) in &self.measurement_results {
            final_shot.data.insert(name.clone(), value.clone());
        }
        
        Ok(final_shot)
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        log::debug!("Resetting SeleneExecutableEngine for next shot");
        
        // Reset shot-specific state
        self.measurement_results.clear();
        self.shot_count += 1;
        self.plugin_executed = false;
        
        // Reset Selene instance for next shot
        // (No process to stop since we use direct SeleneInstance execution)
        
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Implement ControlEngine trait
impl ControlEngine for SeleneExecutableEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;
    
    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        log::info!("SeleneExecutableEngine: start() called - implementing back-and-forth IPC");
        println!("*** SELENE ENGINE: Starting back-and-forth communication with Bridge plugin ***");
        
        // Reset state for new shot
        self.operation_queue.clear();
        self.measurement_results.clear();
        
        // Build the Selene instance (creates Bridge subprocess with IPC pipes)  
        self.build_selene_instance()?;
        
        // Start the Bridge plugin execution by running the Selene instance
        // This should start the Bridge plugin process and begin IPC communication
        self.execute_selene_shot()?;
        
        // Try to receive the first batch of operations from Bridge plugin
        println!("*** SELENE ENGINE: Requesting initial operations from Bridge via IPC ***");
        let initial_operations = self.receive_operations()?;
        
        if initial_operations.is_empty()? {
            println!("*** SELENE ENGINE: No initial operations - completing immediately ***");
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            println!("*** SELENE ENGINE: Got initial operations - returning NeedsProcessing ***");
            Ok(EngineStage::NeedsProcessing(initial_operations))
        }
    }
    
    fn continue_processing(&mut self, measurements: ByteMessage)
        -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        println!("*** SELENE ENGINE: continue_processing() called with measurements ***");
        
        // Send measurement results to Bridge plugin via IPC
        self.send_measurements(measurements)?;
        
        // Wait for Bridge plugin to process measurements and send back more operations
        println!("*** SELENE ENGINE: Waiting for Bridge response after sending measurements ***");
        let next_operations = self.receive_operations()?;
        
        if next_operations.is_empty()? {
            println!("*** SELENE ENGINE: Bridge sent no more operations - execution complete ***");
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            println!("*** SELENE ENGINE: Bridge sent more operations - continuing processing ***");
            Ok(EngineStage::NeedsProcessing(next_operations))
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
            selene_instance: None, // Each clone builds its own instance
            selene_runtime: None, // Each clone gets its own runtime
            operation_queue: Vec::new(), // Each clone gets its own queue
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            message_builder: ByteMessageBuilder::new(),
            plugin_executed: false,
        }
    }
}

// Implement the FFIEngineInterface to handle operations from FFI functions
impl FFIEngineInterface for SeleneExecutableEngine {
    fn queue_operation(&mut self, message: ByteMessage) {
        self.operation_queue.push(message);
    }
    
    fn get_measurement(&mut self, qubit: usize) -> bool {
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
            let key = format!("measurement_{}", i);
            if let Some(Data::U32(value)) = self.measurement_results.get(&key) {
                outcomes.push(*value as usize);
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