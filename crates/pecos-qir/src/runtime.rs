use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use pecos_engines::shot_results::{Data, Shot};
use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::io::{self, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Simplified QIR Runtime Implementation
///
/// This is a simplified version that removes the complex context system
/// and should have much better performance.

/// Helper function to get the current thread ID as a string
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Helper function to convert i64 to usize for qubit/result IDs
#[inline]
fn i64_to_usize(value: i64) -> usize {
    value as usize
}

// Simple global counters for qubit and result allocation
static NEXT_QUBIT_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_RESULT_ID: AtomicUsize = AtomicUsize::new(0);

// Simple global state
use std::sync::LazyLock;

static MESSAGE_BUILDER: LazyLock<Mutex<ByteMessageBuilder>> =
    LazyLock::new(|| {
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        Mutex::new(builder)
    });

static RUNTIME_STATE: LazyLock<Mutex<RuntimeState>> =
    LazyLock::new(|| Mutex::new(RuntimeState::new()));

static LAST_SHOT: LazyLock<Mutex<Option<Shot>>> =
    LazyLock::new(|| Mutex::new(None));

// Simple structure to hold runtime state
struct RuntimeState {
    measurement_results: HashMap<usize, bool>,
    classical_registers: HashMap<String, i64>,
    register_bit_positions: HashMap<String, usize>,
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

/// Removed pointer conversion functions - no longer needed with direct usize parameters

// =============================================================================
// QIR Runtime API Functions
// =============================================================================

/// Reset the QIR runtime state
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_reset() {
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
}

/// Initialize the QIR runtime
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__initialize(_config: *const u8) {
    // Reset global state for new program execution
    NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
    NEXT_RESULT_ID.store(0, Ordering::SeqCst);

    // Reset the message builder to clear any existing commands
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        *builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
    }

    if should_print_commands() {
        println!("Quantum runtime initialized");
    }
}


/// Standard QIR qubit allocation - returns pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> *const u8 {
    let id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Allocated qubit {id}");
    }
    
    id as *const u8
}

/// Standard QIR result allocation - returns pointer  
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> *const u8 {
    let id = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Allocated result {id}");
    }
    
    id as *const u8
}

/// Release a qubit (pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit_ptr: *const u8) {
    let qubit_id = qubit_ptr as usize;
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Released qubit {qubit_id}");
    }
}

/// Release a result (pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_release(result_ptr: *const u8) {
    let result_id = result_ptr as usize;
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Released result {result_id}");
    }
}

// =============================================================================
// Quantum Gate Operations (Dual Convention Support)
// =============================================================================

/// Hadamard gate (QIR pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit_ptr: *const u8) {
    let thread_id = get_thread_id();
    let qubit_id = qubit_ptr as usize;
    
    if should_print_commands() {
        println!("[Thread {thread_id}] H gate on qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_h(&[qubit_id]);
    }
}

/// Hadamard gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body__hugr(qubit: i64) {
    let thread_id = get_thread_id();
    let qubit_id = i64_to_usize(qubit);
    
    if should_print_commands() {
        println!("[Thread {thread_id}] H gate (HUGR) on qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_h(&[qubit_id]);
    }
}

/// X gate (QIR pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit_ptr: *const u8) {
    let thread_id = get_thread_id();
    let qubit_id = qubit_ptr as usize;
    
    if should_print_commands() {
        println!("[Thread {thread_id}] X gate on qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_x(&[qubit_id]);
    }
}

/// X gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body__hugr(qubit: i64) {
    let thread_id = get_thread_id();
    let qubit_id = i64_to_usize(qubit);
    
    if should_print_commands() {
        println!("[Thread {thread_id}] X gate (HUGR) on qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_x(&[qubit_id]);
    }
}

/// Y gate (QIR standard version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_y(&[qubit]);
    }
}

/// Y gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_y(&[qubit_id]);
    }
}

/// Z gate (QIR standard version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_z(&[qubit]);
    }
}

/// Z gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_z(&[qubit_id]);
    }
}

/// CX gate (QIR pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control_ptr: *const u8, target_ptr: *const u8) {
    let control_id = control_ptr as usize;
    let target_id = target_ptr as usize;
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_cx(&[control_id], &[target_id]);
    }
}

/// CX gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body__hugr(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_cx(&[control_id], &[target_id]);
    }
}

/// CZ gate (QIR version with usize parameters)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body_usize(control: usize, target: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_cz(&[control], &[target]);
    }
}

/// RZ gate (QIR standard version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_rz(theta, &[qubit]);
    }
}

/// RZ gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body__hugr(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_rz(theta, &[qubit_id]);
    }
}

/// R1XY gate (QIR standard version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_r1xy(theta, phi, &[qubit]);
    }
}

/// R1XY gate (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body__hugr(theta: f64, phi: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_r1xy(theta, phi, &[qubit_id]);
    }
}

/// RXY gate (QIR standard version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rxy__body(theta: f64, phi: f64, qubit: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_r1xy(theta, phi, &[qubit]);
    }
}

