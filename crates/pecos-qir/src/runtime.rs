use log::{debug, info};
use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use pecos_engines::shot_results::{Data, Shot};
use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::io::{self, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// QIR Runtime Implementation
///
/// This file contains the implementation of the QIR runtime functions that are used
/// when executing QIR programs. It defines the C-compatible functions that are called
/// by the QIR program to perform quantum operations.
///
/// # QIR Runtime Library
///
/// This file is a key component of the QIR runtime library, which is built by the
/// `build.rs` script in the pecos-qir crate. The library is pre-built and placed
/// in the target directory to speed up QIR compilation.
///
/// When the QIR compiler runs, it first checks for a pre-built library. If found,
/// it uses that library directly. If not, it falls back to building the runtime
/// on-demand using this file and related files.
///
/// # Implementation Details
///
/// The runtime provides functions for:
/// - Quantum gate operations (H, X, Y, Z, etc.)
/// - Qubit and result allocation/release
/// - Measurement operations
/// - Classical control operations
/// - Logging and message output
///
/// # Safety
///
/// All quantum gate functions are called from C/C++ code and assume that qubit IDs
/// are valid and have been properly allocated. Calling with invalid qubit IDs may
/// lead to undefined behavior.
///
/// Helper function to get the current thread ID as a string
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Helper function to convert i64 to usize for qubit/result IDs
///
/// In QIR, IDs are typically non-negative, but the interface uses i64.
/// This function performs the conversion with appropriate handling.
#[inline]
fn i64_to_usize(value: i64) -> usize {
    // In practice, qubit/result IDs should never be negative in QIR
    // If they are, it indicates a bug in the calling code
    debug_assert!(
        value >= 0,
        "Qubit/result ID should not be negative: {value}"
    );

    // For release builds, clamp negative values to 0 to avoid crashes
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    if value < 0 { 0 } else { value as usize }
}

// Global counters for qubit and result allocation
static NEXT_QUBIT_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_RESULT_ID: AtomicUsize = AtomicUsize::new(0);

// Global cleanup coordination mutex to prevent race conditions during cleanup
static CLEANUP_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

// Global message builder for quantum operations
static MESSAGE_BUILDER: std::sync::LazyLock<Mutex<ByteMessageBuilder>> =
    std::sync::LazyLock::new(|| {
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        Mutex::new(builder)
    });

// Structure to hold runtime state for classical registers
struct RuntimeState {
    // Measurement results by result ID
    measurement_results: HashMap<usize, bool>,
    // Classical registers by name
    classical_registers: HashMap<String, i64>,
    // Track bit positions for each register (register_name -> next_bit_position)
    register_bit_positions: HashMap<String, usize>,
    // Mapping of result IDs to register assignments (result_id -> (register_name, bit_position))
    result_mappings: HashMap<usize, (String, usize)>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            measurement_results: HashMap::new(),
            classical_registers: HashMap::new(),
            register_bit_positions: HashMap::new(),
            result_mappings: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.measurement_results.clear();
        self.classical_registers.clear();
        self.register_bit_positions.clear();
        self.result_mappings.clear();
    }

    fn apply_mappings(&mut self) {
        // Clear existing register values
        self.classical_registers.clear();

        // Apply all result mappings to build register values
        for (result_id, (register_name, bit_position)) in &self.result_mappings {
            // Get the measurement result
            let measurement_value = self
                .measurement_results
                .get(result_id)
                .copied()
                .unwrap_or(false);

            // Get or create the register
            let register = self
                .classical_registers
                .entry(register_name.clone())
                .or_insert(0);

            // Set the bit
            if measurement_value {
                *register |= 1i64 << bit_position;
            } else {
                *register &= !(1i64 << bit_position);
            }
        }
    }

    fn export_shot(&self) -> Shot {
        let mut shot = Shot::default();

        // Export all classical registers to the shot
        for (name, &value) in &self.classical_registers {
            // Store all values as I64 for consistency with QIR standard
            shot.data.insert(name.clone(), Data::I64(value));
        }

        shot
    }
}

// Global runtime state
static RUNTIME_STATE: std::sync::LazyLock<Mutex<RuntimeState>> =
    std::sync::LazyLock::new(|| Mutex::new(RuntimeState::new()));

