//! Mock QIS Runtime for deterministic testing
//!
//! This module provides a mock implementation of QisRuntime that returns
//! predetermined results for testing purposes. It allows tests to verify
//! the control logic without depending on actual quantum simulation.

use pecos_qis_runtime_trait::{QisRuntime, ClassicalState, Shot, Result};
use pecos_qis_interface::{QisInterface, QuantumOp};
use std::collections::HashMap;

/// Mock runtime that returns predetermined results for testing
#[derive(Debug, Clone)]
pub struct MockRuntime {
    /// Classical state for the runtime
    state: ClassicalState,
    /// Predetermined measurement results (qubit_id -> result)
    measurement_results: HashMap<usize, bool>,
    /// Number of qubits to report as allocated
    num_qubits: usize,
    /// Whether the runtime should report as complete
    is_complete: bool,
    /// Quantum operations to return on next execute_until_quantum call
    next_operations: Option<Vec<QuantumOp>>,
    /// Operations that have been processed (for verification)
    processed_operations: Vec<QuantumOp>,
}

impl MockRuntime {
    /// Create a new mock runtime
    pub fn new() -> Self {
        Self {
            state: ClassicalState::default(),
            measurement_results: HashMap::new(),
            num_qubits: 0,
            is_complete: true,
            next_operations: None,
            processed_operations: Vec::new(),
        }
    }

    /// Set a predetermined measurement result for a qubit
    pub fn set_measurement_result(mut self, qubit_id: usize, result: bool) -> Self {
        self.measurement_results.insert(qubit_id, result);
        self
    }

    /// Set multiple measurement results at once
    pub fn with_measurement_results(mut self, results: HashMap<usize, bool>) -> Self {
        self.measurement_results = results;
        self
    }

    /// Set the number of qubits this runtime should report
    pub fn with_qubits(mut self, num_qubits: usize) -> Self {
        self.num_qubits = num_qubits;
        self
    }

    /// Set whether the runtime should report as complete
    pub fn set_complete(mut self, complete: bool) -> Self {
        self.is_complete = complete;
        self
    }

    /// Set the quantum operations to return on the next execute_until_quantum call
    pub fn with_next_operations(mut self, operations: Vec<QuantumOp>) -> Self {
        self.next_operations = Some(operations);
        self
    }

    /// Get the list of operations that were processed (for test verification)
    pub fn processed_operations(&self) -> &[QuantumOp] {
        &self.processed_operations
    }

    /// Clear the processed operations list
    pub fn clear_processed_operations(&mut self) {
        self.processed_operations.clear();
    }
}

impl Default for MockRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl QisRuntime for MockRuntime {
    fn load_interface(&mut self, interface: QisInterface) -> Result<()> {
        // Update number of qubits based on interface
        self.num_qubits = interface.allocated_qubits.len();

        // Extract quantum operations from the interface for verification
        for operation in interface.operations {
            if let pecos_qis_interface::Operation::Quantum(qop) = operation {
                self.processed_operations.push(qop);
            }
        }

        log::debug!("MockRuntime loaded interface with {} qubits, {} quantum operations",
                   self.num_qubits, self.processed_operations.len());
        Ok(())
    }

    fn execute_until_quantum(&mut self) -> Result<Option<Vec<QuantumOp>>> {
        // Return any operations we were configured to return
        if let Some(ops) = self.next_operations.take() {
            log::debug!("MockRuntime returning {} quantum operations", ops.len());
            Ok(Some(ops))
        } else if self.is_complete {
            // If no operations and we're complete, return None
            log::debug!("MockRuntime execution complete");
            Ok(None)
        } else {
            // If not complete but no operations, return empty vec (waiting for measurements)
            log::debug!("MockRuntime waiting for measurements");
            Ok(Some(Vec::new()))
        }
    }

    fn provide_measurements(&mut self, measurements: HashMap<usize, bool>) -> Result<()> {
        log::debug!("MockRuntime received {} measurements", measurements.len());
        self.state.measurements.extend(measurements);
        Ok(())
    }

    fn get_classical_state(&self) -> &ClassicalState {
        &self.state
    }

    fn get_classical_state_mut(&mut self) -> &mut ClassicalState {
        &mut self.state
    }

