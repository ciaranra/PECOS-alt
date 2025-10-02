//! Native Rust implementation of QisRuntime
//!
//! This provides a simple native Rust interpreter for QIS programs.
//! It processes the operations collected by QisInterface and manages
//! classical control flow without requiring external dependencies.

use log::{debug, trace};
use pecos_qis_interface::{Operation, QisInterface, QuantumOp};
use pecos_qis_runtime_trait::{ClassicalState, QisRuntime, Result, RuntimeError};
use std::collections::BTreeMap;

/// Native Rust implementation of QisRuntime
///
/// This is a simple interpreter that processes QIS operations sequentially.
/// It's primarily useful for testing and as a reference implementation.
#[derive(Clone)]
pub struct NativeRuntime {
    /// The loaded QIS interface
    interface: Option<QisInterface>,

    /// Current classical state
    state: ClassicalState,

    /// Operations buffer for batching
    operations_buffer: Vec<QuantumOp>,

    /// Maximum batch size for operations
    batch_size: usize,

    /// Index of current operation being processed
    current_op_index: usize,

    /// Number of qubits in the program
    num_qubits: usize,
}

impl NativeRuntime {
    /// Create a new native runtime
    pub fn new() -> Self {
        Self {
            interface: None,
            state: ClassicalState::default(),
            operations_buffer: Vec::new(),
            batch_size: 100,
            current_op_index: 0,
            num_qubits: 0,
        }
    }

    /// Extract all qubit IDs referenced by a quantum operation
    fn extract_qubit_ids(&self, qop: &QuantumOp) -> Vec<usize> {
        match qop {
            // Single-qubit gates
            QuantumOp::H(q) | QuantumOp::X(q) | QuantumOp::Y(q) | QuantumOp::Z(q) |
            QuantumOp::S(q) | QuantumOp::Sdg(q) | QuantumOp::T(q) | QuantumOp::Tdg(q) |
            QuantumOp::Reset(q) => vec![*q],

            // Rotation gates
            QuantumOp::RX(_, q) | QuantumOp::RY(_, q) | QuantumOp::RZ(_, q) |
            QuantumOp::RXY(_, _, q) => vec![*q],

            // Two-qubit gates
            QuantumOp::CX(c, t) | QuantumOp::CY(c, t) | QuantumOp::CZ(c, t) |
            QuantumOp::CH(c, t) | QuantumOp::ZZ(c, t) | QuantumOp::RZZ(_, c, t) => vec![*c, *t],

            // Controlled rotations
            QuantumOp::CRZ(_, c, t) => vec![*c, *t],

            // Three-qubit gates
            QuantumOp::CCX(c1, c2, t) => vec![*c1, *c2, *t],

            // Measurement
            QuantumOp::Measure(q, _) => vec![*q],
        }
    }

    /// Process the next operation from the interface
    fn process_next_operation(&mut self) -> Result<Option<QuantumOp>> {
        let interface = self
            .interface
            .as_ref()
            .ok_or(RuntimeError::NoProgramLoaded)?;

        if self.current_op_index >= interface.operations.len() {
            return Ok(None);
        }

        let op = &interface.operations[self.current_op_index];
        self.current_op_index += 1;

        match op {
            Operation::Quantum(qop) => {
                trace!("Processing quantum operation: {:?}", qop);
                Ok(Some(qop.clone()))
            }
            Operation::AllocateQubit { id } => {
                trace!("Allocating qubit {}", id);
                self.num_qubits = self.num_qubits.max(*id + 1);
                // Process next operation
                self.process_next_operation()
            }
            Operation::AllocateResult { id } => {
                trace!("Allocating result {}", id);
                // Just track it, process next operation
                let _ = id;
                self.process_next_operation()
            }
            Operation::ReleaseQubit { id } => {
                trace!("Releasing qubit {}", id);
                // Just track it, process next operation
                let _ = id;
                self.process_next_operation()
            }
            Operation::Barrier => {
                trace!("Barrier encountered");
                // Barrier doesn't produce quantum ops
                self.process_next_operation()
            }
        }
    }
}