// Global storage for the last exported shot
static LAST_SHOT: std::sync::LazyLock<Mutex<Option<Shot>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Helper function to check if we should print commands
///
/// This function checks the `QIR_RUNTIME_QUIET` environment variable
/// to determine if commands should be printed to stdout.
///
/// # Returns
///
/// * `true` - If commands should be printed
/// * `false` - If commands should not be printed
fn should_print_commands() -> bool {
    match env::var("QIR_RUNTIME_QUIET") {
        Ok(val) => val != "1",
        Err(_) => true,
    }
}

/// Helper function to store and optionally print quantum gate commands
///
/// This function stores the gate command in the global message builder
/// and optionally prints it to stdout for debugging.
///
/// # Arguments
///
/// * `gate_name` - The name of the gate for debug printing
/// * `add_to_builder` - A closure that adds the gate to the builder
fn store_gate_command<F>(gate_name: &str, add_to_builder: F)
where
    F: FnOnce(&mut ByteMessageBuilder),
{
    let thread_id = get_thread_id();

    // Add the gate to the global message builder
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        add_to_builder(&mut builder);
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
    }

    // Print the command if not in quiet mode
    if should_print_commands() {
        println!("QIR Runtime: [Thread {thread_id}] {gate_name}");
    }
}

// Helper function for single-qubit gates
fn apply_single_qubit_gate(
    gate_name: &str,
    qubit: usize,
    apply_fn: impl FnOnce(&mut ByteMessageBuilder),
) {
    store_gate_command(&format!("{gate_name} {qubit}"), apply_fn);
}

// Helper function for two-qubit gates
fn apply_two_qubit_gate(
    gate_name: &str,
    qubit1: usize,
    qubit2: usize,
    apply_fn: impl FnOnce(&mut ByteMessageBuilder),
) {
    store_gate_command(&format!("{gate_name} {qubit1} {qubit2}"), apply_fn);
}

// Quantum gate operations

/// Applies a rotation around the Z-axis to the specified qubit.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: usize) {
    store_gate_command(&format!("RZ {theta} {qubit}"), |builder| {
        builder.add_rz(theta, &[qubit]);
    });
}

/// Applies a rotation around an axis in the ZY plane to the specified qubit.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `phi` - The phase angle in radians
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: usize) {
    store_gate_command(&format!("R1XY {theta} {phi} {qubit}"), |builder| {
        builder.add_r1xy(theta, phi, &[qubit]);
    });
}

/// Alias for r1xy to match QIR standard naming
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rxy__body(theta: f64, phi: f64, qubit: usize) {
    unsafe {
        __quantum__qis__r1xy__body(theta, phi, qubit);
    }
}

/// Applies a Hadamard gate to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("H", qubit_id, |builder| {
        builder.add_h(&[qubit_id]);
    });
}

/// Applies an X gate to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("X", qubit_id, |builder| {
        builder.add_x(&[qubit_id]);
    });
}

/// Applies a Y gate to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("Y", qubit_id, |builder| {
        builder.add_y(&[qubit_id]);
    });
}

/// Applies a Z gate to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("Z", qubit_id, |builder| {
        builder.add_z(&[qubit_id]);
    });
}

/// Applies a controlled-X gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    apply_two_qubit_gate("CX", control_id, target_id, |builder| {
        builder.add_cx(&[control_id], &[target_id]);
    });
}

/// Applies a controlled-Z gate to the specified qubits.
///
/// This is implemented as a sequence of H, CX, H gates.
///
/// # Arguments
///
/// * `control` - The control qubit index
/// * `target` - The target qubit index
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    // Implement CZ as a sequence of H, CX, H
    store_gate_command(
        &format!("CZ {control_id} {target_id} (as H-CX-H)"),
        |builder| {
            builder.add_h(&[target_id]);
            builder.add_cx(&[control_id], &[target_id]);
            builder.add_h(&[target_id]);
        },
    );
}

/// Applies a SZZ gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__szz__body(qubit1: usize, qubit2: usize) {
    apply_two_qubit_gate("SZZ", qubit1, qubit2, |builder| {
        builder.add_szz(&[qubit1], &[qubit2]);
    });
}

