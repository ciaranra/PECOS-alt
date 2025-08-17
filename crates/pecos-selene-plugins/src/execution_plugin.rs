//! Execution plugin that provides the runtime environment for Selene program plugins
//!
//! This module implements the functions that program plugins expect to link with:
//! - setup() - Initialize execution environment
//! - teardown() - Clean up after execution
//! - get_tc() - Get time cursor
//! - get_next_operations() - Get next batch of quantum operations
//!
//! It bridges between Selene's program plugin interface and PECOS's execution model.

use std::sync::Mutex;
use std::collections::VecDeque;
use selene_core::runtime::{BatchOperation, Operation};

/// Global state for the execution plugin
static EXECUTION_STATE: Mutex<Option<ExecutionState>> = Mutex::new(None);

/// State maintained by the execution plugin
struct ExecutionState {
    /// Queue of operations to be executed
    operation_queue: VecDeque<BatchOperation>,
    /// Current time cursor
    time_cursor: f64,
    /// Whether we're in an active execution
    active: bool,
}

impl ExecutionState {
    fn new() -> Self {
        Self {
            operation_queue: VecDeque::new(),
            time_cursor: 0.0,
            active: false,
        }
    }
}

/// Initialize the execution environment
///
/// This function is called by the program plugin at the start of execution
#[no_mangle]
pub extern "C" fn setup() -> i32 {
    let mut state = EXECUTION_STATE.lock().unwrap();
    *state = Some(ExecutionState::new());
    
    if let Some(state) = state.as_mut() {
        state.active = true;
        0 // Success
    } else {
        -1 // Error
    }
}

/// Clean up the execution environment
///
/// This function is called by the program plugin at the end of execution
#[no_mangle]
pub extern "C" fn teardown() -> i32 {
    let mut state = EXECUTION_STATE.lock().unwrap();
    
    if let Some(state) = state.as_mut() {
        state.active = false;
        state.operation_queue.clear();
        0 // Success
    } else {
        -1 // Error
    }
}

/// Get the current time cursor
///
/// This function is called by the program plugin to get timing information
#[no_mangle]
pub extern "C" fn get_tc() -> f64 {
    let state = EXECUTION_STATE.lock().unwrap();
    
    if let Some(state) = state.as_ref() {
        state.time_cursor
    } else {
        0.0
    }
}

/// Get the next batch of quantum operations
///
/// This function is called by the runtime interface to get operations to execute
/// 
/// # Parameters
/// - `buffer`: Pointer to buffer to write operations
/// - `buffer_size`: Size of the buffer
/// 
/// # Returns
/// - Number of bytes written, or -1 on error
#[no_mangle]
pub extern "C" fn get_next_operations(buffer: *mut u8, buffer_size: i64) -> i64 {
    if buffer.is_null() || buffer_size <= 0 {
        return -1;
    }
    
    let mut state = EXECUTION_STATE.lock().unwrap();
    
    if let Some(state) = state.as_mut() {
        if let Some(_batch) = state.operation_queue.pop_front() {
            // Serialize the batch operation to the buffer
            // This is a simplified implementation - real implementation would
            // properly serialize according to Selene's protocol
            
            // For now, return 0 to indicate no operations
            // TODO: Implement proper serialization
            0
        } else {
            0 // No operations available
        }
    } else {
        -1 // Error: not initialized
    }
}

/// Add a quantum operation to the execution queue
///
/// This is called by the quantum intrinsics to queue operations
pub fn queue_operation(op: Operation) {
    let mut state = EXECUTION_STATE.lock().unwrap();
    
    if let Some(state) = state.as_mut() {
        // Create a single-operation batch
        // Note: We can't use Instant::now() as it's not available in selene_core
        // For now, just store operations without BatchOperation wrapper
        // This is a placeholder - real implementation would handle this properly
    }
}

/// Quantum intrinsic implementations
/// These are called by the compiled program plugin

