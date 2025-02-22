use super::Processor;
use crate::message::ptr::AlignedCast;
use crate::message::{
    BatchBuilder, GateType, MeasResult, MeasResultData, MessageBatch, MessageHeader, MessageType,
    OperationData, ProgramData,
};
use std::ptr::NonNull;

// Track program processing state
struct ProgramState {
    // Program metadata
    program_length: usize,
    current_position: usize,

    // Current operation pointer
    current_data_ptr: Option<NonNull<u8>>,

    // Collected measurement results
    measurements: Vec<MeasResultData>,
}

impl ProgramState {
    fn new() -> Self {
        Self {
            program_length: 0,
            current_position: 0,
            current_data_ptr: None,
            measurements: Vec::new(),
        }
    }

    fn has_more_operations(&self) -> bool {
        self.current_position < self.program_length
    }

    fn store_measurement(&mut self, result: MeasResultData) {
        self.measurements.push(result);
    }
}

/// Program processor that chunks operations for simulation
pub struct ProgramProcessor {
    state: ProgramState,
}

impl Default for ProgramProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramProcessor {
    /// Create a new program processor
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ProgramState::new(),
        }
    }

    // Helper to build final results batch
    fn build_final_results(&self) -> MessageBatch {
        println!(
            "ProgramProcessor: Building final results with {} measurements",
            self.state.measurements.len()
        );

        let mut builder = BatchBuilder::new();

        // Important: Start with Halted message
        builder.add_message(MessageType::Halted, &[]);

        // Then add all measurements after the Halted message
        for result in &self.state.measurements {
            let meas_result = MeasResult {
                outcome: result.outcome,
                is_deterministic: false,
            };
            builder.add_measurement_result(result.qubit, &meas_result);
        }

        builder.build()
    }

    // Process next set of operations until measurement or end
    fn process_next_operations(&mut self) -> MessageBatch {
        let mut builder = BatchBuilder::new();

        // Start with Input message
        builder.add_message(MessageType::Input, &[]);

        // Process operations until measurement or end
        while let Some(ptr) = self.state.current_data_ptr {
            if !self.state.has_more_operations() {
                break;
            }

            unsafe {
                let current = ptr.as_ptr();
                let op = &*current.cast_aligned::<OperationData>();
                let qubit_ptr = current.add(std::mem::size_of::<OperationData>());
                let qubits = std::slice::from_raw_parts(
                    qubit_ptr.cast_aligned::<u32>(),
                    op.num_qubits as usize,
                );

                println!(
                    "P1: Operation {}: {:?} with {} qubits: {:?}",
                    self.state.current_position, op.gate_type, op.num_qubits, qubits
                );

                // Add quantum op to output batch
                builder.add_quantum_ops(op.gate_type, qubits);

                // Update state for next operation
                let op_size = std::mem::size_of::<OperationData>()
                    + (op.num_qubits as usize * std::mem::size_of::<u32>());

                let next = current.add(op_size);
                let align = next.align_offset(std::mem::align_of::<OperationData>());
                self.state.current_data_ptr = NonNull::new(next.add(align).cast::<u8>());
                self.state.current_position += 1;

                // If measurement, send current batch to simulator
                if op.gate_type == GateType::Measure {
                    println!("P1: Measurement encountered - sending ops to simulator");
                    return builder.build();
                }
            }
        }

        // If we get here, we're done - return final results
        let mut final_builder = BatchBuilder::new();
        for result in &self.state.measurements {
            let meas_result = MeasResult {
                outcome: result.outcome,
                is_deterministic: false,
            };
            final_builder.add_measurement_result(result.qubit, &meas_result);
        }
        final_builder.add_message(MessageType::Halted, &[]);
        final_builder.build()
    }
}

#[allow(clippy::cast_ptr_alignment)]
impl Processor for ProgramProcessor {
    fn process(&mut self, batch: MessageBatch) -> MessageBatch {
        unsafe {
            let mut current = batch.data;
            let header = &*current.cast_aligned::<MessageHeader>();
            current = current.add(std::mem::size_of::<MessageHeader>());

            match header.msg_type {
                MessageType::Input => {
                    // First check for and process any measurement results in this batch
                    if header.payload_size == 0 {
                        let mut cursor = current;
                        let end = batch.data.add(batch.total_size as usize);

                        while cursor < end {
                            let msg_header = &*cursor.cast_aligned::<MessageHeader>();
                            cursor = cursor.add(std::mem::size_of::<MessageHeader>());

                            match msg_header.msg_type {
                                MessageType::MeasResult => {
                                    let result = &*cursor.cast_aligned::<MeasResultData>();
                                    println!(
                                        "ProgramProcessor: Received measurement result for qubit {}: {}",
                                        result.qubit,
                                        if result.outcome { "|1⟩" } else { "|0⟩" }
                                    );
                                    self.state.store_measurement(*result);
                                    cursor = cursor.add(std::mem::size_of::<MeasResultData>());
                                }
                                _ => {
                                    cursor = cursor.add(msg_header.payload_size as usize);
                                }
                            }
                        }
                    }

                    // Now check for program data
                    if header.payload_size == 0 {
                        // Check if there's a following message with program data
                        if current < batch.data.add(batch.total_size as usize) {
                            let next_header = &*current.cast_aligned::<MessageHeader>();
                            if next_header.msg_type == MessageType::Input {
                                current = current.add(std::mem::size_of::<MessageHeader>());
                                let program = &*current.cast_aligned::<ProgramData>();
                                println!(
                                    "ProgramProcessor: Loading program with {} operations",
                                    program.num_operations
                                );

                                let first_op = current.add(std::mem::size_of::<ProgramData>());
                                self.state.current_data_ptr = NonNull::new(first_op.cast_mut());
                                self.state.program_length = program.num_operations as usize;
                                self.state.current_position = 0;

                                return self.process_next_operations();
                            }
                        }

                        // If we have existing state, continue processing
                        if self.state.has_more_operations() {
                            self.process_next_operations()
                        } else {
                            // No more operations - return final results
                            self.build_final_results()
                        }
                    } else {
                        // Direct program data
                        let program = &*current.cast_aligned::<ProgramData>();
                        println!(
                            "ProgramProcessor: Loading program with {} operations",
                            program.num_operations
                        );

                        let first_op = current.add(std::mem::size_of::<ProgramData>());
                        self.state.current_data_ptr = NonNull::new(first_op.cast_mut());
                        self.state.program_length = program.num_operations as usize;
                        self.state.current_position = 0;

                        self.process_next_operations()
                    }
                }
                MessageType::MeasResult => {
                    // Store measurement result
                    let result = &*current.cast_aligned::<MeasResultData>();
                    println!(
                        "ProgramProcessor: Received measurement result for qubit {}: {}",
                        result.qubit,
                        if result.outcome { "|1⟩" } else { "|0⟩" }
                    );
                    self.state.store_measurement(*result);

                    // Continue with next operations
                    if self.state.has_more_operations() {
                        self.process_next_operations()
                    } else {
                        self.build_final_results()
                    }
                }
                _ => {
                    // For any other message type, including Halted,
                    // return final results if we have any
                    self.build_final_results()
                }
            }
        }
    }
}
