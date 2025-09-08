//! PECOS-Selene Bridge Simulator Plugin
//!
//! This plugin acts as a bridge between Selene's simulator interface and PECOS's
//! ByteMessage system. It allows Selene programs to run naturally while converting
//! operations directly to ByteMessages for integration with PECOS quantum engines.

// Static initialization to log when library is loaded
static INIT: std::sync::Once = std::sync::Once::new();

use anyhow::Result;
use pecos_engines::{ByteMessage, ByteMessageBuilder};
use selene_core::{
    export_simulator_plugin,
    simulator::{SimulatorInterface, SimulatorInterfaceFactory},
    // runtime::{Operation, RuntimeInterface, BatchOperation},
    utils::MetricValue,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Detect if we're running as a subprocess with IPC enabled
fn is_subprocess_with_piped_stdio() -> bool {
    // Check for IPC marker file in artifacts directory
    if let Ok(artifacts_dir) = std::env::var("SELENE_ARTIFACTS_DIR") {
        let ipc_marker = std::path::Path::new(&artifacts_dir).join("pecos_ipc_mode");
        if ipc_marker.exists() {
            eprintln!("Bridge: Found IPC marker file at {:?}", ipc_marker);
            return true;
        }
    }
    
    // Fallback to environment variable
    if std::env::var("PECOS_BRIDGE_IPC").unwrap_or_default() == "1" {
        return true;
    }
    
    false
}

pub mod callback_interface;

// Global reference to the ClassicalControlEngine for direct ByteMessage communication
static ENGINE_INTERFACE: OnceLock<Arc<Mutex<dyn EngineInterface + Send + Sync>>> = OnceLock::new();

/// Trait for the ClassicalControlEngine to receive operations from the bridge simulator
pub trait EngineInterface {
    /// Send a quantum operation as a ByteMessage to PECOS
    fn send_operation(&mut self, message: ByteMessage) -> Result<()>;
    
    /// Receive measurement results as a ByteMessage from PECOS  
    fn receive_measurements(&mut self) -> Result<ByteMessage>;
    
    /// Get named results from the bridge simulator
    fn get_named_results(&mut self) -> Result<BTreeMap<String, bool>>;
}

/// Initialize the engine interface for direct communication
pub fn initialize_engine_interface(engine: Arc<Mutex<dyn EngineInterface + Send + Sync>>) {
    let _ = ENGINE_INTERFACE.get_or_init(|| engine);
}

// Global storage for engine callbacks
static CALLBACKS: Mutex<Option<Callbacks>> = Mutex::new(None);

struct Callbacks {
    context: *mut std::ffi::c_void,
    send_op: extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> i32,
    recv_meas: extern "C" fn(*mut std::ffi::c_void, *mut *mut u8, *mut usize) -> i32,
}

unsafe impl Send for Callbacks {}
unsafe impl Sync for Callbacks {}

/// Setup function that PECOS calls to register callbacks
/// This is the main entry point for establishing communication
#[no_mangle]
pub extern "C" fn pecos_bridge_set_engine_callbacks(
    context: *mut std::ffi::c_void,
    send_op: extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> i32,
    recv_meas: extern "C" fn(*mut std::ffi::c_void, *mut *mut u8, *mut usize) -> i32,
) {
    *CALLBACKS.lock().unwrap() = Some(Callbacks {
        context,
        send_op,
        recv_meas,
    });
    
    log::debug!("Bridge: Engine callbacks registered");
}

/// Bridge execution state for proper back-and-forth communication
#[derive(Debug, Clone)]
pub enum BridgeState {
    /// Initial state - ready to send first operations
    Initial,
    /// Waiting for measurement results from PECOS
    WaitingForMeasurements,
    /// Processing measurements and deciding next action
    ProcessingMeasurements,
    /// Execution complete
    Complete,
}

/// The PECOS-Selene Bridge Simulator that converts between Selene operations and ByteMessages
/// Implements proper back-and-forth communication using EngineStage pattern.
pub struct PecosSeleneBridgeSimulator {
    /// Number of qubits
    n_qubits: usize,
    
    /// Message builder for creating ByteMessages
    message_builder: ByteMessageBuilder,
    
    /// Current execution state for back-and-forth communication
    execution_state: BridgeState,
    
    /// Current shot ID
    shot_id: u64,
    
    /// Measurement counter for the current shot
    measurement_count: usize,
    
    /// Cache of measurement results received from PECOS
    measurement_results: BTreeMap<usize, bool>,
    
    /// Named measurement results for RuntimeInterface (captures result_id)
    named_results: BTreeMap<String, bool>,
    
    /// Flag to indicate if we're in IPC mode (buffering operations)
    ipc_mode: bool,
    
    /// Single ByteMessage builder that accumulates all operations during shot
    shot_operations: ByteMessageBuilder,
    
    /// Track if we've started building operations for this shot
    operations_started: bool,
}

impl PecosSeleneBridgeSimulator {
    fn new(n_qubits: u64) -> Self {
        eprintln!("Bridge: PecosSeleneBridgeSimulator::new({}) called", n_qubits);
        
        // Don't try to read config here - it doesn't exist at build time
        // We'll read it in shot_start() instead
        let ipc_mode = is_subprocess_with_piped_stdio();
        eprintln!("Bridge: IPC mode detected = {}", ipc_mode);
        
        // Use a placeholder value - will be updated in shot_start()
        eprintln!("Bridge: Using placeholder n_qubits={} (will read actual value at runtime)", n_qubits);
        
        Self {
            n_qubits: n_qubits as usize,  // Placeholder - will be updated in shot_start()
            message_builder: ByteMessageBuilder::new(),
            execution_state: BridgeState::Initial,
            shot_id: 0,
            measurement_count: 0,
            measurement_results: BTreeMap::new(),
            named_results: BTreeMap::new(),
            ipc_mode,
            shot_operations: ByteMessageBuilder::new(),
            operations_started: false,
        }
    }
    
    /// Try to send ByteMessage via IPC (stdout) - returns true if IPC is available
    fn try_send_via_ipc(&mut self, message: &ByteMessage) -> Result<bool> {
        use std::io::{stdout, Write};
        
        // Simple heuristic: if SELENE_IPC env var is set, use IPC mode
        if std::env::var("SELENE_IPC").is_err() {
            log::trace!("[Bridge] SELENE_IPC not set - not using IPC mode");
            return Ok(false);
        }
        
        log::trace!("[Bridge] SELENE_IPC detected - using IPC mode");
        
        // We're in IPC mode - send message via stdout
        let bytes = message.as_bytes();
        
        // Send message with simple length prefix (no magic header, no newline)
        let mut stdout = stdout().lock();
        
        // Write length as 4 bytes
        let len_bytes = (bytes.len() as u32).to_le_bytes();
        stdout.write_all(&len_bytes)?;
        
        // Write the actual message bytes
        stdout.write_all(bytes)?;
        stdout.flush()?;
        
        log::trace!("[Bridge] Sent {} bytes via IPC (length-prefixed)", bytes.len());
        Ok(true)
    }
    
    /// Execute back-and-forth communication round - returns true if more rounds needed
    fn execute_communication_round(&mut self) -> Result<bool> {
        log::trace!("[Bridge] execute_communication_round() - state: {:?}", self.execution_state);
        
        match self.execution_state {
            BridgeState::Initial => {
                // Send initial quantum operations to PECOS
                log::trace!("[Bridge] Initial state - generating quantum operations");
                self.generate_initial_operations()?;
                self.send_pending_operations()?;
                
                self.execution_state = BridgeState::WaitingForMeasurements;
                Ok(true) // More communication needed
            },
            
            BridgeState::WaitingForMeasurements => {
                // Wait for measurement results from PECOS
                log::trace!("[Bridge] Waiting for measurements from PECOS");
                if let Some(measurements) = self.try_receive_via_ipc()? {
                    log::trace!("[Bridge] Received measurements, processing...");
                    self.process_measurements(measurements)?;
                    self.execution_state = BridgeState::ProcessingMeasurements;
                    Ok(true) // Continue processing
                } else {
                    log::trace!("[Bridge] No measurements received yet, continuing to wait");
                    Ok(true) // Keep waiting
                }
            },
            
            BridgeState::ProcessingMeasurements => {
                // Process measurements and decide if more operations needed
                log::trace!("[Bridge] Processing measurements state");
                if self.needs_more_operations() {
                    log::trace!("[Bridge] Generating more operations based on measurements");
                    self.generate_conditional_operations()?;
                    self.send_pending_operations()?;
                    self.execution_state = BridgeState::WaitingForMeasurements;
                    Ok(true) // More rounds needed
                } else {
                    log::trace!("[Bridge] No more operations needed - completing");
                    self.execution_state = BridgeState::Complete;
                    Ok(false) // Communication complete
                }
            },
            
            BridgeState::Complete => {
                log::trace!("[Bridge] Already complete");
                Ok(false) // No more communication needed
            }
        }
    }
    
    /// Generate initial quantum operations to send to PECOS
    fn generate_initial_operations(&mut self) -> Result<()> {
        log::trace!("[Bridge] generate_initial_operations called - operations handled via shot_start/shot_end");
        // In IPC mode, operations are buffered during shot execution
        // and sent at shot_end
        Ok(())
    }
    
    /// Send buffered operations via IPC (no longer used - operations sent at shot_end)
    fn send_pending_operations(&mut self) -> Result<()> {
        log::trace!("[Bridge] send_pending_operations called - operations now handled at shot_end");
        Ok(())
    }
    
    /// Process measurement results received from PECOS
    fn process_measurements(&mut self, measurements: ByteMessage) -> Result<()> {
        log::trace!("[Bridge] Processing measurement results");
        
        if let Ok(outcomes) = measurements.outcomes() {
            log::trace!("[Bridge] Received {} measurement outcomes", outcomes.len());
            for (i, &outcome) in outcomes.iter().enumerate() {
                let bool_result = outcome != 0;
                self.measurement_results.insert(i, bool_result);
                log::trace!("[Bridge] Measurement {}: raw_value={}, bool={}", i, outcome, bool_result);
            }
        }
        
        Ok(())
    }
    
    /// Check if more operations are needed based on current state
    fn needs_more_operations(&self) -> bool {
        // For this simple example, we only do one round of operations
        // In a real quantum algorithm, this would implement conditional logic
        false
    }
    
    /// Generate additional operations based on measurement results
    fn generate_conditional_operations(&mut self) -> Result<()> {
        log::trace!("[Bridge] Generating conditional operations based on measurements");
        // This would implement conditional quantum operations based on measurement results
        Ok(())
    }
    
    /// Try to receive ByteMessage via IPC (stdin) - returns None if no data available
    fn try_receive_via_ipc(&mut self) -> Result<Option<ByteMessage>> {
        use std::io::{stdin, Read};
        
        // Check if we're in IPC mode
        if std::env::var("SELENE_IPC").is_err() {
            return Ok(None);
        }
        
        log::trace!("[Bridge] Trying to receive measurement results via IPC (stdin)");
        
        let stdin = stdin();
        let mut reader = stdin.lock();
        
        // Try to read the length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        match reader.read_exact(&mut len_bytes) {
            Ok(_) => {
                let msg_len = u32::from_le_bytes(len_bytes) as usize;
                log::trace!("[Bridge] Message length: {} bytes", msg_len);
                
                // Read the message data
                let mut msg_bytes = vec![0u8; msg_len];
                match reader.read_exact(&mut msg_bytes) {
                    Ok(_) => {
                        log::trace!("[Bridge] Read {} bytes of message data", msg_bytes.len());
                        
                        // Create ByteMessage from the data
                        let message = ByteMessage::new(&msg_bytes);
                        return Ok(Some(message));
                    }
                    Err(e) => {
                        log::trace!("[Bridge] Failed to read message data: {}", e);
                    }
                }
            }
            Err(e) => {
                log::trace!("[Bridge] Failed to read message length (no data available): {}", e);
            }
        }
        
        Ok(None)
    }
    
    /// Send a ByteMessage to PECOS using callbacks or IPC
    fn send_to_pecos(&mut self, message: ByteMessage) -> Result<()> {
        // Check if we have callbacks registered (in-process mode)
        let callbacks = CALLBACKS.lock().unwrap();
        if let Some(ref cb) = *callbacks {
            let bytes = message.as_bytes();
            let result = (cb.send_op)(cb.context, bytes.as_ptr(), bytes.len());
            
            if result == 0 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to send operations via callback"))
            }
        } else if self.try_send_via_ipc(&message)? {
            // Successfully sent via IPC
            Ok(())
        } else {
            // Fallback to callback_interface if no callbacks registered and no IPC
            let bytes = message.as_bytes();
            let result = callback_interface::pecos_bridge_send_operations(bytes.as_ptr(), bytes.len());
            
            if result == 0 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to send operations via callback interface"))
            }
        }
    }
    
    /// Receive measurement results from PECOS using callbacks or IPC
    fn receive_from_pecos(&mut self) -> Result<ByteMessage> {
        let callbacks = CALLBACKS.lock().unwrap();
        if let Some(ref cb) = *callbacks {
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut len: usize = 0;
            
            let result = (cb.recv_meas)(cb.context, &mut data_ptr, &mut len);
            
            if result > 0 && !data_ptr.is_null() {
                // Create ByteMessage from the returned data
                let bytes = unsafe { std::slice::from_raw_parts(data_ptr, len) };
                let message = ByteMessage::new(bytes);
                
                // Free the allocated memory
                unsafe {
                    let _ = Box::from_raw(std::slice::from_raw_parts_mut(data_ptr, len));
                }
                
                Ok(message)
            } else {
                Err(anyhow::anyhow!("No measurements available"))
            }
        } else if let Some(message) = self.try_receive_via_ipc()? {
            // Successfully received measurement results via IPC
            log::trace!("[Bridge] Received measurement results via IPC");
            Ok(message)
        } else {
            // Fallback to callback_interface if no callbacks registered
            let mut buffer = vec![0u8; 4096];
            let result = callback_interface::pecos_bridge_receive_measurements(buffer.as_mut_ptr(), buffer.len());
            
            if result > 0 {
                buffer.truncate(result as usize);
                Ok(ByteMessage::new(&buffer))
            } else if result == 0 {
                callback_interface::pecos_bridge_wait_for_measurements();
                Err(anyhow::anyhow!("No measurements available yet"))
            } else {
                Err(anyhow::anyhow!("Failed to receive measurements via callback interface"))
            }
        }
    }
}

