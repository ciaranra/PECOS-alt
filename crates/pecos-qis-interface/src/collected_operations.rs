//! Collected quantum operations from program execution
//!
//! This module defines the data structure for storing quantum operations
//! that have been collected during program execution.

use crate::{Operation, QuantumOp};
use std::collections::HashMap;

/// Collection of quantum operations from a program execution
///
/// This struct holds all the quantum operations that were collected
/// during the execution of a QIS program, along with metadata about
/// allocated resources.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CollectedOperations {
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

impl CollectedOperations {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue an operation to be executed
    pub fn queue_operation(&mut self, op: QuantumOp) {
        self.operations.push(Operation::Quantum(op));
    }

    /// Allocate a new qubit
    pub fn allocate_qubit(&mut self) -> usize {
        let id = self.next_qubit_id;
        self.next_qubit_id += 1;
        self.allocated_qubits.push(id);
        id
    }

    /// Allocate a new result ID
    pub fn allocate_result(&mut self) -> usize {
        let id = self.next_result_id;
        self.next_result_id += 1;
        self.allocated_results.push(id);
        id
    }

    /// Free a qubit
    pub fn free_qubit(&mut self, _qubit_id: usize) {
        // TODO: Track freed qubits
    }

    /// Clear all operations and reset state
    pub fn clear(&mut self) {
        self.operations.clear();
        self.measurements.clear();
        self.allocated_qubits.clear();
        self.allocated_results.clear();
        self.next_qubit_id = 0;
        self.next_result_id = 0;
    }

    /// Get the number of operations
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

// Keep compatibility with old name for now
#[deprecated(since = "0.2.0", note = "Use `CollectedOperations` instead")]
pub type QisInterface = CollectedOperations;