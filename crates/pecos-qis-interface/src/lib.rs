//! Minimal QIS Interface for Fast Linking
//!
//! This crate provides the minimal FFI interface needed to link QIS (Quantum Instruction Set)
//! programs with Rust functions. It's designed to be lightweight and compile quickly.
//!
//! The interface collects quantum operations during program execution without performing
//! any simulation or complex state management. These operations are later processed by
//! a QisRuntime implementation.

use log::trace;
use std::cell::RefCell;
use std::collections::HashMap;

pub mod ffi;
pub mod linker;
pub mod operations;
pub mod runtime;

pub use linker::{QisLinker, LinkerError};
pub use operations::{Operation, QuantumOp};

// Type alias to clarify the purpose
/// Collection of quantum operations from program execution
///
/// Note: This will be renamed in a future version. The name QisInterface
/// is being repurposed as a trait for interface implementations.
pub type OperationList = QisInterface;

thread_local! {
    /// Thread-local storage for the current QIS interface
    static INTERFACE: RefCell<QisInterface> = RefCell::new(QisInterface::new());
}

/// Interface for collecting quantum operations from a linked QIS program
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct QisInterface {
    /// Collected quantum operations in order
    pub operations: Vec<Operation>,

    /// Mapping of measurement result IDs to their values (when known)
    pub measurements: HashMap<usize, Option<bool>>,

    /// Allocated qubit IDs
    pub allocated_qubits: Vec<usize>,

    /// Allocated result IDs
    pub allocated_results: Vec<usize>,

    /// Next available qubit ID
    next_qubit_id: usize,

    /// Next available result ID
    next_result_id: usize,
}

impl QisInterface {
    /// Create a new QIS interface
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            measurements: HashMap::new(),
            allocated_qubits: Vec::new(),
            allocated_results: Vec::new(),
            next_qubit_id: 0,
            next_result_id: 0,
        }
    }

    /// Queue an operation for later execution
    pub fn queue_operation(&mut self, op: Operation) {
        trace!("Queueing operation: {:?}", op);
        self.operations.push(op);
    }

    /// Allocate a new qubit and return its ID
    pub fn allocate_qubit(&mut self) -> usize {
        let id = self.next_qubit_id;
        self.next_qubit_id += 1;
        self.allocated_qubits.push(id);
        trace!("Allocated qubit {}", id);
        id
    }

    /// Allocate a new result slot and return its ID
    pub fn allocate_result(&mut self) -> usize {
        let id = self.next_result_id;
        self.next_result_id += 1;
        self.allocated_results.push(id);
        self.measurements.insert(id, None);
        trace!("Allocated result {}", id);
        id
    }

    /// Store a measurement result (used by runtime when results are available)
    pub fn store_result(&mut self, result_id: usize, value: bool) {
        trace!("Storing result {} = {}", result_id, value);
        self.measurements.insert(result_id, Some(value));
    }

    /// Get a measurement result (blocks until available in actual runtime)
    pub fn get_result(&self, result_id: usize) -> Option<bool> {
        self.measurements.get(&result_id).and_then(|v| *v)
    }

    /// Reset the interface for a new shot
    pub fn reset(&mut self) {
        self.operations.clear();
        self.measurements.clear();
        self.allocated_qubits.clear();
        self.allocated_results.clear();
        self.next_qubit_id = 0;
        self.next_result_id = 0;
    }

    /// Extract the collected operations (consumes them)
    pub fn take_operations(&mut self) -> Vec<Operation> {
        std::mem::take(&mut self.operations)
    }
}

/// Get the thread-local interface
pub fn with_interface<F, R>(f: F) -> R
where
    F: FnOnce(&mut QisInterface) -> R,
{
    INTERFACE.with(|interface| f(&mut interface.borrow_mut()))
}

/// Reset the thread-local interface
pub fn reset_interface() {
    with_interface(|interface| interface.reset());
}