    fn is_complete(&self) -> bool {
        self.is_complete
    }

    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn shot_end(&mut self) -> Result<Shot> {
        // Return a shot with the predetermined measurement results
        let mut shot = Shot {
            measurements: self.measurement_results.clone(),
            registers: self.state.registers.clone(),
            metadata: HashMap::new(),
        };

        // If we have received measurements during execution, use those instead
        if !self.state.measurements.is_empty() {
            shot.measurements = self.state.measurements.clone();
        }

        log::debug!("MockRuntime ending shot with {} measurements", shot.measurements.len());
        Ok(shot)
    }
}


/// Convenience function to create a MockRuntime for Bell state testing
///
/// Returns a mock runtime configured for 2-qubit Bell state with alternating outcomes
pub fn mock_bell_state_runtime() -> MockRuntime {
    MockRuntime::new()
        .with_qubits(2)
        .set_measurement_result(0, false)  // |00⟩ state
        .set_measurement_result(1, false)
}

/// Convenience function to create a MockRuntime that always returns |11⟩
pub fn mock_all_ones_runtime() -> MockRuntime {
    MockRuntime::new()
        .with_qubits(2)
        .set_measurement_result(0, true)  // |11⟩ state
        .set_measurement_result(1, true)
}

/// Convenience function to create a MockRuntime with custom pattern
pub fn mock_pattern_runtime(pattern: &[(usize, bool)]) -> MockRuntime {
    let mut runtime = MockRuntime::new();

    for &(qubit_id, result) in pattern {
        runtime = runtime.set_measurement_result(qubit_id, result);
        runtime.num_qubits = runtime.num_qubits.max(qubit_id + 1);
    }

    runtime
}

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_qis_interface::{QisInterface, QuantumOp};

    #[test]
    fn test_mock_runtime_creation() {
        let runtime = MockRuntime::new();
        assert_eq!(runtime.num_qubits(), 0);
        assert!(runtime.is_complete());
        assert_eq!(runtime.processed_operations().len(), 0);
    }

    #[test]
    fn test_mock_runtime_with_predetermined_results() {
        let runtime = MockRuntime::new()
            .with_qubits(2)
            .set_measurement_result(0, true)
            .set_measurement_result(1, false);

        assert_eq!(runtime.num_qubits(), 2);
        assert_eq!(runtime.measurement_results.get(&0), Some(&true));
        assert_eq!(runtime.measurement_results.get(&1), Some(&false));
    }

    #[test]
    fn test_mock_runtime_interface_loading() {
        let mut runtime = MockRuntime::new();

        // Create a simple interface
        let mut interface = QisInterface::new();
        let q0 = interface.allocate_qubit();
        let q1 = interface.allocate_qubit();
        interface.queue_operation(QuantumOp::H(q0).into());
        interface.queue_operation(QuantumOp::CX(q0, q1).into());

        // Load the interface
        runtime.load_interface(interface).unwrap();

        assert_eq!(runtime.num_qubits(), 2);
        assert_eq!(runtime.processed_operations().len(), 2);
    }

    #[test]
    fn test_mock_runtime_shot_generation() {
        let mut runtime = MockRuntime::new()
            .with_qubits(2)
            .set_measurement_result(0, true)
            .set_measurement_result(1, true);

        let shot = runtime.shot_end().unwrap();

        // Check that we get the predetermined results
        assert_eq!(shot.measurements.get(&0), Some(&true));
        assert_eq!(shot.measurements.get(&1), Some(&true));
    }

    #[test]
    fn test_mock_runtime_convenience_functions() {
        let bell_runtime = mock_bell_state_runtime();
        assert_eq!(bell_runtime.num_qubits(), 2);
        assert_eq!(bell_runtime.measurement_results.get(&0), Some(&false));
        assert_eq!(bell_runtime.measurement_results.get(&1), Some(&false));

        let ones_runtime = mock_all_ones_runtime();
        assert_eq!(ones_runtime.measurement_results.get(&0), Some(&true));
        assert_eq!(ones_runtime.measurement_results.get(&1), Some(&true));

        let pattern_runtime = mock_pattern_runtime(&[(0, true), (2, false), (1, true)]);
        assert_eq!(pattern_runtime.num_qubits(), 3);
        assert_eq!(pattern_runtime.measurement_results.get(&0), Some(&true));
        assert_eq!(pattern_runtime.measurement_results.get(&1), Some(&true));
        assert_eq!(pattern_runtime.measurement_results.get(&2), Some(&false));
    }
}