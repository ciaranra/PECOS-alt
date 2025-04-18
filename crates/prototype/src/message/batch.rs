use crate::message::ptr::AlignedCast;
use crate::message::types::{
    GateType, MeasResult, MeasResultData, MessageBatch, MessageHeader, MessageType, OperationData,
    ProgramData, QuantumOpData,
};

#[allow(clippy::cast_ptr_alignment)]
pub struct BatchBuilder {
    buffer: Vec<u8>,
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchBuilder {
    /// Create a new empty batch builder
    #[must_use]
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    // Helper to write a struct with proper alignment
    pub fn write_struct<T>(&mut self, value: &T) {
        // Calculate current position and required alignment
        let current_pos = self.buffer.len();
        let align = std::mem::align_of::<T>();
        let size = std::mem::size_of::<T>();

        // Add padding if needed
        if current_pos % align != 0 {
            let padding = align - (current_pos % align);
            self.buffer.extend(std::iter::repeat_n(0, padding));
        }

        // Create a zeroed buffer of the exact size we need
        let mut temp = vec![0u8; size];

        unsafe {
            // First zero the entire buffer
            std::ptr::write_bytes(temp.as_mut_ptr(), 0, size);

            // Then copy just the actual data, which will preserve our padding bytes as zero
            std::ptr::copy_nonoverlapping(
                std::ptr::from_ref::<T>(value).cast::<u8>(),
                temp.as_mut_ptr(),
                size,
            );
        }

        // Debug output
        println!(
            "Wrote struct of type {} at offset {} (aligned to {}), size {}",
            std::any::type_name::<T>(),
            current_pos,
            align,
            size
        );

        print!("  Bytes written: ");
        for b in &temp {
            print!("{b:02x} ");
        }
        println!();

        // Extend buffer
        self.buffer.extend_from_slice(&temp);
    }

    /// Add a message with header and data
    ///
    /// # Panics
    /// - If data length exceeds `u16::MAX`
    pub fn add_message(&mut self, msg_type: MessageType, data: &[u8]) {
        let payload_size =
            u16::try_from(data.len()).expect("Message payload size exceeds u16::MAX");

        let header = MessageHeader {
            msg_type,
            payload_size,
        };
        self.write_struct(&header);
        self.buffer.extend_from_slice(data);
    }

    /// Add a message with a measurement result
    ///
    /// # Panics
    /// - If `MeasResultData` size exceeds `u16::MAX`
    pub fn add_measurement_result(&mut self, qubit: u32, result: &MeasResult) {
        // If buffer is empty, start with Input message
        if self.buffer.is_empty() {
            self.add_message(MessageType::Input, &[]);
        }

        let data = MeasResultData {
            qubit,
            outcome: result.outcome,
        };

        let header = MessageHeader {
            msg_type: MessageType::MeasResult,
            payload_size: u16::try_from(std::mem::size_of::<MeasResultData>())
                .expect("MeasResultData size exceeds u16::MAX"),
        };
        self.write_struct(&header);
        self.write_struct(&data);
    }

    /// Add quantum operations to the batch
    ///
    /// # Panics
    /// - If qubit validation fails
    /// - If quantum operation size exceeds `u16::MAX`
    pub fn add_quantum_ops(&mut self, gate: GateType, qubits: &[u32]) {
        // Validate before writing
        if let Err(e) = QuantumOpData::validate_qubits(gate, qubits) {
            panic!("Invalid quantum operation: {e}");
        }

        // Calculate total size
        let quantum_size = std::mem::size_of::<QuantumOpData>() + std::mem::size_of_val(qubits);

        // Write header
        let header = MessageHeader {
            msg_type: MessageType::QuantumOp,
            payload_size: u16::try_from(quantum_size)
                .expect("Quantum operation size exceeds u16::MAX"),
        };
        self.write_struct(&header);

        // Write quantum data
        let quantum_op = QuantumOpData {
            gate_type: gate,
            num_qubits: u8::try_from(qubits.len()).expect("Too many qubits for operation"),
            has_extra_data: false,
            _pad: 0,
            qubit_indices: [],
        };
        self.write_struct(&quantum_op);

        // Write qubit indices
        for &qubit in qubits {
            self.buffer.extend_from_slice(&qubit.to_le_bytes());
        }

        // Debug output
        println!("Added quantum operation:");
        println!(
            "  Gate: {:?} with {} qubits: {:?}",
            gate,
            qubits.len(),
            qubits
        );
        println!(
            "  Message size: {} bytes",
            std::mem::size_of::<MessageHeader>() + quantum_size
        );
    }

    /// Add program operations to the batch
    ///
    /// # Panics
    /// - If program size exceeds `u16::MAX`
    /// - If number of operations exceeds `u32::MAX`
    /// - If number of qubits in any operation exceeds `u32::MAX`
    pub fn add_program(&mut self, operations: &[(GateType, Vec<u32>)]) {
        // Calculate total size
        let total_size = Self::calculate_program_size(operations);

        // Write single Input header with correct size
        let header = MessageHeader {
            msg_type: MessageType::Input,
            payload_size: u16::try_from(total_size).expect("Program size exceeds u16::MAX"),
        };
        self.write_struct(&header);

        // Write program data
        let program = ProgramData {
            num_operations: u32::try_from(operations.len()).expect("Too many operations"),
            operations: [],
        };
        self.write_struct(&program);

        // Write operations
        for (gate_type, qubits) in operations {
            // Ensure alignment for OperationData
            let current_pos = self.buffer.len();
            let align = std::mem::align_of::<OperationData>();
            if current_pos % align != 0 {
                let padding = align - (current_pos % align);
                println!("Adding {padding} bytes padding for OperationData alignment");
                self.buffer.extend(std::iter::repeat_n(0, padding));
            }

            // Create and write operation
            let op = OperationData {
                gate_type: *gate_type,
                num_qubits: u32::try_from(qubits.len()).expect("Too many qubits"),
                qubit_indices: [],
            };
            self.write_struct(&op);

            // Write qubit indices
            for &qubit in qubits {
                println!("Writing qubit: {qubit}");
                self.buffer.extend_from_slice(&qubit.to_ne_bytes());
            }
        }

        println!("Program structure:");
        println!("  Total size: {total_size} bytes");
        println!("  Operations: {}", operations.len());
        for (i, (gate, qubits)) in operations.iter().enumerate() {
            println!(
                "  Operation {i}: {:?} with {} qubits: {:?}",
                gate,
                qubits.len(),
                qubits
            );
        }
    }

    /// Calculate program size for operations
    #[must_use]
    pub fn calculate_program_size(operations: &[(GateType, Vec<u32>)]) -> usize {
        let mut size = std::mem::size_of::<ProgramData>();

        for (_gate_type, qubits) in operations {
            // Account for OperationData alignment
            let current = size;
            let align = std::mem::align_of::<OperationData>();
            if current % align != 0 {
                size += align - (current % align);
            }

            // Add operation size
            size += std::mem::size_of::<OperationData>();
            size += qubits.len() * std::mem::size_of::<u32>();
        }

        size
    }

    /// Build final message batch from collected operations
    ///
    /// # Panics
    /// - If buffer is empty
    /// - If first message is not a control message
    /// - If total size exceeds `u32::MAX`
    #[must_use]
    pub fn build(self) -> MessageBatch {
        // Ensure we have at least one message
        assert!(
            !self.buffer.is_empty(),
            "Attempting to build empty message batch"
        );

        // Verify first message is a control message
        let first_header = unsafe { &*self.buffer.as_ptr().cast_aligned::<MessageHeader>() };
        match first_header.msg_type {
            MessageType::Input | MessageType::Halted | MessageType::Error | MessageType::Panic => {}
            _ => panic!(
                "First message must be a control message, got {:?}",
                first_header.msg_type
            ),
        }

        println!("Building batch with buffer size: {}", self.buffer.len());
        println!("\nFirst 128 bytes of batch:");
        for (i, &byte) in self.buffer.iter().take(128).enumerate() {
            if i % 16 == 0 {
                println!();
            }
            print!("{byte:02x} ");
        }
        println!();

        let ptr = self.buffer.as_ptr();
        let size = self.buffer.len();
        let total_size = u32::try_from(size).expect("Batch size exceeds u32::MAX");
        std::mem::forget(self.buffer); // Don't drop the buffer

        MessageBatch {
            total_size,
            data: ptr,
        }
    }
}
