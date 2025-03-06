use lazy_static::lazy_static;
use log::{debug, trace};
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::Mutex;

use crate::channels::ByteMessage;
use pecos_core::types::{GateType, QuantumCommand};

lazy_static! {
    // A thread-safe global queue to store quantum commands
    static ref COMMAND_QUEUE: Mutex<VecDeque<QuantumCommand>> = Mutex::new(VecDeque::new());
}

/// Represents a quantum measurement result.
///
/// This struct is an opaque placeholder, as the internal details of a measurement
/// result are not meant to be exposed. Instead, it is used as a reference in
/// quantum runtime functions to store and manage measurement outcomes.
#[repr(C)]
pub struct Result {
    _private: [u8; 0],
}

/// Represents a quantum bit (qubit) in the quantum system.
///
/// This structure is defined as an empty opaque struct to prevent users from
/// directly manipulating qubit internals. Instead, it is intended to be used
/// as a pointer in function calls within the quantum runtime.
#[repr(C)]
pub struct Qubit {
    _private: [u8; 0],
}

/// Represents the RZ rotation gate on the specified qubit and queues it for execution.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians.
/// * `qubit` - A pointer to the qubit on which the RZ gate will be applied.
///
/// # Panics
///
/// This function will panic if:
/// - The `qubit` pointer is invalid or cannot be converted to a valid index.
/// - The global `COMMAND_QUEUE` mutex is poisoned.
///
/// # Safety
///
/// The `qubit` pointer must be valid and not null. Behavior is undefined if this condition is not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: *const Qubit) {
    let qubit_idx = usize::try_from(qubit as u64).expect("Invalid RZ qubit pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        let cmd = QuantumCommand {
            gate: GateType::RZ { theta },
            qubits: vec![qubit_idx],
        };
        trace!("Queueing RZ gate: {:?}", cmd);
        queue.push_back(cmd);
    }
}

/// Represents the R1XY rotation gate on the specified qubit and queues it for execution.
///
/// # Arguments
///
/// * `phi` - The azimuthal angle in radians.
/// * `theta` - The polar angle in radians.
/// * `qubit` - A pointer to the qubit on which the R1XY gate will be applied.
///
/// # Panics
///
/// This function will panic if:
/// - The `qubit` pointer is invalid or cannot be converted to a valid index.
/// - The global `COMMAND_QUEUE` mutex is poisoned.
///
/// # Safety
///
/// The `qubit` pointer must be valid and not null. Behavior is undefined if this condition is not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__rxy__body(phi: f64, theta: f64, qubit: *const Qubit) {
    let qubit_idx = usize::try_from(qubit as u64).expect("Invalid R1XY qubit pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        let cmd = QuantumCommand {
            gate: GateType::R1XY { phi, theta },
            qubits: vec![qubit_idx],
        };
        trace!("Queueing R1XY gate: {:?}", cmd);
        queue.push_back(cmd);
    }
}

/// Represents the SZZ gate applied to two specified qubits and queues it for execution.
///
/// # Arguments
///
/// * `qubit1` - A pointer to the first qubit.
/// * `qubit2` - A pointer to the second qubit.
///
/// # Panics
///
/// This function will panic if:
/// - The `qubit1` or `qubit2` pointer is invalid or cannot be converted to a valid index.
/// - The global `COMMAND_QUEUE` mutex is poisoned.
///
/// # Safety
///
/// Both `qubit1` and `qubit2` pointers must be valid and not null. Undefined behavior may occur if these conditions are not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__zz__body(qubit1: *const Qubit, qubit2: *const Qubit) {
    let qubit1_idx = usize::try_from(qubit1 as u64).expect("Invalid ZZ qubit1 pointer");
    let qubit2_idx = usize::try_from(qubit2 as u64).expect("Invalid ZZ qubit2 pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        let cmd = QuantumCommand {
            gate: GateType::SZZ,
            qubits: vec![qubit1_idx, qubit2_idx],
        };
        trace!("Queueing SZZ gate: {:?}", cmd);
        queue.push_back(cmd);
    }
}

/// Applies a Hadamard (H) gate to the specified qubit and queues it for execution.
///
/// # Arguments
///
/// * `qubit` - A pointer to the qubit on which the H gate will be applied.
///
/// # Panics
///
/// This function will panic if:
/// - The `qubit` pointer is invalid or cannot be converted to a valid index.
/// - The global `COMMAND_QUEUE` mutex is poisoned.
///
/// # Safety
///
/// The `qubit` pointer must be valid and not null. Behavior is undefined if this condition is not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__h__body(qubit: *const Qubit) {
    let qubit_idx = usize::try_from(qubit as u64).expect("Invalid H qubit pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        let cmd = QuantumCommand {
            gate: GateType::H,
            qubits: vec![qubit_idx],
        };
        trace!("Queueing H gate: {:?}", cmd);
        queue.push_back(cmd);
    }
}