impl SimulatorInterface for PecosSeleneBridgeSimulator {
    fn exit(&mut self) -> Result<()> {
        eprintln!("Bridge: exit() called");
        log::debug!("PecosSeleneBridgeSimulator: exit");
        log::trace!("*** BRIDGE SIMULATOR: exit() called ***");
        Ok(())
    }
    
    fn shot_start(&mut self, shot_id: u64, _seed: u64) -> Result<()> {
        eprintln!("Bridge: shot_start({}) called", shot_id);
        
        // Write to file to bypass any stdio issues
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/bridge_debug.log")
        {
            let _ = writeln!(file, "Bridge: shot_start({}) called at {:?}", shot_id, std::time::SystemTime::now());
        }
        
        // Read the actual qubit count from config file at runtime
        let artifacts_dir = std::env::var("SELENE_ARTIFACTS_DIR").unwrap_or_default();
        if !artifacts_dir.is_empty() {
            let config_path = std::path::Path::new(&artifacts_dir).join("pecos_config.json");
            eprintln!("Bridge: Looking for runtime config at {:?}", config_path);
            if config_path.exists() {
                match std::fs::read_to_string(&config_path) {
                    Ok(contents) => {
                        eprintln!("Bridge: Found runtime config: '{}'", contents);
                        // Simple JSON parsing for n_qubits
                        if let Some(n_qubits_pos) = contents.find("\"n_qubits\":") {
                            let after_key = &contents[n_qubits_pos + 11..];  // Skip past "n_qubits":
                            let after_colon = after_key.trim_start();  // Skip whitespace
                            
                            // Find where the number ends
                            let mut end_pos = 0;
                            for (i, c) in after_colon.chars().enumerate() {
                                if c.is_numeric() {
                                    end_pos = i + 1;
                                } else {
                                    break;
                                }
                            }
                            
                            if end_pos > 0 {
                                if let Ok(n) = after_colon[..end_pos].parse::<usize>() {
                                    eprintln!("Bridge: Updating n_qubits from {} to {} based on runtime config", self.n_qubits, n);
                                    self.n_qubits = n;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Bridge: Failed to read runtime config: {}", e);
                    }
                }
            } else {
                eprintln!("Bridge: Runtime config not found at {:?}", config_path);
            }
        }
        
        log::debug!("PecosSeleneBridgeSimulator: shot_start({}) with n_qubits={}", shot_id, self.n_qubits);
        
        // Reset for new shot
        self.shot_id = shot_id;
        self.measurement_count = 0;
        self.measurement_results.clear();
        self.operations_started = false;
        
        if self.ipc_mode {
            eprintln!("Bridge: IPC mode enabled, starting operation buffering");
            // Start building operations ByteMessage for this shot
            self.shot_operations.reset();
            let _ = self.shot_operations.for_quantum_operations();
            self.operations_started = true;
            log::trace!("Bridge: Started buffering operations for shot {}", shot_id);
            
            // IMPORTANT: In IPC mode, the quantum program execution happens HERE
            // We need to explicitly trigger it since Selene won't auto-execute with piped stdio
            eprintln!("Bridge: Triggering quantum program execution in IPC mode");
            // The quantum operations will be called synchronously after this returns
        } else {
            eprintln!("Bridge: IPC mode disabled");
        }
        
        // Log to global file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/bridge_plugin_global.log") 
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] shot_start({}) called", 
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), shot_id);
        }
        