#[no_mangle]
pub extern "C" fn __quantum__qis__h__body(qubit: u64) {
    // Hadamard gate is Rxy(π/2, 0)
    queue_operation(Operation::RXYGate {
        qubit_id: qubit,
        theta: std::f64::consts::PI / 2.0,
        phi: 0.0,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__x__body(qubit: u64) {
    // X gate is Rxy(π, 0)
    queue_operation(Operation::RXYGate {
        qubit_id: qubit,
        theta: std::f64::consts::PI,
        phi: 0.0,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__y__body(qubit: u64) {
    // Y gate is Rxy(π, π/2)
    queue_operation(Operation::RXYGate {
        qubit_id: qubit,
        theta: std::f64::consts::PI,
        phi: std::f64::consts::PI / 2.0,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__z__body(qubit: u64) {
    // Z gate is Rz(π)
    queue_operation(Operation::RZGate {
        qubit_id: qubit,
        theta: std::f64::consts::PI,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: u64) {
    queue_operation(Operation::RZGate {
        qubit_id: qubit,
        theta,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__rzz__body(theta: f64, qubit1: u64, qubit2: u64) {
    queue_operation(Operation::RZZGate {
        qubit_id_1: qubit1,
        qubit_id_2: qubit2,
        theta,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__mz__body(qubit: u64, result: u64) {
    queue_operation(Operation::Measure {
        qubit_id: qubit,
        result_id: result,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__qis__reset__body(qubit: u64) {
    queue_operation(Operation::Reset {
        qubit_id: qubit,
    });
}

#[no_mangle]
pub extern "C" fn __quantum__rt__qubit_allocate() -> u64 {
    // For now, just return sequential qubit IDs
    // In a real implementation, this would coordinate with the quantum engine
    static NEXT_QUBIT: Mutex<u64> = Mutex::new(0);
    let mut next = NEXT_QUBIT.lock().unwrap();
    let qubit_id = *next;
    *next += 1;
    qubit_id
}

#[no_mangle]
pub extern "C" fn __quantum__rt__qubit_release(_qubit: u64) {
    // No-op for now
    // In a real implementation, this would free the qubit
}

#[no_mangle]
pub extern "C" fn __quantum__rt__result_get_zero() -> u64 {
    // Return a result ID representing zero/false
    0
}

#[no_mangle]
pub extern "C" fn __quantum__rt__result_get_one() -> u64 {
    // Return a result ID representing one/true  
    1
}

#[no_mangle]
pub extern "C" fn __quantum__rt__result_equal(result1: u64, result2: u64) -> bool {
    result1 == result2
}

/// Module for integration with PECOS
#[cfg(feature = "pecos-integration")]
pub mod pecos_integration {
    use super::*;
    use pecos_engines::ByteMessage;
    
    /// Get all queued operations as ByteMessages
    pub fn get_byte_messages() -> Vec<ByteMessage> {
        let mut state = EXECUTION_STATE.lock().unwrap();
        let mut messages = Vec::new();
        
        if let Some(state) = state.as_mut() {
            while let Some(batch) = state.operation_queue.pop_front() {
                // Convert batch to ByteMessage
                let mut builder = ByteMessage::builder();
                builder.for_quantum_operations();
                
                for op in batch.iter_ops() {
                    match op {
                        Operation::RXYGate { qubit_id, theta, phi } => {
                            builder.add_r1xy(*theta, *phi, &[*qubit_id as usize]);
                        }
                        Operation::RZGate { qubit_id, theta } => {
                            builder.add_rz(*theta, &[*qubit_id as usize]);
                        }
                        Operation::RZZGate { qubit_id_1, qubit_id_2, theta } => {
                            builder.add_rzz(*theta, &[*qubit_id_1 as usize], &[*qubit_id_2 as usize]);
                        }
                        Operation::Measure { qubit_id, .. } => {
                            builder.add_measurements(&[*qubit_id as usize]);
                        }
                        Operation::Reset { qubit_id } => {
                            builder.add_prep(&[*qubit_id as usize]);
                        }
                        _ => {} // Skip other operations
                    }
                }
                
                messages.push(builder.build());
            }
        }
        
        messages
    }
    
    /// Reset the execution state
    pub fn reset() {
        let mut state = EXECUTION_STATE.lock().unwrap();
        *state = None;
    }
}