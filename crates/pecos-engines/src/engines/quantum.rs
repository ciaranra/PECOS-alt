use crate::channels::ByteMessage;
use crate::channels::byte::gate_type::GateTypeId;
use crate::engines::Engine;
use crate::errors::QueueError;
use dyn_clone::DynClone;
use log::debug;
use pecos_qsim::{ArbitraryRotationGateable, CliffordGateable, QuantumSimulator, StateVec};
use std::fmt::Debug;

/// Trait for quantum engines that can process quantum operations
pub trait QuantumEngine:
    Engine<Input = ByteMessage, Output = ByteMessage> + DynClone + Debug
{
}

dyn_clone::clone_trait_object!(QuantumEngine);

/// A quantum engine that uses a state vector simulator
#[derive(Debug, Clone)]
pub struct StateVecEngine {
    simulator: StateVec,
}

impl StateVecEngine {
    /// Create a new state vector engine with the specified number of qubits
    #[must_use]
    pub fn new(num_qubits: usize) -> Self {
        Self {
            simulator: StateVec::new(num_qubits),
        }
    }
}

impl Engine for StateVecEngine {
    type Input = ByteMessage;
    type Output = ByteMessage;

    fn process(&mut self, message: Self::Input) -> Result<Self::Output, QueueError> {
        // Parse commands from the message
        let batch = message.parse_quantum_operations()?;
        let mut measurements = Vec::new();

        for cmd in &batch {
            match cmd.gate_type {
                GateTypeId::X => {
                    debug!("Processing X gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.x(cmd.qubits[0]);
                }
                GateTypeId::Y => {
                    debug!("Processing Y gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.y(cmd.qubits[0]);
                }
                GateTypeId::Z => {
                    debug!("Processing Z gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.z(cmd.qubits[0]);
                }
                GateTypeId::H => {
                    debug!("Processing H gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.h(cmd.qubits[0]);
                }
                GateTypeId::CX => {
                    debug!(
                        "Processing CX gate with control {:?} and target {:?}",
                        cmd.qubits[0], cmd.qubits[1]
                    );
                    self.simulator.cx(cmd.qubits[0], cmd.qubits[1]);
                }
                GateTypeId::SZZ => {
                    debug!(
                        "Processing SZZ gate on qubits {:?} and {:?}",
                        cmd.qubits[0], cmd.qubits[1]
                    );
                    self.simulator.szz(cmd.qubits[0], cmd.qubits[1]);
                }
                GateTypeId::RZ => {
                    if !cmd.params.is_empty() {
                        debug!(
                            "Processing RZ gate with angle {:?} on qubit {:?}",
                            cmd.params[0], cmd.qubits[0]
                        );
                        self.simulator.rz(cmd.params[0], cmd.qubits[0]);
                    }
                }
                GateTypeId::R1XY => {
                    if cmd.params.len() >= 2 {
                        debug!(
                            "Processing R1XY gate with angles phi={:?}, theta={:?} on qubit {:?}",
                            cmd.params[0], cmd.params[1], cmd.qubits[0]
                        );
                        self.simulator
                            .r1xy(cmd.params[0], cmd.params[1], cmd.qubits[0]);
                    }
                }
                GateTypeId::Measure => {
                    if let Some(result_id) = cmd.result_id {
                        debug!(
                            "Processing measurement on qubit {:?} with result_id {:?}",
                            cmd.qubits[0], result_id
                        );
                        let meas_result = self.simulator.mz(cmd.qubits[0]);
                        let outcome = u32::from(meas_result.outcome);
                        measurements.push((result_id, outcome));
                    }
                }
            }
        }

        // Create a message with the measurement results
        let result_message = ByteMessage::record_measurement_results(&measurements);
        Ok(result_message)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.simulator.reset();
        Ok(())
    }
}

impl QuantumEngine for StateVecEngine {}

/// Creates a new quantum engine that supports both Clifford gates and arbitrary rotation gates
///
/// This factory function creates a new `StateVecEngine` and returns it as a boxed `QuantumEngine`
/// trait object, allowing for polymorphic usage.
///
/// # Parameters
/// * `simulator` - A state vector simulator
///
/// # Returns
/// A boxed `QuantumEngine` trait object
#[must_use]
pub fn new_quantum_engine_arbitrary_qgate(simulator: StateVec) -> Box<dyn QuantumEngine> {
    Box::new(StateVecEngine { simulator })
}
