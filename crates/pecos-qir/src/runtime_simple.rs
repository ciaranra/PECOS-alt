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
}

/// Helper function to check if command printing is enabled
fn should_print_commands() -> bool {
    env::var("QIR_PRINT_COMMANDS").unwrap_or_default() == "1"
}

/// Pointer conversion functions for QIR convention
fn qubit_ptr_to_id(qubit_ptr: *const u8) -> Result<usize, String> {
    // In proper QIR, pointers are used as direct qubit indices via inttoptr
    let qubit_id = qubit_ptr as usize;
    Ok(qubit_id)
}

fn result_ptr_to_id(result_ptr: *const u8) -> Result<usize, String> {
    // Similar to qubit pointers
    let result_id = result_ptr as usize;
    Ok(result_id)
}

// =============================================================================
// QIR Runtime API Functions
// =============================================================================

/// Reset the QIR runtime state
#[no_mangle]
pub unsafe extern "C" fn qir_runtime_reset() {
    let thread_id = get_thread_id();
    
    if should_print_commands() {
        println!("[Thread {thread_id}] QIR Runtime Reset");
    }

    // Reset global counters
    NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
    NEXT_RESULT_ID.store(0, Ordering::SeqCst);

    // Reset message builder
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        *builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
    }

    // Reset runtime state
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        state.reset();
    }

    // Clear last shot
    if let Ok(mut shot) = LAST_SHOT.lock() {
        *shot = None;
    }

    if should_print_commands() {
        println!("[Thread {thread_id}] QIR Runtime Reset Complete");
    }
}

/// Initialize the QIR runtime
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__initialize(_config: *const u8) {
    let thread_id = get_thread_id();
    
    if should_print_commands() {
        println!("[Thread {thread_id}] QIR Runtime Initialize");
    }
    
    // Simple initialization - just print that we're ready
    println!("Quantum runtime initialized");
}

/// Allocate a new qubit (integer version for HUGR)
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> usize {
    let id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Allocated qubit {id}");
    }
    
    id
}

/// Allocate a new result (integer version for HUGR)
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> usize {
    let id = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Allocated result {id}");
    }
    
    id
}

/// Allocate a new qubit (pointer version for QIR)
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate_ptr() -> *const u8 {
    let id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Allocated qubit pointer {id}");
    }
    
    id as *const u8
}

/// Allocate a new result (pointer version for QIR)
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__result_allocate_ptr() -> *const u8 {
    let id = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Allocated result pointer {id}");
    }
    
    id as *const u8
}

/// Release a qubit (integer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit: usize) {
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Released qubit {qubit}");
    }
}

/// Release a result (integer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__rt__result_release(result: usize) {
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] Released result {result}");
    }
}

// =============================================================================
// Quantum Gate Operations (Dual Convention Support)
// =============================================================================

/// Hadamard gate (QIR pointer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit_ptr: *const u8) {
    let thread_id = get_thread_id();
    
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => {
            if should_print_commands() {
                println!("[Thread {thread_id}] H gate on qubit {qubit_id}");
            }
            
            if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                let _ = builder.h(qubit_id);
            }
        }
        Err(e) => {
            if should_print_commands() {
                eprintln!("[Thread {thread_id}] ERROR: H gate failed: {e}");
            }
        }
    }
}

/// Hadamard gate (HUGR integer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__h__body__hugr(qubit: i64) {
    let thread_id = get_thread_id();
    let qubit_id = i64_to_usize(qubit);
    
    if should_print_commands() {
        println!("[Thread {thread_id}] H gate (HUGR) on qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.h(qubit_id);
    }
}

/// X gate (QIR pointer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit_ptr: *const u8) {
    let thread_id = get_thread_id();
    
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => {
            if should_print_commands() {
                println!("[Thread {thread_id}] X gate on qubit {qubit_id}");
            }
            
            if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                let _ = builder.x(qubit_id);
            }
        }
        Err(e) => {
            if should_print_commands() {
                eprintln!("[Thread {thread_id}] ERROR: X gate failed: {e}");
            }
        }
    }
}

/// X gate (HUGR integer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__x__body__hugr(qubit: i64) {
    let thread_id = get_thread_id();
    let qubit_id = i64_to_usize(qubit);
    
    if should_print_commands() {
        println!("[Thread {thread_id}] X gate (HUGR) on qubit {qubit_id}");
    }
    
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        let _ = builder.x(qubit_id);
    }
}

// Add other quantum gates following the same pattern...
// (Y, Z, CX, etc.) - keeping this short for now

/// Measurement (QIR pointer version)
#[no_mangle]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: *const u8, result: *const u8) {
    let thread_id = get_thread_id();
    
    match (qubit_ptr_to_id(qubit), result_ptr_to_id(result)) {
        (Ok(qubit_id), Ok(result_id)) => {
            if should_print_commands() {
                println!("[Thread {thread_id}] Measuring qubit {qubit_id} -> result {result_id}");
            }
            
            if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                let _ = builder.m(qubit_id, result_id);
            }
        }
        (Err(e), _) | (_, Err(e)) => {
            if should_print_commands() {
                eprintln!("[Thread {thread_id}] ERROR: Measurement failed: {e}");
            }
        }
    }
}

/// Get binary commands for execution
#[repr(C)]
pub struct FFIByteData {
    pub data: *mut u32,
    pub word_count: usize,
    pub byte_len: usize,
}

#[no_mangle]
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
#[no_mangle]
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