/// Applies a controlled-X (CX) gate to the specified qubits and queues it for execution.
///
/// # Arguments
///
/// * `control` - A pointer to the control qubit.
/// * `target` - A pointer to the target qubit.
///
/// # Panics
///
/// This function will panic if:
/// - The `control` or `target` pointers are invalid or cannot be converted to valid indices.
/// - The global `COMMAND_QUEUE` mutex is poisoned.
///
/// # Safety
///
/// Both `control` and `target` pointers must be valid and not null. Undefined behavior may occur if these conditions are not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__cx__body(control: *const Qubit, target: *const Qubit) {
    let control_idx = usize::try_from(control as u64).expect("Invalid CX control pointer");
    let target_idx = usize::try_from(target as u64).expect("Invalid CX target pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        let cmd = QuantumCommand {
            gate: GateType::CX,
            qubits: vec![control_idx, target_idx],
        };
        trace!("Queueing CX gate: {:?}", cmd);
        queue.push_back(cmd);
    }
}

/// Queues a measurement operation on the specified qubit and associates it with a result.
///
/// # Arguments
///
/// * `qubit` - A pointer to the qubit to be measured. The pointer must be valid and not null.
/// * `result` - A pointer to the Result structure that will store the measurement result. The pointer must be valid and not null.
///
/// # Panics
///
/// This function will panic if:
/// - The `qubit` or `result` pointers are invalid or cannot be converted to valid indices.
/// - The global `COMMAND_QUEUE` mutex is poisoned.
///
/// # Safety
///
/// Both `qubit` and `result` pointers must be valid and not null. Undefined behavior may occur if these conditions are not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__m__body(qubit: *const Qubit, result: *const Result) {
    let qubit_idx = usize::try_from(qubit as u64).expect("Invalid Measurement qubit pointer");
    let result_idx = usize::try_from(result as u64).expect("Invalid Measurement result pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        let cmd = QuantumCommand {
            gate: GateType::Measure {
                result_id: result_idx,
            },
            qubits: vec![qubit_idx],
        };
        trace!("Queueing measurement: {:?}", cmd);
        queue.push_back(cmd);
    }
}

/// Records the result of a quantum measurement and outputs it.
///
/// This function finalizes the current quantum operations by flushing the command queue.
/// It processes any pending commands by sending them through the byte protocol,
/// waits for the measurement result, and then associates the provided result pointer
/// with the parsed measurement.
///
/// # Arguments
///
/// * `result` - A pointer to the `Result` structure where the measurement result will be stored.
///              This pointer must be valid and non-null.
/// * `_label` - A pointer to a null-terminated C-style string representing an optional label for
///              the result (currently unused in this implementation).
///
/// # Behavior
///
/// 1. Flushes the `COMMAND_QUEUE` by sending queued commands through the byte protocol.
/// 2. Waits for a measurement result from the input stream.
/// 3. Associates the parsed measurement result with the given `result` pointer.
///
/// # Panics
///
/// This function will panic if:
/// - The `result` pointer is invalid or cannot be converted to a valid index.
/// - The queue mutex (`COMMAND_QUEUE`) is poisoned.
///
/// # Errors
///
/// - If the received measurement result is invalid or cannot be parsed, an error will be logged.
///
/// # Safety
///
/// The `result` pointer must be valid and not null. Undefined behavior may occur if this
/// condition is not met.
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__rt__result_record_output(result: *const Result, _label: *const i8) {
    let result_idx = usize::try_from(result as u64).expect("Invalid result pointer");

    if let Ok(mut queue) = COMMAND_QUEUE.lock() {
        if !queue.is_empty() {
            debug!("Flushing {} commands", queue.len());

            // Convert queue to Vec<QuantumCommand>
            let commands: Vec<QuantumCommand> = queue.drain(..).collect();

            // Create ByteMessage using the builder pattern
            let message = ByteMessage::builder()
                .add_quantum_commands(&commands)
                .build();

            // Write to stdout
            io::stdout().write_all(message.as_bytes()).unwrap();
            io::stdout().flush().unwrap();
        }

        // Read binary response
        let mut header_buffer =
            [0u8; std::mem::size_of::<crate::channels::byte::protocol::BatchHeader>()];
        io::stdin().read_exact(&mut header_buffer).unwrap();

        // Parse the binary message to get the measurement result
        // This is a simplified version - in a real implementation, you'd need to
        // properly parse the full binary message format
        let mut measurement_buffer = [0u8; 4]; // Assuming a 32-bit measurement
        io::stdin().read_exact(&mut measurement_buffer).unwrap();
        let measurement = u32::from_le_bytes(measurement_buffer);

        debug!("Received measurement: {}", measurement);

        // Create a ByteMessage for the measurement result using the builder pattern
        // UPDATED to use add_measurement_results instead of add_measurement_result
        let result_id = u32::try_from(result_idx).expect("Problem converting result id to u32");
        let result_message = ByteMessage::builder()
            .add_measurement_results(&[measurement as usize], &[result_id as usize])
            .build();

        io::stdout().write_all(result_message.as_bytes()).unwrap();
        io::stdout().flush().unwrap();
    }
}