/// Alias for szz to match QIR standard naming
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body(qubit1: usize, qubit2: usize) {
    unsafe {
        __quantum__qis__szz__body(qubit1, qubit2);
    }
}

/// Applies a RZZ gate to the specified qubits.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `qubit1` - The first qubit index
/// * `qubit2` - The second qubit index
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rzz__body(theta: f64, qubit1: usize, qubit2: usize) {
    store_gate_command(&format!("RZZ {theta} {qubit1} {qubit2}"), |builder| {
        builder.add_rzz(theta, &[qubit1], &[qubit2]);
    });
}

/// Measures a qubit and stores the result.
///
/// # Arguments
///
/// * `qubit` - The qubit index to measure
/// * `result` - The result ID to store the measurement result
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID and result ID
/// are valid and have been properly allocated. Calling with invalid IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: i64, result: i64) -> u32 {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    store_gate_command(&format!("M {qubit_id}"), |builder| {
        builder.add_measurements(&[qubit_id]);
    });

    // Store a placeholder measurement result
    // In a real implementation, this would be populated by the quantum engine
    // For now, we'll set it when processing measurement results
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Mark that this result ID is associated with a measurement
        // The actual value will be populated later by process_measurement_results
        state.measurement_results.insert(result_id, false);
    }

    // In the real QIR runtime, this would return the actual measurement result
    // For this implementation, we return 0 (will be updated later)
    0
}

/// Prepares a qubit in the |0⟩ state.
///
/// # Arguments
///
/// * `qubit` - The qubit index to prepare
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__reset__body(qubit: usize) {
    store_gate_command(&format!("PREP {qubit}"), |builder| {
        builder.add_prep(&[qubit]);
    });
}

/// Initialize the quantum runtime.
///
/// This function is called at the beginning of QIR programs to set up the runtime.
///
/// # Arguments
///
/// * `config` - Configuration string (currently unused, can be null)
///
/// # Safety
///
/// This function is called from C/C++ code. The config parameter can be null.
///
/// # Panics
///
/// This function will panic if the `MESSAGE_BUILDER` mutex is poisoned (i.e., if another
/// thread panicked while holding the lock).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__initialize(_config: *const u8) {
    // Reset global state for new program execution
    NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
    NEXT_RESULT_ID.store(0, Ordering::SeqCst);

    // Reset the message builder to clear any existing commands
    let mut builder = MESSAGE_BUILDER.lock().unwrap();
    *builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();

    if should_print_commands() {
        println!("Quantum runtime initialized");
    }
}

/// Allocates a new qubit.
///
/// # Returns
///
/// The ID of the newly allocated qubit
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> usize {
    let qubit_id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Allocated qubit {qubit_id}");
    }

    qubit_id
}

/// Allocates a new result.
///
/// # Returns
///
/// The ID of the newly allocated result
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> usize {
    let result_id = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Allocated result {result_id}");
    }

    result_id
}

/// Releases a qubit.
///
/// # Arguments
///
/// * `qubit` - The qubit ID to release
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit: usize) {
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Released qubit {qubit}");
    }

    // We don't actually do anything with the qubit ID
    // In a real implementation, we would recycle the ID
}

/// Releases a result.
///
/// # Arguments
///
/// * `result` - The result ID to release
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_release(result: usize) {
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Released result {result}");
    }

    // We don't actually do anything with the result ID
    // In a real implementation, we would recycle the ID
}

/// Records a message using Rust logging.
///
/// # Arguments
///
/// * `msg` - The message to record
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__message(msg: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(msg) };
    let msg_str = c_str.to_string_lossy();
    let thread_id = get_thread_id();

    // Use proper Rust logging instead of storing as QuantumCmd
    info!("QIR Message [Thread {}]: {}", thread_id, msg_str);
}

/// Records data.
///
/// # Arguments
///
/// * `data` - The data to record
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__record(data: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(data) };
    let data_str = c_str.to_string_lossy().into_owned();
    let thread_id = get_thread_id();

    // Log the record command
    debug!("QIR Runtime [Thread {}]: Record: {}", thread_id, data_str);

    if should_print_commands() {
        println!("QIR Runtime: [Thread {thread_id}] RECORD: {data_str}");
    }
}

