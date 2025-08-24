//! PECOS-Selene Bridge Simulator Plugin
//!
//! This plugin acts as a bridge between Selene's simulator interface and PECOS's
//! ByteMessage system. It allows Selene programs to run naturally while converting
//! operations directly to ByteMessages for integration with PECOS quantum engines.

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
    
    println!("[Bridge] Engine callbacks registered");
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
    
    /// Operations to send in current communication round
    pending_operations: Vec<ByteMessage>,
}

impl PecosSeleneBridgeSimulator {
    fn new(n_qubits: u64) -> Self {
        Self {
            n_qubits: n_qubits as usize,
            message_builder: ByteMessageBuilder::new(),
            execution_state: BridgeState::Initial,
            shot_id: 0,
            measurement_count: 0,
            measurement_results: BTreeMap::new(),
            named_results: BTreeMap::new(),
            pending_operations: Vec::new(),
        }
    }
    
    /// Try to send ByteMessage via IPC (stdout) - returns true if IPC is available
    fn try_send_via_ipc(&mut self, message: &ByteMessage) -> Result<bool> {
        use std::io::{stdout, Write};
        
        // Simple heuristic: if SELENE_IPC env var is set, use IPC mode
        if std::env::var("SELENE_IPC").is_err() {
            println!("[Bridge] SELENE_IPC not set - not using IPC mode");
            return Ok(false);
        }
        
        println!("[Bridge] SELENE_IPC detected - using IPC mode");
        
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
        
        println!("[Bridge] Sent {} bytes via IPC (length-prefixed)", bytes.len());
        Ok(true)
    }
    
    /// Execute back-and-forth communication round - returns true if more rounds needed
    fn execute_communication_round(&mut self) -> Result<bool> {
        println!("[Bridge] execute_communication_round() - state: {:?}", self.execution_state);
        
        match self.execution_state {
            BridgeState::Initial => {
                // Send initial quantum operations to PECOS
                println!("[Bridge] Initial state - generating quantum operations");
                self.generate_initial_operations()?;
                self.send_pending_operations()?;
                
                self.execution_state = BridgeState::WaitingForMeasurements;
                Ok(true) // More communication needed
            },
            
            BridgeState::WaitingForMeasurements => {
                // Wait for measurement results from PECOS
                println!("[Bridge] Waiting for measurements from PECOS");
                if let Some(measurements) = self.try_receive_via_ipc()? {
                    println!("[Bridge] Received measurements, processing...");
                    self.process_measurements(measurements)?;
                    self.execution_state = BridgeState::ProcessingMeasurements;
                    Ok(true) // Continue processing
                } else {
                    println!("[Bridge] No measurements received yet, continuing to wait");
                    Ok(true) // Keep waiting
                }
            },
            
            BridgeState::ProcessingMeasurements => {
                // Process measurements and decide if more operations needed
                println!("[Bridge] Processing measurements state");
                if self.needs_more_operations() {
                    println!("[Bridge] Generating more operations based on measurements");
                    self.generate_conditional_operations()?;
                    self.send_pending_operations()?;
                    self.execution_state = BridgeState::WaitingForMeasurements;
                    Ok(true) // More rounds needed
                } else {
                    println!("[Bridge] No more operations needed - completing");
                    self.execution_state = BridgeState::Complete;
                    Ok(false) // Communication complete
                }
            },
            
            BridgeState::Complete => {
                println!("[Bridge] Already complete");
                Ok(false) // No more communication needed
            }
        }
    }
    
    /// Generate initial quantum operations to send to PECOS
    fn generate_initial_operations(&mut self) -> Result<()> {
        println!("[Bridge] Generating initial operations for {} qubits", self.n_qubits);
        
        // Create quantum operations message (gates + measurement requests)
        self.message_builder = ByteMessageBuilder::new();
        let _ = self.message_builder.for_quantum_operations();
        
        // Add gates
        for qubit_id in 0..self.n_qubits {
            // Add Hadamard gate
            self.message_builder.add_h(&[qubit_id]);
        }
        
        // Add measurement requests (not results!)
        for qubit_id in 0..self.n_qubits {
            self.message_builder.add_measurements(&[qubit_id]);
        }
        
        let quantum_ops_message = self.message_builder.build();
        self.pending_operations.push(quantum_ops_message);
        
        println!("[Bridge] Generated 1 message with quantum ops and measurement requests");
        Ok(())
    }
    
    /// Send all pending operations via IPC
    fn send_pending_operations(&mut self) -> Result<()> {
        let operations = self.pending_operations.clone();
        for operation in &operations {
            self.try_send_via_ipc(operation)?;
        }
        self.pending_operations.clear();
        Ok(())
    }
    