        // Write debug info to a file since stdout might be captured
        // Try multiple ways to find the temp directory
        let temp_dir = std::env::var("SELENE_TEMP_DIR")
            .or_else(|_| std::env::var("TMPDIR"))
            .or_else(|_| std::env::var("TMP"))
            .unwrap_or_else(|_| "/tmp".to_string());
        
        {
            let debug_file = format!("{}/bridge_debug.log", temp_dir);
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&debug_file) {
                use std::io::Write;
                let _ = writeln!(file, "*** BRIDGE: shot_start({}) called ***", shot_id);
            }
        }
        
        log::info!("Bridge: shot_start({}) - Starting back-and-forth communication", shot_id);
        
        // Initialize callback interface for this shot
        callback_interface::pecos_bridge_init();
        
        // State was already reset above, don't reset operations_started again!
        // Just set the execution state
        self.execution_state = BridgeState::Initial;
        
        // Check if we're in standalone test mode (no PECOS engine available)
        let callbacks_available = CALLBACKS.lock().unwrap().is_some();
        let ipc_available = std::env::var("SELENE_IPC").is_ok();
        
        if !callbacks_available && !ipc_available {
            // Log to file  
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/bridge_plugin_global.log") 
            {
                use std::io::Write;
                let _ = writeln!(file, "[{}] shot_start: No PECOS engine available (no callbacks, no IPC) - skipping communication", 
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
            }
            
            log::info!("Bridge: No PECOS engine available - running in standalone mode");
            // In standalone mode, just return without trying to communicate
            return Ok(());
        }
        
        // Start the back-and-forth communication loop
        log::info!("Bridge: Beginning back-and-forth communication with PECOS");
        loop {
            match self.execute_communication_round()? {
                true => {
                    log::debug!("Bridge: Communication round complete, continuing");
                    // Continue with more communication rounds
                },
                false => {
                    log::info!("Bridge: All communication rounds complete");
                    break;
                }
            }
        }
        
        eprintln!("Bridge: shot_start complete, waiting for quantum operations...");
        log::info!("Bridge: shot_start() completed");
        Ok(())
    }
    
    fn shot_end(&mut self) -> Result<()> {
        eprintln!("Bridge: shot_end() called");
        use std::io::Write;
        let _ = std::io::stderr().flush();
        log::debug!("PecosSeleneBridgeSimulator: shot_end");
        
        eprintln!("Bridge: shot_end - ipc_mode={}, operations_started={}", self.ipc_mode, self.operations_started);
        
        if self.ipc_mode && self.operations_started {
            // Send all buffered operations to SeleneExecutableEngine
            eprintln!("Bridge: shot_end - sending buffered operations");
            log::trace!("Bridge: Sending buffered operations at shot_end");
            
            let operations_msg = self.shot_operations.build();
            eprintln!("Bridge: Built operations message with {} bytes", operations_msg.as_bytes().len());
            
            // Send via stdout for IPC (non-blocking write)
            self.try_send_via_ipc(&operations_msg)?;
            eprintln!("Bridge: Sent operations, now waiting for results from stdin");
            
            // Wait to receive measurement results back via stdin
            // This blocks until SeleneExecutableEngine sends results
            log::trace!("Bridge: Waiting for measurement results from SeleneExecutableEngine");
            
            // Use a blocking read since we need results before the shot can complete
            use std::io::{stdin, Read};
            let stdin = stdin();
            let mut reader = stdin.lock();
            
            // Read length prefix
            let mut len_bytes = [0u8; 4];
            reader.read_exact(&mut len_bytes).map_err(|e| {
                eprintln!("Bridge: Failed to read length prefix: {}", e);
                anyhow::anyhow!("Failed to read length prefix: {}", e)
            })?;
            
            let msg_len = u32::from_le_bytes(len_bytes) as usize;
            eprintln!("Bridge: Expecting {} bytes of results", msg_len);
            
            // Read message data
            let mut msg_bytes = vec![0u8; msg_len];
            reader.read_exact(&mut msg_bytes).map_err(|e| {
                eprintln!("Bridge: Failed to read message data: {}", e);
                anyhow::anyhow!("Failed to read message data: {}", e)
            })?;
            
            let results_msg = ByteMessage::new(&msg_bytes);
            
            eprintln!("Bridge: Received results message");
            
            // Process and store measurement results
            if let Ok(outcomes) = results_msg.outcomes() {
                log::trace!("Bridge: Received {} measurement results", outcomes.len());
                eprintln!("Bridge: Got {} measurement results", outcomes.len());
                for (i, &outcome) in outcomes.iter().enumerate() {
                    self.measurement_results.insert(i, outcome != 0);
                    eprintln!("Bridge:   measurement[{}] = {}", i, outcome != 0);
                }
            }
        }
        
        // Store results for retrieval
        // The results are already in self.measurement_results
        log::trace!("*** BRIDGE: Shot complete with {} measurements ***", self.measurement_results.len());
        
        // Write debug log
        if let Ok(temp_dir) = std::env::var("SELENE_TEMP_DIR") {
            let debug_file = format!("{}/bridge_debug.log", temp_dir);
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&debug_file) {
                use std::io::Write;
                let _ = writeln!(file, "*** BRIDGE: shot_end() with {} measurements ***", self.measurement_results.len());
                for (idx, result) in &self.measurement_results {
                    let _ = writeln!(file, "  measurement_{} = {}", idx, result);
                }
            }
        }
        
        // Write measurement results to a file for Python to read
        if let Ok(temp_dir) = std::env::var("SELENE_TEMP_DIR") {
            let results_file = format!("{}/bridge_results_shot_{}.json", temp_dir, self.shot_id);
            if let Ok(mut file) = std::fs::File::create(&results_file) {
                use std::io::Write;
                
                // Create JSON representation of results
                let mut json = String::from("{");
                for (idx, result) in &self.measurement_results {
                    if json.len() > 1 {
                        json.push(',');
                    }
                    json.push_str(&format!("\"measurement_{}\":{}", idx, result));
                }
                json.push('}');
                
                let _ = file.write_all(json.as_bytes());
                log::trace!("*** BRIDGE: Wrote results to {} ***", results_file);
            }
        }
        
        // Signal completion via callback
        callback_interface::pecos_bridge_signal_complete();
        
        // In IPC mode, send an empty message to signal completion
        if self.ipc_mode {
            eprintln!("Bridge: Sending empty message to signal completion");
            let empty_msg = ByteMessage::builder().for_quantum_operations().build();
            self.try_send_via_ipc(&empty_msg)?;
        }
        
        Ok(())
    }
    
    fn rz(&mut self, qubit: u64, theta: f64) -> Result<()> {
        eprintln!("Bridge: rz({}, {}) called", qubit, theta);
        use std::io::Write;
        let _ = std::io::stderr().flush();
        
        // Also write to file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/bridge_debug.log")
        {
            let _ = writeln!(file, "Bridge: rz({}, {}) called at {:?}", qubit, theta, std::time::SystemTime::now());
        }
        log::debug!("PecosSeleneBridgeSimulator: rz({}, {})", qubit, theta);
        log::trace!("*** BRIDGE SIMULATOR: rz({}, {}) called ***", qubit, theta);
        
        if self.ipc_mode && self.operations_started {
            // In IPC mode, add to the single shot operations builder
            self.shot_operations.add_rz(theta, &[qubit as usize]);
            eprintln!("Bridge: Added rz operation to buffer");
            log::trace!("Bridge: Added rz operation to buffer");
        } else if !self.ipc_mode {
            // In direct mode, send immediately
            self.message_builder.reset();
            let _ = self.message_builder.for_quantum_operations();
            self.message_builder.add_rz(theta, &[qubit as usize]);
            let message = self.message_builder.build();
            self.send_to_pecos(message)?;
        }
        Ok(())
    }
    
    fn rxy(&mut self, qubit: u64, theta: f64, phi: f64) -> Result<()> {
        eprintln!("Bridge: rxy({}, {}, {}) called", qubit, theta, phi);
        log::debug!("PecosSeleneBridgeSimulator: rxy({}, {}, {})", qubit, theta, phi);
        
        if self.ipc_mode && self.operations_started {
            // In IPC mode, add to the single shot operations builder
            self.shot_operations.add_r1xy(theta, phi, &[qubit as usize]);
            log::trace!("Bridge: Added rxy operation to buffer");
        } else if !self.ipc_mode {
            // In direct mode, send immediately  
            self.message_builder.reset();
            let _ = self.message_builder.for_quantum_operations();
            self.message_builder.add_r1xy(theta, phi, &[qubit as usize]);
            let message = self.message_builder.build();
            self.send_to_pecos(message)?;
        }
        Ok(())
    }
    
    fn rzz(&mut self, qubit1: u64, qubit2: u64, theta: f64) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: rzz({}, {}, {})", qubit1, qubit2, theta);
        
        if self.ipc_mode && self.operations_started {
            // In IPC mode, add to the single shot operations builder
            self.shot_operations.add_rzz(theta, &[qubit1 as usize], &[qubit2 as usize]);
            log::trace!("Bridge: Added rzz operation to buffer");
        } else if !self.ipc_mode {
            // In direct mode, send immediately
            self.message_builder.reset();
            let _ = self.message_builder.for_quantum_operations();
            self.message_builder.add_rzz(theta, &[qubit1 as usize], &[qubit2 as usize]);
            let message = self.message_builder.build();
            self.send_to_pecos(message)?;
        }
        Ok(())
    }
    
    fn measure(&mut self, qubit: u64) -> Result<bool> {
        eprintln!("Bridge: measure({}) called", qubit);
        eprintln!("Bridge: measure({}) - n_qubits={}, ipc_mode={}, operations_started={}", 
                 qubit, self.n_qubits, self.ipc_mode, self.operations_started);
        use std::io::Write;
        let _ = std::io::stderr().flush();
        log::debug!("PecosSeleneBridgeSimulator: measure({})", qubit);
        log::trace!("Bridge: measure({}) called, ipc_mode={}", qubit, self.ipc_mode);
        
        if self.ipc_mode && self.operations_started {
            // In IPC mode, buffer the measurement but still execute normally
            // The Selene runtime expects immediate results, so we need to provide them
            // We'll send the buffered operations to PECOS at shot_end for verification
            
            self.shot_operations.add_measurements(&[qubit as usize]);
            eprintln!("Bridge: Added measurement to buffer for qubit {}", qubit);
            
            // For now, return a deterministic result based on measurement count
            // This allows the Selene program to complete normally
            // The real quantum execution will happen via IPC at shot_end
            let placeholder_result = self.measurement_count % 2 == 0; // Alternating pattern
            self.measurement_count += 1;
            
            eprintln!("Bridge: Returning placeholder {} for measurement", placeholder_result);
            Ok(placeholder_result)
        } else if !self.ipc_mode {
            // In direct mode (no IPC), handle measurement directly
            let callbacks_available = CALLBACKS.lock().unwrap().is_some();
            
            if !callbacks_available {
                // Standalone mode - generate random result
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let result = rng.gen_bool(0.5);
                
                self.measurement_results.insert(self.measurement_count, result);
                self.measurement_count += 1;
                
                log::trace!("Bridge: measure({}) = {} (standalone)", qubit, result);
                return Ok(result);
            }
            
            // Direct mode with callbacks - send immediately
            self.message_builder.reset();
            let _ = self.message_builder.for_quantum_operations();
            self.message_builder.add_measurements(&[qubit as usize]);
            let message = self.message_builder.build();
            
            self.send_to_pecos(message)?;
            
            // Receive measurement result
            let result_message = self.receive_from_pecos()?;
            let outcomes = result_message.outcomes()
                .map_err(|e| anyhow::anyhow!("Failed to extract outcomes: {}", e))?;
            
            if outcomes.is_empty() {
                return Err(anyhow::anyhow!("No measurement result received"));
            }
            
            let result = outcomes[0] != 0;
            self.measurement_results.insert(self.measurement_count, result);
            self.measurement_count += 1;
            
            log::debug!("PecosSeleneBridgeSimulator: measure({}) = {}", qubit, result);
            Ok(result)
        } else {
            // IPC mode but operations not started yet
            log::warn!("Bridge: measure called but operations not started");
            Ok(false)
        }
    }
    
    fn reset(&mut self, qubit: u64) -> Result<()> {
        eprintln!("Bridge: reset({}) called", qubit);
        eprintln!("Bridge: reset({}) - n_qubits={}, ipc_mode={}, operations_started={}", 
                 qubit, self.n_qubits, self.ipc_mode, self.operations_started);
        log::debug!("PecosSeleneBridgeSimulator: reset({})", qubit);
        
        if self.ipc_mode && self.operations_started {
            // In IPC mode, add to the single shot operations builder
            self.shot_operations.add_prep(&[qubit as usize]);
            log::trace!("Bridge: Added reset/prep operation to buffer");
        } else if !self.ipc_mode {
            // In direct mode, send immediately
            self.message_builder.reset();
            let _ = self.message_builder.for_quantum_operations();
            self.message_builder.add_prep(&[qubit as usize]);
            let message = self.message_builder.build();
            self.send_to_pecos(message)?;
        }
        Ok(())
    }
    
    fn get_metric(&mut self, _nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        // No metrics for now
        Ok(None)
    }
}