/// Additional gates for integer types
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cy__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_cy(&[control_id], &[target_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_cz(&[control_id], &[target_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ch__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // CH implemented as H on target, then CX
        let _ = builder.add_h(&[target_id]);
        let _ = builder.add_cx(&[control_id], &[target_id]);
        let _ = builder.add_h(&[target_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__s__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_sz(&[qubit_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__sdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_szdg(&[qubit_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__t__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_t(&[qubit_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__tdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_tdg(&[qubit_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rx__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_rx(theta, &[qubit_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ry__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_ry(theta, &[qubit_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__crz__body(theta: f64, control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // CRZ implemented as CX-RZ-CX sequence
        let _ = builder.add_cx(&[control_id], &[target_id]);
        let _ = builder.add_rz(theta / 2.0, &[target_id]);
        let _ = builder.add_cx(&[control_id], &[target_id]);
        let _ = builder.add_rz(-theta / 2.0, &[target_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ccx__body(control1: i64, control2: i64, target: i64) {
    let control1_id = i64_to_usize(control1);
    let control2_id = i64_to_usize(control2);
    let target_id = i64_to_usize(target);
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // CCX (Toffoli) - simplified implementation
        let _ = builder.add_h(&[target_id]);
        let _ = builder.add_cx(&[control2_id], &[target_id]);
        let _ = builder.add_tdg(&[target_id]);
        let _ = builder.add_cx(&[control1_id], &[target_id]);
        let _ = builder.add_t(&[target_id]);
        let _ = builder.add_cx(&[control2_id], &[target_id]);
        let _ = builder.add_tdg(&[target_id]);
        let _ = builder.add_cx(&[control1_id], &[target_id]);
        let _ = builder.add_t(&[control2_id]);
        let _ = builder.add_t(&[target_id]);
        let _ = builder.add_cx(&[control1_id], &[control2_id]);
        let _ = builder.add_t(&[control1_id]);
        let _ = builder.add_tdg(&[control2_id]);
        let _ = builder.add_cx(&[control1_id], &[control2_id]);
        let _ = builder.add_h(&[target_id]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__szz__body(qubit1: usize, qubit2: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_szz(&[qubit1], &[qubit2]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body(qubit1: usize, qubit2: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // ZZ gate implementation using CZ
        let _ = builder.add_cz(&[qubit1], &[qubit2]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rzz__body(theta: f64, qubit1: usize, qubit2: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_rzz(theta, &[qubit1], &[qubit2]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__reset__body(qubit: usize) {
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // Reset implemented as preparation
        let _ = builder.add_prep(&[qubit]);
    }
}

/// Measurement (HUGR integer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body_i64(qubit: i64, result: i64) -> u32 {
    let qubit_id = i64_to_usize(qubit);
    let _result_id = i64_to_usize(result);
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_measurements(&[qubit_id]);
    }
    
    // Return a dummy measurement result
    0
}

/// Measurement (HUGR convention)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__qis__m__body(qubit: i64, result: i64) {
    let qubit_id = i64_to_usize(qubit);
    let _result_id = i64_to_usize(result);
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_measurements(&[qubit_id]);
    }
}

/// Measurement (QIR pointer version) - DEPRECATED: kept for legacy compatibility
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body_ptr(qubit_ptr: *const u8, result_ptr: *const u8) {
    let thread_id = get_thread_id();
    let qubit_id = qubit_ptr as usize;
    let _result_id = result_ptr as usize;
    
    if should_print_commands() {
        println!("[Thread {thread_id}] Measuring qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_measurements(&[qubit_id]);
    }
}

/// Measurement (HUGR convention with integer parameters) - Primary implementation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: i64, result: i64) -> i32 {
    let thread_id = get_thread_id();
    let qubit_id = qubit as usize;
    let _result_id = result as usize;
    
    if should_print_commands() {
        println!("[Thread {thread_id}] Measuring qubit {qubit_id} (HUGR convention)");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.add_measurements(&[qubit_id]);
    }
    
    // Return 0 for |0⟩ state, 1 for |1⟩ state (simulated)
    // In a real implementation, this would be the actual measurement result
    0
}

/// Get binary commands for execution
#[repr(C)]
pub struct FFIByteData {
    pub data: *mut u32,
    pub word_count: usize,
    pub byte_len: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_binary_commands() -> *mut FFIByteData {
    let thread_id = get_thread_id();

    let message = if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        builder.build()
    } else {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Failed to lock message builder");
        }
        ByteMessage::create_empty()
    };

    let bytes = message.into_bytes();
    let byte_len = bytes.len();

    let (data_ptr, word_count) = if byte_len > 0 {
        let word_count = byte_len.div_ceil(4);
        let mut aligned_data = vec![0u32; word_count];
        let aligned_bytes = bytemuck::cast_slice_mut::<u32, u8>(&mut aligned_data);
        aligned_bytes[..byte_len].copy_from_slice(&bytes);
        let data_ptr = aligned_data.as_mut_ptr();
        std::mem::forget(aligned_data);
        (data_ptr, word_count)
    } else {
        (std::ptr::null_mut(), 0)
    };

    let ffi_data = FFIByteData {
        data: data_ptr,
        word_count,
        byte_len,
    };

    let boxed_ffi = Box::new(ffi_data);
    let ptr = Box::into_raw(boxed_ffi);

    if should_print_commands() {
        println!("[Thread {thread_id}] Got binary commands: {byte_len} bytes ({word_count} words)");
    }

    ptr
}

/// Free binary commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_binary_commands(ptr: *mut FFIByteData) {
    let thread_id = get_thread_id();

    if ptr.is_null() {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Attempted to free null FFIByteData pointer");
        }
        return;
    }

    let ffi_data = unsafe { Box::from_raw(ptr) };

    if !ffi_data.data.is_null() && ffi_data.word_count > 0 {
        let _aligned_data = unsafe { 
            Vec::from_raw_parts(ffi_data.data, ffi_data.word_count, ffi_data.word_count) 
        };
    }

    if should_print_commands() {
        println!("[Thread {thread_id}] Freed FFIByteData");
    }
}

/// Record a result output
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(result_ptr: *const u8, name: *const c_char) {
    let thread_id = get_thread_id();
    let result_id = result_ptr as usize;
    
    let name_str = if name.is_null() {
        format!("result_{result_id}")
    } else {
        let c_str = unsafe { CStr::from_ptr(name) };
        c_str.to_string_lossy().into_owned()
    };

    if should_print_commands() {
        println!("[Thread {thread_id}] Recording result {result_id} as '{name_str}'");
    }

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
        state
            .result_mappings
            .insert(result_id, (name_str.clone(), current_bit_position));

        if should_print_commands() {
            println!(
                "[Thread {thread_id}] Mapped result {result_id} to register '{name_str}' bit {current_bit_position}"
            );
        }
    }
}

/// Message printing
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__message(msg: *const c_char) {
    if !msg.is_null() {
        let c_str = unsafe { CStr::from_ptr(msg) };
        if let Ok(rust_str) = c_str.to_str() {
            println!("QIR Message: {rust_str}");
        }
    }
}

/// Record data
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__record(data: *const c_char) {
    if !data.is_null() {
        let c_str = unsafe { CStr::from_ptr(data) };
        if let Ok(rust_str) = c_str.to_str() {
            if should_print_commands() {
                println!("QIR Record: {rust_str}");
            }
        }
    }
}

/// Update measurement results
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_update_measurement_results(
    results_ptr: *const u32,
    results_len: usize,
) {
    let thread_id = get_thread_id();
    
    if results_ptr.is_null() || results_len == 0 {
        if should_print_commands() {
            println!("[Thread {thread_id}] No measurement results to update");
        }
        return;
    }

    // Convert the raw pointer to a slice (pairs of result_id, value)
    // results_len is already the number of pairs, so total length is results_len * 2
    let results = unsafe { std::slice::from_raw_parts(results_ptr, results_len * 2) };
    
    if should_print_commands() {
        println!("[Thread {thread_id}] Updating {results_len} measurement results");
    }

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
    }
}

/// Finalize shot
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_finalize_shot() {
    let thread_id = get_thread_id();
    
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Apply mappings to calculate final register values
        state.apply_mappings();
        
        // Create shot using the original working approach
        let shot = state.export_shot();
        
        if should_print_commands() {
            println!("[Thread {thread_id}] Finalized shot with {} registers", state.classical_registers.len());
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

/// Get shot results
#[repr(C)]
pub struct FFIShotData {
    pub names: *mut *mut c_char,
    pub values: *mut i64,
    pub count: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_shot_results() -> *mut FFIShotData {
    let thread_id = get_thread_id();

    if let Ok(last_shot) = LAST_SHOT.lock() {
        if let Some(shot) = last_shot.as_ref() {
            let count = shot.data.len();

            if count == 0 {
                // Return null for empty shots
                return std::ptr::null_mut();
            }

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

            // Create and return the FFI struct
            let ffi_data = FFIShotData {
                names,
                values,
                count,
            };

            let boxed_ffi = Box::new(ffi_data);
            let ptr = Box::into_raw(boxed_ffi);

            if should_print_commands() {
                println!("[Thread {thread_id}] Got shot results: {count} registers");
            }

            return ptr;
        }
    }

    // Return null if no shot available
    std::ptr::null_mut()
}

/// Free shot data
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_shot_data(data: *mut FFIShotData) {
    let thread_id = get_thread_id();

    if data.is_null() {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Attempted to free null FFIShotData pointer");
        }
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
            // Reconstruct Vec to properly deallocate
            let _ = Vec::from_raw_parts(ffi_data.names, 0, ffi_data.count);
            let _ = Vec::from_raw_parts(ffi_data.values, 0, ffi_data.count);
        }

        if should_print_commands() {
            println!("[Thread {thread_id}] Freed FFIShotData");
        }

        // Box automatically frees the FFIShotData
    }
}