    /// Process measurement results received from PECOS
    fn process_measurements(&mut self, measurements: ByteMessage) -> Result<()> {
        println!("[Bridge] Processing measurement results");
        
        if let Ok(outcomes) = measurements.outcomes() {
            println!("[Bridge] Received {} measurement outcomes", outcomes.len());
            for (i, &outcome) in outcomes.iter().enumerate() {
                self.measurement_results.insert(i, outcome != 0);
                println!("[Bridge] Qubit {}: {}", i, outcome != 0);
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
        println!("[Bridge] Generating conditional operations based on measurements");
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
        
        println!("[Bridge] Trying to receive measurement results via IPC (stdin)");
        
        let stdin = stdin();
        let mut reader = stdin.lock();
        
        // Try to read the length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        match reader.read_exact(&mut len_bytes) {
            Ok(_) => {
                let msg_len = u32::from_le_bytes(len_bytes) as usize;
                println!("[Bridge] Message length: {} bytes", msg_len);
                
                // Read the message data
                let mut msg_bytes = vec![0u8; msg_len];
                match reader.read_exact(&mut msg_bytes) {
                    Ok(_) => {
                        println!("[Bridge] Read {} bytes of message data", msg_bytes.len());
                        
                        // Create ByteMessage from the data
                        let message = ByteMessage::new(&msg_bytes);
                        return Ok(Some(message));
                    }
                    Err(e) => {
                        println!("[Bridge] Failed to read message data: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("[Bridge] Failed to read message length (no data available): {}", e);
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
            println!("[Bridge] Received measurement results via IPC");
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
        log::debug!("PecosSeleneBridgeSimulator: exit");
        println!("*** BRIDGE SIMULATOR: exit() called ***");
        Ok(())
    }
    
    fn shot_start(&mut self, shot_id: u64, _seed: u64) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: shot_start({})", shot_id);
        println!("*** BRIDGE SIMULATOR: shot_start({}) - Starting back-and-forth communication ***", shot_id);
        
        // Initialize callback interface for this shot
        callback_interface::pecos_bridge_init();
        
        // Reset state for new shot
        self.shot_id = shot_id;
        self.measurement_count = 0;
        self.measurement_results.clear();
        self.named_results.clear();
        self.pending_operations.clear();
        self.execution_state = BridgeState::Initial;
        
        // Start the back-and-forth communication loop
        println!("*** BRIDGE SIMULATOR: Beginning back-and-forth communication with PECOS ***");
        loop {
            match self.execute_communication_round()? {
                true => {
                    println!("*** BRIDGE SIMULATOR: Communication round complete, continuing ***");
                    // Continue with more communication rounds
                },
                false => {
                    println!("*** BRIDGE SIMULATOR: All communication rounds complete ***");
                    break;
                }
            }
        }
        
        println!("*** BRIDGE SIMULATOR: shot_start() completed back-and-forth communication ***");
        Ok(())
    }
    
    fn shot_end(&mut self) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: shot_end");
        
        // Store results for retrieval
        // The results are already in self.measurement_results
        println!("*** BRIDGE: Shot complete with {} measurements ***", self.measurement_results.len());
        
        // Signal completion via callback
        callback_interface::pecos_bridge_signal_complete();
        
        Ok(())
    }
    
    fn rz(&mut self, qubit: u64, theta: f64) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: rz({}, {})", qubit, theta);
        println!("*** BRIDGE SIMULATOR: rz({}, {}) called ***", qubit, theta);
        
        // Build and send RZ operation
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        self.message_builder.add_rz(theta, &[qubit as usize]);
        let message = self.message_builder.build();
        
        self.send_to_pecos(message)?;
        Ok(())
    }
    
    fn rxy(&mut self, qubit: u64, theta: f64, phi: f64) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: rxy({}, {}, {})", qubit, theta, phi);
        
        // Build and send RXY operation (using R1XY in PECOS)
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        self.message_builder.add_r1xy(theta, phi, &[qubit as usize]);
        let message = self.message_builder.build();
        
        self.send_to_pecos(message)?;
        Ok(())
    }
    
    fn rzz(&mut self, qubit1: u64, qubit2: u64, theta: f64) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: rzz({}, {}, {})", qubit1, qubit2, theta);
        
        // Build and send RZZ operation
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        self.message_builder.add_rzz(theta, &[qubit1 as usize], &[qubit2 as usize]);
        let message = self.message_builder.build();
        
        self.send_to_pecos(message)?;
        Ok(())
    }
    
    fn measure(&mut self, qubit: u64) -> Result<bool> {
        log::debug!("PecosSeleneBridgeSimulator: measure({})", qubit);
        println!("*** BRIDGE SIMULATOR: measure({}) called ***", qubit);
        
        // Build and send measurement operation  
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();  // Fixed: use for_quantum_operations() for measurement requests
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
        println!("*** BRIDGE SIMULATOR: measure({}) = {} ***", qubit, result);
        Ok(result)
    }
    
    fn reset(&mut self, qubit: u64) -> Result<()> {
        log::debug!("PecosSeleneBridgeSimulator: reset({})", qubit);
        
        // Build and send reset operation (using prep in PECOS)
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        self.message_builder.add_prep(&[qubit as usize]);
        let message = self.message_builder.build();
        
        self.send_to_pecos(message)?;
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
        _args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        log::info!("Initializing PecosSeleneBridgeSimulator with {} qubits", n_qubits);
        Ok(Box::new(PecosSeleneBridgeSimulator::new(n_qubits)))
    }
}

// Export the plugin using Selene's macro
export_simulator_plugin!(crate::PecosSeleneBridgeSimulatorFactory);

/// Clear the engine interface (for cleanup/testing)
pub fn clear_engine_interface() {
    // OnceLock doesn't provide a clear method, but in most cases
    // the interface will remain active for the lifetime of the process
}

// NOTE: Global result storage functions removed to avoid conflicts with pecos-llvm-runtime.
// Results are now handled through the proper LLVM runtime registry system.

// NOTE: C FFI functions (__quantum__rt__*) are provided by pecos-llvm-runtime
// to avoid symbol collisions. The Interface Plugin will link against those functions.