/// Resets the QIR runtime.
///
/// This function clears all commands and measurement results.
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_reset() {
    // Acquire cleanup coordination lock to prevent race conditions
    let _cleanup_guard = match CLEANUP_MUTEX.lock() {
        Ok(guard) => guard,
        Err(_) => {
            eprintln!("QIR: Failed to acquire cleanup mutex during reset");
            return;
        }
    };
    
    let thread_id = get_thread_id();

    // Reset the message builder
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        builder.reset();
        let _ = builder.for_quantum_operations();

        if should_print_commands() {
            println!("[Thread {thread_id}] Reset QIR runtime (reset message builder)");
        }
    } else {
        // If we can't lock the mutex, print an error
        if should_print_commands() {
            eprintln!(
                "[Thread {thread_id}] ERROR: Failed to lock message builder mutex during reset"
            );
            io::stderr().flush().unwrap_or_default();
        }
    }

    // Reset qubit and result counters
    NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
    NEXT_RESULT_ID.store(0, Ordering::SeqCst);

    // Reset runtime state
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        state.reset();
        if should_print_commands() {
            println!("[Thread {thread_id}] Reset runtime state (classical registers cleared)");
        }
    } else if should_print_commands() {
        eprintln!("[Thread {thread_id}] ERROR: Failed to lock runtime state mutex during reset");
    }

    // Clear the last shot
    if let Ok(mut last_shot) = LAST_SHOT.lock() {
        *last_shot = None;
    }

    if should_print_commands() {
        println!("[Thread {thread_id}] Reset QIR runtime (reset counters)");
    }
}

/// Gets the binary commands generated by the QIR runtime as a `ByteMessage`.
///
/// # Returns
///
/// A pointer to a `ByteMessage` containing the commands.
/// The caller is responsible for freeing the `ByteMessage` using `qir_runtime_free_binary_commands`.
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[repr(C)]
pub struct FFIByteData {
    pub data: *mut u32,
    pub word_count: usize,
    pub byte_len: usize,
}

/// # Safety
///
/// This function is unsafe because it returns a raw pointer to allocated memory that must be
/// properly freed by the caller using the appropriate deallocation function. The caller is
/// responsible for ensuring the returned pointer is not used after being freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_binary_commands() -> *mut FFIByteData {
    let thread_id = get_thread_id();

    // Build the message from the global message builder
    let message = if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // Build and return the current message
        builder.build()
    } else {
        // If we can't lock the mutex, return an empty message
        if should_print_commands() {
            eprintln!(
                "[Thread {thread_id}] ERROR: Failed to lock message builder mutex during get_binary_commands"
            );
            io::stderr().flush().unwrap_or_default();
        }
        ByteMessage::create_empty()
    };

    // Extract the aligned data directly from the message
    let bytes = message.into_bytes();
    let byte_len = bytes.len();

    // Transfer aligned u32 data across FFI boundary
    let (data_ptr, word_count) = if byte_len > 0 {
        // Calculate word count (round up)
        let word_count = byte_len.div_ceil(4);

        // Create aligned storage
        let mut aligned_data = vec![0u32; word_count];

        // Copy bytes into aligned storage using bytemuck
        let aligned_bytes = bytemuck::cast_slice_mut::<u32, u8>(&mut aligned_data);
        aligned_bytes[..byte_len].copy_from_slice(&bytes);

        // Convert to raw pointer
        let data_ptr = aligned_data.as_mut_ptr();
        std::mem::forget(aligned_data); // Don't drop, will be freed on other side

        (data_ptr, word_count)
    } else {
        (std::ptr::null_mut(), 0)
    };

    // Create the FFI structure
    let ffi_data = FFIByteData {
        data: data_ptr,
        word_count,
        byte_len,
    };

    // Allocate the FFI structure on the heap
    let boxed_ffi = Box::new(ffi_data);
    let ptr = Box::into_raw(boxed_ffi);

    if should_print_commands() {
        println!(
            "[Thread {thread_id}] Got binary commands as {byte_len} bytes ({word_count} words)"
        );
    }

    ptr
}