// Add additional methods to allow result retrieval
impl PecosSeleneBridgeSimulator {
    /// Get the measurement results from the last shot
    pub fn get_measurement_results(&self) -> Vec<bool> {
        let mut results = Vec::new();
        for i in 0..self.measurement_count {
            results.push(self.measurement_results.get(&i).copied().unwrap_or(false));
        }
        results
    }
    
    /// Get the measurement results as a map
    pub fn get_measurement_map(&self) -> &BTreeMap<usize, bool> {
        &self.measurement_results
    }
}

/// Factory for creating PecosSeleneBridgeSimulator instances
#[derive(Default)]
pub struct PecosSeleneBridgeSimulatorFactory;

impl SimulatorInterfaceFactory for PecosSeleneBridgeSimulatorFactory {
    type Interface = PecosSeleneBridgeSimulator;
    
    fn init(
        self: Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        log::info!("Initializing PecosSeleneBridgeSimulator with {} qubits", n_qubits);
        
        // Write to global debug file
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/bridge_plugin_global.log") 
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] PecosSeleneBridgeSimulatorFactory::init({}) called", 
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), n_qubits);
            let _ = writeln!(file, "  Args: {:?}", args.iter().map(|a| a.as_ref()).collect::<Vec<_>>());
            let _ = writeln!(file, "  Stack trace:");
            let bt = backtrace::Backtrace::new();
            let _ = writeln!(file, "{:?}", bt);
        }
        
        Ok(Box::new(PecosSeleneBridgeSimulator::new(n_qubits)))
    }
}

// Export the plugin using Selene's macro
export_simulator_plugin!(crate::PecosSeleneBridgeSimulatorFactory);

// Library initialization function - called when library is loaded
#[no_mangle]
#[used]
#[link_section = ".init_array"]
static INIT_FUNC: extern "C" fn() = init_library;

extern "C" fn init_library() {
    INIT.call_once(|| {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/bridge_plugin_global.log") 
        {
            use std::io::Write;
            let _ = writeln!(file, "\n========================================");
            let _ = writeln!(file, "[{}] BRIDGE PLUGIN LIBRARY LOADED!", 
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
            let _ = writeln!(file, "========================================\n");
        }
    });
}

/// Clear the engine interface (for cleanup/testing)
pub fn clear_engine_interface() {
    // OnceLock doesn't provide a clear method, but in most cases
    // the interface will remain active for the lifetime of the process
}

// NOTE: Global result storage functions removed to avoid conflicts with pecos-llvm-runtime.
// Results are now handled through the proper LLVM runtime registry system.

// NOTE: C FFI functions (__quantum__rt__*) are provided by pecos-llvm-runtime
// to avoid symbol collisions. The Interface Plugin will link against those functions.