use crate::channels::Message;
use crate::engines::Engine;
use crate::errors::QueueError;
use log::debug;
use pecos_core::types::{GateType, QuantumCommand};
use pecos_qsim::{ArbitraryRotationGateable, CliffordGateable, QuantumSimulator};

/// Marker trait for engines that manage quantum state
///
/// This trait indicates that an engine specifically deals with
/// quantum state evolution and measurements.
pub trait QuantumEngine: Engine<Input = QuantumCommand, Output = Option<Message>> {}

// Engine for simulators that only support Clifford gates
pub struct CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync,
{
    simulator: S,
}

impl<S> CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync,
{
    pub fn new(simulator: S) -> Self {
        Self { simulator }
    }
}

impl<S> Engine for CliffordEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync,
{
    type Input = QuantumCommand;
    type Output = Option<Message>;

    fn process(&mut self, cmd: Self::Input) -> Result<Self::Output, QueueError> {
        match &cmd.gate {
            GateType::H => {
                debug!("Processing H gate on qubit {:?}", cmd.qubits[0]);
                self.simulator.h(cmd.qubits[0]);
                Ok(None)
            }
            GateType::CX => {
                debug!(
                    "Processing CX gate with control {:?} and target {:?}",
                    cmd.qubits[0], cmd.qubits[1]
                );
                self.simulator.cx(cmd.qubits[0], cmd.qubits[1]);
                Ok(None)
            }
            GateType::SZZ => {
                debug!("Processing SZZ gate on qubits {:?}", cmd.qubits);
                self.simulator.szz(cmd.qubits[0], cmd.qubits[1]);
                Ok(None)
            }
            GateType::Measure { result_id: _ } => {
                let result = self.simulator.mz(cmd.qubits[0]);
                let measurement = u32::from(result.outcome);
                debug!(
                    "Generated measurement {} for qubit {:?}",
                    measurement, cmd.qubits[0]
                );
                Ok(Some(measurement))
            }
            GateType::RZ { .. } | GateType::R1XY { .. } => Err(QueueError::OperationError(
                "This simulator only supports Clifford operations".into(),
            )),
        }
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.simulator.reset();
        Ok(())
    }
}

impl<S> QuantumEngine for CliffordEngine<S> where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync
{
}

// Engine for simulators that support arbitrary rotations using state vectors
pub struct ArbitraryQGateEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + ArbitraryRotationGateable<usize> + Send + Sync,
{
    simulator: S,
}

impl<S> ArbitraryQGateEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + ArbitraryRotationGateable<usize> + Send + Sync,
{
    pub fn new(simulator: S) -> Self {
        Self { simulator }
    }
}

impl<S> Engine for ArbitraryQGateEngine<S>
where
    S: QuantumSimulator + CliffordGateable<usize> + ArbitraryRotationGateable<usize> + Send + Sync,
{
    type Input = QuantumCommand;
    type Output = Option<Message>;

    fn process(&mut self, cmd: Self::Input) -> Result<Self::Output, QueueError> {
        match &cmd.gate {
            GateType::H => {
                debug!("Processing H gate on qubit {:?}", cmd.qubits[0]);
                self.simulator.h(cmd.qubits[0]);
                Ok(None)
            }
            GateType::CX => {
                debug!(
                    "Processing CX gate with control {:?} and target {:?}",
                    cmd.qubits[0], cmd.qubits[1]
                );
                self.simulator.cx(cmd.qubits[0], cmd.qubits[1]);
                Ok(None)
            }
            GateType::SZZ => {
                debug!("Processing SZZ gate on qubits {:?}", cmd.qubits);
                self.simulator.szz(cmd.qubits[0], cmd.qubits[1]);
                Ok(None)
            }
            GateType::Measure { result_id: _ } => {
                let result = self.simulator.mz(cmd.qubits[0]);
                let measurement = u32::from(result.outcome);
                debug!(
                    "Generated measurement {} for qubit {:?}",
                    measurement, cmd.qubits[0]
                );
                Ok(Some(measurement))
            }
            GateType::RZ { theta } => {
                debug!(
                    "Processing RZ gate with theta={} on qubit {:?}",
                    theta, cmd.qubits[0]
                );
                self.simulator.rz(*theta, cmd.qubits[0]);
                Ok(None)
            }
            GateType::R1XY { phi, theta } => {
                debug!(
                    "Processing R1XY gate with phi={}, theta={} on qubit {:?}",
                    phi, theta, cmd.qubits[0]
                );
                self.simulator.r1xy(*theta, *phi, cmd.qubits[0]);
                Ok(None)
            }
        }
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.simulator.reset();
        Ok(())
    }
}

impl<S> QuantumEngine for ArbitraryQGateEngine<S> where
    S: QuantumSimulator + CliffordGateable<usize> + ArbitraryRotationGateable<usize> + Send + Sync
{
}

// Factory function to create the appropriate engine based on simulator type
pub fn new_quantum_engine<S>(simulator: S) -> Box<dyn QuantumEngine>
where
    S: QuantumSimulator + CliffordGateable<usize> + Send + Sync + 'static,
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
        + 'static,
{
    Box::new(ArbitraryQGateEngine::new(simulator))
}