/// Frees a `ByteMessage` allocated by `qir_runtime_get_binary_commands`.
///
/// # Arguments
///
/// * `ptr` - The pointer to the `ByteMessage` to free
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_binary_commands(ptr: *mut FFIByteData) {
    let thread_id = get_thread_id();

    if ptr.is_null() {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Attempted to free null FFIByteData pointer");
            io::stderr().flush().unwrap_or_default();
        }
        return;
    }

    // Reconstruct the Box to get the FFIByteData
    let ffi_data = unsafe { Box::from_raw(ptr) };

    // Free the u32 data if it exists
    if !ffi_data.data.is_null() && ffi_data.word_count > 0 {
        // Reconstruct the Vec<u32> to properly deallocate
        let _aligned_data =
            unsafe { Vec::from_raw_parts(ffi_data.data, ffi_data.word_count, ffi_data.word_count) };
        // _aligned_data will be dropped here, properly deallocating the memory
    }

    if should_print_commands() {
        println!(
            "[Thread {thread_id}] Freed FFIByteData with {} bytes ({} words)",
            ffi_data.byte_len, ffi_data.word_count
        );
    }
}

/// Records a result output.
///
/// # Arguments
///
/// * `result` - The result ID to record
/// * `name` - The name to record the result as, or null for default naming
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(result: i64, name: *const c_char) {
    let thread_id = get_thread_id();

    // Generate a name for the result
    let name_str = if name.is_null() {
        // If name is null, use a default name based on the result ID
        format!("result_{result}")
    } else {
        // Convert C string to Rust string
        let c_str = unsafe { CStr::from_ptr(name) };
        c_str.to_string_lossy().into_owned()
    };

    if should_print_commands() {
        println!("[Thread {thread_id}] Recording result {result} as '{name_str}'");
    }

    // Record the mapping of this result to a register and bit position
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Get the next bit position for this register
        let current_bit_position = {
            let bit_position = state
                .register_bit_positions
                .entry(name_str.clone())
                .or_insert(0);
            let pos = *bit_position;
            *bit_position += 1;
            pos
        };

        // Store the mapping for when we get the actual measurement result
        let result_id = i64_to_usize(result);
        state
            .result_mappings
            .insert(result_id, (name_str.clone(), current_bit_position));

        if should_print_commands() {
            println!(
                "[Thread {thread_id}] Mapped result {result} to register '{name_str}' bit {current_bit_position}"
            );
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock runtime state mutex");
    }
}

/// Updates the measurement results in the runtime state.
///
/// This function should be called by the QIR engine after processing measurements
/// from the quantum system.
///
/// # Arguments
///
/// * `results` - A slice of (`result_id`, `measurement_value`) pairs
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_update_measurement_results(
    results_ptr: *const u32,
    results_len: usize,
) {
    let thread_id = get_thread_id();

    if results_ptr.is_null() || results_len == 0 {
        return;
    }

    // Convert the raw pointer to a slice (pairs of result_id, value)
    let results = unsafe { std::slice::from_raw_parts(results_ptr, results_len * 2) };

    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Process pairs of (result_id, measurement_value)
        for i in (0..results.len()).step_by(2) {
            let result_id = results[i] as usize;
            let measurement_value = results[i + 1] != 0;

            state
                .measurement_results
                .insert(result_id, measurement_value);

            if should_print_commands() {
                println!(
                    "[Thread {thread_id}] Updated measurement result {result_id} = {measurement_value}"
                );
            }
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock runtime state mutex");
    }
}

/// Finalizes the QIR program execution and exports the shot results.
///
/// This function should be called when the QIR program's main function returns.
/// It exports the classical registers to a Shot and stores it for retrieval.
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_finalize_shot() {
    let thread_id = get_thread_id();

    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Apply the result mappings to build register values
        state.apply_mappings();

        let shot = state.export_shot();

        if should_print_commands() {
            println!(
                "[Thread {thread_id}] Finalizing shot with {} registers",
                state.classical_registers.len()
            );
            for (name, value) in &state.classical_registers {
                println!("[Thread {thread_id}]   Register '{name}' = {value}");
            }
        }

        // Store the shot for retrieval
        if let Ok(mut last_shot) = LAST_SHOT.lock() {
            *last_shot = Some(shot);
        } else {
            eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock last shot mutex");
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock runtime state mutex");
    }
}