impl Default for NativeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl QisRuntime for NativeRuntime {
    fn load_interface(&mut self, interface: QisInterface) -> Result<()> {
        debug!("Loading QIS interface with {} operations", interface.operations.len());

        // Count qubits from both explicit allocations AND from operations that reference qubits
        let max_qubit_from_allocations = interface.allocated_qubits.iter().max().cloned();
        let mut max_qubit_from_operations: Option<usize> = None;

        // Scan all operations to find the maximum qubit ID referenced
        for operation in &interface.operations {
            match operation {
                Operation::Quantum(qop) => {
                    let qubits = self.extract_qubit_ids(qop);
                    if let Some(max_in_op) = qubits.into_iter().max() {
                        max_qubit_from_operations = Some(max_qubit_from_operations.map_or(max_in_op, |current: usize| current.max(max_in_op)));
                    }
                }
                Operation::AllocateQubit { id } => {
                    max_qubit_from_operations = Some(max_qubit_from_operations.map_or(*id, |current: usize| current.max(*id)));
                }
                Operation::ReleaseQubit { id } => {
                    max_qubit_from_operations = Some(max_qubit_from_operations.map_or(*id, |current: usize| current.max(*id)));
                }
                _ => {} // Other operations don't reference qubits
            }
        }

        // Use the maximum qubit ID found from either source
        let max_qubit = match (max_qubit_from_allocations, max_qubit_from_operations) {
            (Some(a), Some(o)) => Some(a.max(o)),
            (Some(a), None) => Some(a),
            (None, Some(o)) => Some(o),
            (None, None) => None,
        };

        self.num_qubits = max_qubit.map_or(0, |q| q + 1);
        debug!("Determined {} qubits needed (from allocations: {:?}, from operations: {:?})",
               self.num_qubits, max_qubit_from_allocations, max_qubit_from_operations);

        self.interface = Some(interface);
        self.current_op_index = 0;
        Ok(())
    }

    fn execute_until_quantum(&mut self) -> Result<Option<Vec<QuantumOp>>> {
        self.operations_buffer.clear();

        // Process operations until we have a batch or reach the end
        while self.operations_buffer.len() < self.batch_size {
            match self.process_next_operation()? {
                Some(qop) => {
                    self.operations_buffer.push(qop);
                }
                None => {
                    // No more operations
                    break;
                }
            }
        }

        if self.operations_buffer.is_empty() {
            trace!("No more quantum operations");
            Ok(None)
        } else {
            trace!("Returning batch of {} quantum operations", self.operations_buffer.len());
            Ok(Some(self.operations_buffer.clone()))
        }
    }

    fn provide_measurements(&mut self, measurements: BTreeMap<usize, bool>) -> Result<()> {
        debug!("Received {} measurement results", measurements.len());

        // Store measurements in classical state
        for (result_id, value) in measurements {
            trace!("Measurement result {} = {}", result_id, value);
            self.state.measurements.insert(result_id, value);

            // Also update the interface if it exists
            if let Some(interface) = &mut self.interface {
                interface.store_result(result_id, value);
            }
        }

        Ok(())
    }

    fn get_classical_state(&self) -> &ClassicalState {
        &self.state
    }

    fn get_classical_state_mut(&mut self) -> &mut ClassicalState {
        &mut self.state
    }

    fn is_complete(&self) -> bool {
        self.interface
            .as_ref()
            .map_or(false, |i| self.current_op_index >= i.operations.len())
    }

    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }

    fn reset(&mut self) -> Result<()> {
        // Reset the state but keep the interface
        self.state = ClassicalState::default();
        self.operations_buffer.clear();
        self.current_op_index = 0;  // Reset to beginning of operations
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_qis_interface::QisInterface;

    #[test]
    fn test_native_runtime_basic() {
        let _ = env_logger::try_init();

        let mut runtime = NativeRuntime::new();

        // Create a simple interface with some operations
        let mut interface = QisInterface::new();
        let q0 = interface.allocate_qubit();
        let q1 = interface.allocate_qubit();
        let r0 = interface.allocate_result();

        interface.queue_operation(Operation::AllocateQubit { id: q0 });
        interface.queue_operation(Operation::AllocateQubit { id: q1 });
        interface.queue_operation(QuantumOp::H(q0).into());
        interface.queue_operation(QuantumOp::CX(q0, q1).into());
        interface.queue_operation(QuantumOp::Measure(q0, r0).into());

        // Load the interface
        runtime.load_interface(interface).unwrap();

        // Execute and get quantum operations
        let ops = runtime.execute_until_quantum().unwrap();
        assert!(ops.is_some());

        let ops = ops.unwrap();
        assert_eq!(ops.len(), 3); // H, CX, Measure

        // Verify operations
        assert_eq!(ops[0], QuantumOp::H(0));
        assert_eq!(ops[1], QuantumOp::CX(0, 1));
        assert_eq!(ops[2], QuantumOp::Measure(0, 0));

        // Should be complete now
        assert!(runtime.is_complete());
    }

    #[test]
    fn test_measurement_feedback() {
        let mut runtime = NativeRuntime::new();

        let mut interface = QisInterface::new();
        let q0 = interface.allocate_qubit();
        let r0 = interface.allocate_result();

        interface.queue_operation(QuantumOp::H(q0).into());
        interface.queue_operation(QuantumOp::Measure(q0, r0).into());

        runtime.load_interface(interface).unwrap();

        // Execute operations
        let _ops = runtime.execute_until_quantum().unwrap();

        // Provide measurement result
        let mut measurements = BTreeMap::new();
        measurements.insert(r0, true);
        runtime.provide_measurements(measurements).unwrap();

        // Check that measurement was stored
        assert_eq!(runtime.get_classical_state().measurements.get(&r0), Some(&true));
    }
}