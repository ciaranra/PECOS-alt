// src/engines/quantum.rs (fully updated interface)

use crate::channels::byte_message::ByteMessage;
use crate::engines::Engine;
use crate::errors::QueueError;
use log::debug;
use pecos_core::types::GateType;
use pecos_qsim::{ArbitraryRotationGateable, CliffordGateable, QuantumSimulator};

/// Marker trait for engines that manage quantum state
///
/// This trait indicates that an engine specifically deals with
/// quantum state evolution and measurements.
pub trait QuantumEngine: Engine<Input = ByteMessage, Output = ByteMessage> + Send + Sync {
    fn clone_box(&self) -> Box<dyn QuantumEngine>;
}

impl Engine for Box<dyn QuantumEngine> {
    type Input = ByteMessage;
    type Output = ByteMessage;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        self.as_mut().process(input)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.as_mut().reset()
    }
}

// Engine for simulators that only support Clifford gates
pub struct CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync + Clone + 'static,
{
    simulator: S,
}

impl<S> CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync + Clone + 'static,
{
    pub fn new(simulator: S) -> Self {
        Self { simulator }
    }
}

impl<S> Engine for CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync + Clone + 'static,
{
    type Input = ByteMessage;
    type Output = ByteMessage;

    fn process(&mut self, message: Self::Input) -> Result<Self::Output, QueueError> {
        // Parse commands from the message
        let batch = message.parse_quantum_operations()?;
        let mut measurements = Vec::new();

        for cmd in &batch {
            match &cmd.gate {
                GateType::X => {
                    debug!("Processing X gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.x(cmd.qubits[0]);
                }
                GateType::Y => {
                    debug!("Processing Y gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.y(cmd.qubits[0]);
                }
                GateType::Z => {
                    debug!("Processing Z gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.z(cmd.qubits[0]);
                }
                GateType::H => {
                    debug!("Processing H gate on qubit {:?}", cmd.qubits[0]);
                    self.simulator.h(cmd.qubits[0]);
                }
                GateType::CX => {
                    debug!(
                        "Processing CX gate with control {:?} and target {:?}",
                        cmd.qubits[0], cmd.qubits[1]
                    );
                    self.simulator.cx(cmd.qubits[0], cmd.qubits[1]);
                }
                GateType::SZZ => {
                    debug!("Processing SZZ gate on qubits {:?}", cmd.qubits);
                    self.simulator.szz(cmd.qubits[0], cmd.qubits[1]);
                }
                GateType::Measure { result_id } => {
                    debug!(
                        "QUANTUM SIM: Starting measurement of qubit {} (result_id={})",
                        cmd.qubits[0], result_id
                    );
                    let meas_result = self.simulator.mz(cmd.qubits[0]);
                    let raw_outcome = u32::from(meas_result.outcome);

                    // Convert result_id to u32 safely
                    let result_id_u32: u32 = *result_id as u32;

                    let encoded = (result_id_u32 << 16) | raw_outcome;
                    debug!(
                        "QUANTUM SIM: Got measurement {} for qubit {} with result_id={}",
                        raw_outcome, cmd.qubits[0], result_id
                    );
                    debug!("QUANTUM SIM: Encoded as {}", encoded);
                    measurements.push(encoded);
                }

                GateType::RZ { .. } | GateType::R1XY { .. } => {
                    return Err(QueueError::OperationError(
                        "This simulator only supports Clifford operations.".into(),
                    ));
                }
            }
        }

        // Create a measurement message
        if measurements.is_empty() {
            Ok(ByteMessage::builder().add_flush(true).build())
        } else {
            // Extract result_ids and outcomes from measurements
            let mut result_ids: Vec<usize> = Vec::with_capacity(measurements.len());
            let mut outcomes: Vec<usize> = Vec::with_capacity(measurements.len());

            for &m in &measurements {
                result_ids.push((m >> 16) as usize);
                outcomes.push((m & 0xFFFF) as usize);
            }

            Ok(ByteMessage::builder()
                .add_measurement_results(&outcomes, &result_ids)
                .build())
        }
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.simulator.reset();
        Ok(())
    }
}

impl<S> QuantumEngine for CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn QuantumEngine> {
        Box::new(CliffordEngine::new(self.simulator.clone()))
    }
}

// Engine for simulators that support arbitrary rotations using state vectors
pub struct ArbitraryQGateEngine<S>
where
    S: QuantumSimulator
        + CliffordGateable<usize>
        + ArbitraryRotationGateable<usize>
        + Send
        + Sync
        + Clone
        + 'static,
{
    simulator: S,
}

impl<S> ArbitraryQGateEngine<S>
where
    S: QuantumSimulator
        + CliffordGateable<usize>
        + ArbitraryRotationGateable<usize>
        + Send
        + Sync
        + Clone
        + 'static,
{
    pub fn new(simulator: S) -> Self {
        Self { simulator }
    }
}

impl<S> Engine for ArbitraryQGateEngine<S>
where
    S: QuantumSimulator
        + CliffordGateable<usize>
        + ArbitraryRotationGateable<usize>
        + Send
        + Sync
        + Clone
        + 'static,
{
    type Input = ByteMessage;
    type Output = ByteMessage;

    fn process(&mut self, message: Self::Input) -> Result<Self::Output, QueueError> {
        // Parse commands from the message
        let batch = message.parse_quantum_operations()?;
        let mut result_ids = Vec::new();
        let mut outcomes = Vec::new();

        for cmd in batch {
            debug!("Quantum engine processing command: {:?}", cmd);
            match cmd.gate {
                GateType::X => {
                    debug!("Executing X gate on qubit {}", cmd.qubits[0]);
                    self.simulator.x(cmd.qubits[0]);
                }
                GateType::Y => {
                    debug!("Executing Y gate on qubit {}", cmd.qubits[0]);
                    self.simulator.y(cmd.qubits[0]);
                }
                GateType::Z => {
                    debug!("Executing Z gate on qubit {}", cmd.qubits[0]);
                    self.simulator.z(cmd.qubits[0]);
                }
                GateType::H => {
                    debug!("Executing H gate on qubit {}", cmd.qubits[0]);
                    self.simulator.h(cmd.qubits[0]);
                }
                GateType::CX => {
                    debug!(
                        "Executing CX gate with control {} and target {}",
                        cmd.qubits[0],
                        cmd.qubits.get(1).unwrap_or(&0)
                    );

                    if cmd.qubits.len() == 2 {
                        self.simulator.cx(cmd.qubits[0], cmd.qubits[1]);
                    } else {
                        return Err(QueueError::OperationError(format!(
                            "Got the wrong number of arguments for CX: {}",
                            cmd.qubits.len()
                        )));
                    }
                }
                GateType::SZZ => {
                    debug!(
                        "Executing SZZ gate on qubits {} and {}",
                        cmd.qubits[0], cmd.qubits[1]
                    );
                    self.simulator.szz(cmd.qubits[0], cmd.qubits[1]);
                }
                GateType::Measure { result_id } => {
                    debug!(
                        "Starting measurement of qubit {} (result_id={})",
                        cmd.qubits[0], result_id
                    );
                    let meas_result = self.simulator.mz(cmd.qubits[0]);
                    let raw_outcome = u32::from(meas_result.outcome);

                    // Convert result_id to u32 safely
                    let result_id_u32: u32 = result_id as u32;

                    let encoded = (result_id_u32 << 16) | raw_outcome;
                    debug!(
                        "Measurement complete: qubit={}, result_id={}, outcome={}, encoded={}",
                        cmd.qubits[0], result_id, raw_outcome, encoded
                    );

                    // Store outcomes and result_ids separately
                    result_ids.push(result_id);
                    outcomes.push(raw_outcome as usize);
                }
                GateType::RZ { theta } => {
                    debug!("Executing RZ(theta={}) on qubit {}", theta, cmd.qubits[0]);
                    self.simulator.rz(theta, cmd.qubits[0]);
                }
                GateType::R1XY { phi, theta } => {
                    debug!(
                        "Executing R1XY(phi={}, theta={}) on qubit {}",
                        phi, theta, cmd.qubits[0]
                    );
                    self.simulator.r1xy(theta, phi, cmd.qubits[0]);
                }
            }
        }

        // Create a message with all measurements using the improved MessageBuilder API
        if outcomes.is_empty() {
            Ok(ByteMessage::builder().add_flush(true).build())
        } else {
            Ok(ByteMessage::builder()
                .add_measurement_results(&outcomes, &result_ids)
                .build())
        }
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        debug!("Resetting quantum simulator state");
        self.simulator.reset();
        debug!("Quantum simulator state reset complete");
        Ok(())
    }
}

impl<S> QuantumEngine for ArbitraryQGateEngine<S>
where
    S: QuantumSimulator
        + CliffordGateable<usize>
        + ArbitraryRotationGateable<usize>
        + Send
        + Sync
        + Clone
        + 'static,
{
    fn clone_box(&self) -> Box<dyn QuantumEngine> {
        Box::new(ArbitraryQGateEngine::new(self.simulator.clone()))
    }
}

// Factory function to create the appropriate engine based on simulator type
pub fn new_quantum_engine<S>(simulator: S) -> Box<dyn QuantumEngine>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync + Clone + 'static,
{
    Box::new(CliffordEngine::new(simulator))
}

pub fn new_quantum_engine_arbitrary_qgate<S>(simulator: S) -> Box<dyn QuantumEngine>
where
    S: QuantumSimulator
        + CliffordGateable<usize>
        + ArbitraryRotationGateable<usize>
        + Send
        + Sync
        + Clone
        + 'static,
{
    Box::new(ArbitraryQGateEngine::new(simulator))
}