/// Representation of a shot result for FFI
#[repr(C)]
pub struct FFIShotData {
    /// Pointer to register names (null-terminated C strings)
    names: *mut *mut c_char,
    /// Pointer to register values
    values: *mut i64,
    /// Number of registers
    count: usize,
}

/// Gets the shot results from the last finalized execution.
///
/// # Returns
///
/// A pointer to an `FFIShotData` structure containing the shot results,
/// or null if no shot is available.
///
/// # Safety
///
/// This function allocates memory that must be freed by calling `qir_runtime_free_shot_data`.
///
/// # Panics
///
/// This function may panic if:
/// - The array layout cannot be created (e.g., size overflow)
/// - Creating a C string from the register name fails (e.g., contains null bytes)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_shot_results() -> *mut FFIShotData {
    let thread_id = get_thread_id();

    if let Ok(last_shot) = LAST_SHOT.lock() {
        if let Some(shot) = last_shot.as_ref() {
            let count = shot.data.len();

            // Allocate arrays using Vec to ensure proper alignment
            let mut names_vec: Vec<*mut c_char> = Vec::with_capacity(count);
            let names = names_vec.as_mut_ptr();
            std::mem::forget(names_vec); // Prevent deallocation, we'll manage it manually

            let mut values_vec: Vec<i64> = Vec::with_capacity(count);
            let values = values_vec.as_mut_ptr();
            std::mem::forget(values_vec); // Prevent deallocation, we'll manage it manually

            // Populate the arrays
            for (i, (name, data)) in shot.data.iter().enumerate() {
                // Convert name to C string
                let c_name = std::ffi::CString::new(name.as_str()).unwrap();
                unsafe {
                    *names.add(i) = c_name.into_raw();
                }

                // Extract value
                let value = match data {
                    Data::U32(v) => i64::from(*v),
                    Data::I64(v) => *v,
                    _ => 0, // Default for other types
                };
                unsafe {
                    *values.add(i) = value;
                }
            }

            // Create and return the FFI structure
            let ffi_data = Box::new(FFIShotData {
                names,
                values,
                count,
            });

            if should_print_commands() {
                println!("[Thread {thread_id}] Exported shot with {count} registers");
            }

            Box::into_raw(ffi_data)
        } else {
            if should_print_commands() {
                println!("[Thread {thread_id}] No shot results available");
            }
            std::ptr::null_mut()
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock last shot mutex");
        std::ptr::null_mut()
    }
}

/// Frees the shot data allocated by `qir_runtime_get_shot_results`.
///
/// # Arguments
///
/// * `data` - The pointer to the `FFIShotData` to free
///
/// # Safety
///
/// This function should only be called with a valid pointer returned by
/// `qir_runtime_get_shot_results`. Calling with an invalid pointer will
/// result in undefined behavior.
///
/// # Panics
///
/// This function may panic if the array layout cannot be created (e.g., size overflow).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_shot_data(data: *mut FFIShotData) {
    if data.is_null() {
        return;
    }

    unsafe {
        let ffi_data = Box::from_raw(data);

        // Free the name strings
        for i in 0..ffi_data.count {
            let name_ptr = *ffi_data.names.add(i);
            if !name_ptr.is_null() {
                let _ = CString::from_raw(name_ptr);
            }
        }

        // Free the arrays by reconstructing the Vecs
        if ffi_data.count > 0 {
            // Reconstruct Vec to properly deallocate - fix: use correct length parameter
            let _ = Vec::from_raw_parts(ffi_data.names, ffi_data.count, ffi_data.count);
            let _ = Vec::from_raw_parts(ffi_data.values, ffi_data.count, ffi_data.count);
        }

        // Box automatically frees the FFIShotData
    }
}

//
// Pointer-based QIR compatibility functions
//
// These functions provide compatibility with standard QIR format that uses
// %Qubit* and %Result* pointer types. They convert pointer addresses to
// integer IDs and call the integer-based implementations above.
//

