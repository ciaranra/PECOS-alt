use crate::message::ptr::AlignedCast;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MessageType {
    // Control messages (must be first)
    Input = 1,  // Here's input to process
    Halted = 2, // Final result/done
    Error = 3,  // Error occurred
    Panic = 4,  // Unrecoverable error

    // Operation messages
    QuantumOp = 64,
    MeasResult = 65,
}

// Simple header before each message
#[repr(C)]
#[derive(Debug)]
pub struct MessageHeader {
    pub msg_type: MessageType, // u8
    pub payload_size: u16,
}

// Main batch structure
#[repr(C)]
pub struct MessageBatch {
    pub total_size: u32, // Total size of all headers + data
    pub data: *const u8, // Points to [header1][data1][header2][data2]...
}

#[repr(C)]
pub struct QuantumOpData {
    pub gate_type: GateType, // u8
    pub num_qubits: u8,
    pub has_extra_data: bool,
    pub(crate) _pad: u8,
    pub qubit_indices: [u32; 0], // Variable length array
}

impl QuantumOpData {
    // Define constants at the top of impl
    const MAX_QUBIT_INDEX: u32 = 1000;

    /// Validate that the qubit indices are within reasonable bounds for the given gate type
    ///
    /// # Errors
    /// Returns an error if:
    /// - The gate type is unsupported
    /// - The number of qubits doesn't match the gate type
    /// - Any qubit index is too large (>= `MAX_QUBIT_INDEX`)
    /// - The same qubit is used twice in a two-qubit gate
    pub fn validate_qubits(gate_type: GateType, qubits: &[u32]) -> Result<(), &'static str> {
        // Check number of qubits matches gate type
        let expected = match gate_type {
            GateType::H
            | GateType::X
            | GateType::Y
            | GateType::Z
            | GateType::S
            | GateType::T
            | GateType::Measure => 1,
            GateType::CX | GateType::CZ | GateType::SWAP => 2,
            _ => return Err("Unsupported gate type"),
        };

        if qubits.len() != expected {
            return Err("Wrong number of qubits for gate type");
        }

        // Check qubit indices are reasonable
        for &qubit in qubits {
            if qubit >= Self::MAX_QUBIT_INDEX {
                return Err("Qubit index too large");
            }
        }

        // For two-qubit gates, check indices aren't same
        if qubits.len() == 2 && qubits[0] == qubits[1] {
            return Err("Two-qubit gate cannot act on same qubit");
        }

        Ok(())
    }

    /// Validate the raw bytes representing a `QuantumOpData` structure
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The pointer is valid for reads of size bytes
    /// - The memory region is properly aligned
    ///
    /// # Errors
    /// Returns an error if:
    /// - The pointer is not properly aligned
    /// - The buffer is too small
    /// - Any padding bytes are non-zero
    /// - The gate type is invalid
    /// - The number of qubits is invalid
    pub unsafe fn validate_raw(ptr: *const u8, size: usize) -> Result<(), &'static str> {
        unsafe {
            // Check alignment
            if (ptr as usize) % 32 != 0 {
                return Err("QuantumOpData not properly aligned");
            }

            // Check size is sufficient
            if size < std::mem::size_of::<QuantumOpData>() {
                return Err("Buffer too small for QuantumOpData");
            }

            // Validate padding is zero
            for i in 1..4 {
                // Check first padding block
                if *ptr.add(i) != 0 {
                    return Err("Padding bytes not zero");
                }
            }
            for i in 8..32 {
                // Check second padding block
                if *ptr.add(i) != 0 {
                    return Err("Padding bytes not zero");
                }
            }

            // Read and validate gate type
            let gate_type: GateType = std::mem::transmute(*ptr);
            match gate_type {
                GateType::H
                | GateType::X
                | GateType::Y
                | GateType::Z
                | GateType::S
                | GateType::T
                | GateType::CX
                | GateType::CZ
                | GateType::SWAP
                | GateType::Measure => {}
                _ => return Err("Invalid gate type"),
            }

            // Read and validate num_ops
            let num_ops = *(ptr.add(4).cast_aligned::<u32>());
            if num_ops > 2 {
                // No gates use more than 2 qubits
                return Err("Invalid number of qubits");
            }

            Ok(())
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MeasResultData {
    pub qubit: u32,
    pub outcome: bool,
}

#[repr(C)]
pub struct MeasResult {
    pub outcome: bool,
    pub is_deterministic: bool,
}

// Quantum-specific structures
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum GateType {
    // Single-qubit unitaries
    // -- Paulis: binary rep chose so XORing two Paulis gives the right Pauli (up to phases)
    I = 0, // 00
    X = 1, // 01
    Z = 2, // 10
    Y = 3, // 11

    H = 4,
    S = 5,
    T = 6,

    // Two-qubit unitaries
    CX = 7,
    CZ = 8,
    SWAP = 9,

    // Measurements
    Measure = 10,
    // Preps
    Prep = 11,

    CustomGate = 65, // Requires addition data: matrix or some specification
}

#[allow(dead_code)]
#[repr(C)]
pub struct MeasurementData {
    num_results: u16,
    results: [u8; 0], // Bit-packed results
}

#[repr(C)]
pub struct ProgramData {
    pub num_operations: u32,
    pub(crate) operations: [OperationData; 0], // Variable length
}

#[repr(C)]
pub struct OperationData {
    pub gate_type: GateType,
    pub num_qubits: u32,
    pub(crate) qubit_indices: [u32; 0], // Variable length
}

#[repr(C)]
pub struct ProcessingState {
    pub program_offset: u32, // Where we are in program
    pub num_operations: u32, // Total operations in program
}
