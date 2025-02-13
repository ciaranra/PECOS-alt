use super::Processor;
use crate::message::ptr::AlignedCast;
use crate::message::{
    BatchBuilder, GateType, MeasResult, MessageBatch, MessageHeader, MessageType, QuantumOpData,
};
use pecos_qsim::{CliffordGateable, StateVec};

/// Simulator processor that executes quantum operations
pub struct SimulatorProcessor {
    simulator: StateVec,
}

impl SimulatorProcessor {
    /// Create a new simulator processor with the given number of qubits
    #[must_use]
    pub fn new(num_qubits: usize) -> Self {
        Self {
            simulator: StateVec::new(num_qubits),
        }
    }
}

#[allow(clippy::cast_ptr_alignment)]
impl Processor for SimulatorProcessor {
    fn process(&mut self, batch: MessageBatch) -> MessageBatch {
        let mut builder = BatchBuilder::new();

        unsafe {
            let mut current = batch.data;
            let end = current.add(batch.total_size as usize);

            // First message should be control message
            let header = &*current.cast_aligned::<MessageHeader>();
            current = current.add(std::mem::size_of::<MessageHeader>());

            match header.msg_type {
                MessageType::Input => {
                    // Skip input message payload
                    current = current.add(header.payload_size as usize);

                    // Process quantum operations until end
                    while current < end {
                        let op_header = &*current.cast_aligned::<MessageHeader>();
                        current = current.add(std::mem::size_of::<MessageHeader>());

                        match op_header.msg_type {
                            MessageType::QuantumOp => {
                                // Read quantum op data
                                let op = &*current.cast_aligned::<QuantumOpData>();
                                println!(
                                    "SimulatorProcessor: Processing {:?} gate with {} qubits",
                                    op.gate_type, op.num_qubits
                                );

                                // Read qubit indices
                                let qubit_ptr = current.add(std::mem::size_of::<QuantumOpData>());
                                let qubits = std::slice::from_raw_parts(
                                    qubit_ptr.cast_aligned::<u32>(),
                                    op.num_qubits as usize,
                                );

                                // Process operation
                                match op.gate_type {
                                    GateType::Measure => {
                                        println!(
                                            "SimulatorProcessor: Measuring qubit {}",
                                            qubits[0]
                                        );
                                        let result = self.simulator.mz(qubits[0] as usize);

                                        let meas_result = MeasResult {
                                            outcome: result.outcome,
                                            is_deterministic: result.is_deterministic,
                                        };
                                        builder.add_measurement_result(qubits[0], &meas_result);
                                    }
                                    GateType::H => {
                                        println!(
                                            "SimulatorProcessor: Applying H gate to qubit {}",
                                            qubits[0]
                                        );
                                        self.simulator.h(qubits[0] as usize);
                                    }
                                    GateType::CX => {
                                        println!(
                                            "SimulatorProcessor: Applying CX gate with control {} target {}",
                                            qubits[0], qubits[1]
                                        );
                                        self.simulator.cx(qubits[0] as usize, qubits[1] as usize);
                                    }
                                    _ => panic!(
                                        "SimulatorProcessor: Unsupported gate type: {:?}",
                                        op.gate_type
                                    ),
                                }

                                // Move to next message
                                current = current.add(op_header.payload_size as usize);
                            }
                            MessageType::Halted => break,
                            _ => current = current.add(op_header.payload_size as usize),
                        }
                    }
                }
                _ => panic!(
                    "SimulatorProcessor: Expected Input message, got {:?}",
                    header.msg_type
                ),
            }
        }

        // Add final Halted message
        builder.add_message(MessageType::Halted, &[]);
        builder.build()
    }
}