/// Convert a qubit pointer to an integer ID
fn qubit_ptr_to_id(qubit_ptr: *const u8) -> i64 {
    qubit_ptr as i64
}

/// Convert a result pointer to an integer ID
fn result_ptr_to_id(result_ptr: *const u8) -> i64 {
    result_ptr as i64
}

// Standard QIR function implementations with pointer interfaces
// These call the existing integer-based implementations

/// Hadamard gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// pointer must be a valid qubit reference from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body__ptr(qubit: *const u8) {
    unsafe {
        __quantum__qis__h__body(qubit_ptr_to_id(qubit));
    }
}

/// X gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// pointer must be a valid qubit reference from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body__ptr(qubit: *const u8) {
    unsafe {
        __quantum__qis__x__body(qubit_ptr_to_id(qubit));
    }
}

/// Y gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// pointer must be a valid qubit reference from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body__ptr(qubit: *const u8) {
    unsafe {
        __quantum__qis__y__body(qubit_ptr_to_id(qubit));
    }
}

/// Z gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// pointer must be a valid qubit reference from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body__ptr(qubit: *const u8) {
    unsafe {
        __quantum__qis__z__body(qubit_ptr_to_id(qubit));
    }
}

/// RZ gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// pointer must be a valid qubit reference from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body__ptr(theta: f64, qubit: *const u8) {
    unsafe {
        __quantum__qis__rz__body(theta, i64_to_usize(qubit_ptr_to_id(qubit)));
    }
}

/// RXY gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// pointer must be a valid qubit reference from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rxy__body__ptr(theta: f64, phi: f64, qubit: *const u8) {
    unsafe {
        __quantum__qis__rxy__body(theta, phi, i64_to_usize(qubit_ptr_to_id(qubit)));
    }
}

/// CX gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both control
/// and target pointers must be valid qubit references from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body__ptr(control: *const u8, target: *const u8) {
    unsafe {
        __quantum__qis__cx__body(qubit_ptr_to_id(control), qubit_ptr_to_id(target));
    }
}

/// CZ gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both control
/// and target pointers must be valid qubit references from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body__ptr(control: *const u8, target: *const u8) {
    unsafe {
        __quantum__qis__cz__body(qubit_ptr_to_id(control), qubit_ptr_to_id(target));
    }
}

/// ZZ gate with pointer interface (Standard QIR format)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both qubit
/// pointers must be valid qubit references from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body__ptr(qubit1: *const u8, qubit2: *const u8) {
    unsafe {
        __quantum__qis__zz__body(
            i64_to_usize(qubit_ptr_to_id(qubit1)),
            i64_to_usize(qubit_ptr_to_id(qubit2)),
        );
    }
}

/// Measurement with pointer interface (void return for standard QIR compatibility)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both qubit
/// and result pointers must be valid references from QIR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body__ptr(qubit: *const u8, result: *const u8) {
    let _measurement_result =
        unsafe { __quantum__qis__m__body(qubit_ptr_to_id(qubit), result_ptr_to_id(result)) };
    // Standard QIR measurement functions return void, not the measurement result
    // The result is stored internally and accessed via result_record_output
}

/// Result recording with pointer interface
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers. The result pointer
/// must be a valid result reference and the name must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output__ptr(
    result: *const u8,
    name: *const c_char,
) {
    unsafe {
        __quantum__rt__result_record_output(result_ptr_to_id(result), name);
    }
}

//
// Standard QIR function names (without suffix) for compatibility with standard QIR files
//
// These functions detect the pointer format and call the appropriate implementation.
// For standard QIR files with %Qubit* parameters, this provides automatic detection.
//

// For the Hadamard gate, we need to overload the function name since it's already defined
// with i64 parameters. We'll use a different approach by providing a C-compatible wrapper.

// Note: The existing __quantum__qis__h__body and other functions already handle integer IDs.
// Standard QIR files use pointer types that encode the qubit ID in the pointer value.
// The engine already includes patterns to detect both formats, so the existing functions
// should work correctly when called from QIR files that use "inttoptr (i64 N to %Qubit*)"
// syntax, as the LLVM JIT will convert these to the appropriate integer values.
