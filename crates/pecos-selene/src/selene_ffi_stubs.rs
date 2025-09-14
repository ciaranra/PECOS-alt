//! FFI stub functions that match Selene's exact interface
//! These are compiled directly into the Rust library and exported with C linkage
//! so that Interface Plugins can find and call them.

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::ffi::c_void;

// Global counters for generating unique IDs
static NEXT_QUBIT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_RESULT_ID: AtomicU64 = AtomicU64::new(0);
static CURRENT_SHOT: AtomicU64 = AtomicU64::new(0);
static TIME_CURSOR: AtomicU64 = AtomicU64::new(0);

// Exact type definitions matching Selene's C header
#[repr(C)]
pub struct SeleneU64Result {
    pub error_code: u32,
    pub value: u64,
}

#[repr(C)]
pub struct SeleneVoidResult {
    pub error_code: u32,
}

#[repr(C)]
pub struct SeleneBoolResult {
    pub error_code: u32,
    pub value: bool,
}

#[repr(C)]
pub struct SeleneFutureResult {
    pub error_code: u32,
    pub reference: u64,
}

#[repr(C)]
pub struct SeleneF64Result {
    pub error_code: u32,
    pub value: f64,
}

#[repr(C)]
pub struct SeleneU32Result {
    pub error_code: u32,
    pub value: u32,
}

#[repr(C)]
pub struct SeleneString {
    pub data: *const i8,
    pub length: u64,
    pub owned: bool,
}

// Core quantum operations - matching exact Selene signatures
#[unsafe(no_mangle)]
pub extern "C" fn selene_qalloc(_instance: *mut c_void) -> SeleneU64Result {
    log::trace!("SELENE STUB: selene_qalloc called with instance={:?} ===", _instance);
    let qubit_id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    log::trace!("SELENE STUB: selene_qalloc returning qubit_id={} ===", qubit_id);
    SeleneU64Result {
        error_code: 0,
        value: qubit_id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qfree(_instance: *mut c_void, q: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_qfree called with instance={:?}, qubit={} ===", _instance, q);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_measure(_instance: *mut c_void, q: u64) -> SeleneBoolResult {
    log::trace!("SELENE STUB: selene_qubit_measure called with instance={:?}, qubit={} ===", _instance, q);
    // Return alternating values for testing
    static COUNTER: AtomicBool = AtomicBool::new(false);
    let result = !COUNTER.load(Ordering::SeqCst);
    COUNTER.store(result, Ordering::SeqCst);
    log::trace!("SELENE STUB: selene_qubit_measure returning {} ===", result);
    SeleneBoolResult {
        error_code: 0,
        value: result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_lazy_measure(_instance: *mut c_void, q: u64) -> SeleneFutureResult {
    log::trace!("SELENE STUB: selene_qubit_lazy_measure called with instance={:?}, qubit={} ===", _instance, q);
    let reference = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    log::trace!("SELENE STUB: selene_qubit_lazy_measure returning reference={} ===", reference);
    SeleneFutureResult {
        error_code: 0,
        reference
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_lazy_measure_leaked(_instance: *mut c_void, q: u64) -> SeleneFutureResult {
    log::trace!("SELENE STUB: selene_qubit_lazy_measure_leaked called with instance={:?}, qubit={} ===", _instance, q);
    let reference = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    SeleneFutureResult {
        error_code: 0,
        reference
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_reset(_instance: *mut c_void, q: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_qubit_reset called with instance={:?}, qubit={} ===", _instance, q);
    SeleneVoidResult { error_code: 0 }
}

// Future reading functions
#[unsafe(no_mangle)]
pub extern "C" fn selene_future_read_bool(_instance: *mut c_void, r: u64) -> SeleneBoolResult {
    log::trace!("SELENE STUB: selene_future_read_bool called with instance={:?}, reference={} ===", _instance, r);
    // Return alternating values for testing
    static COUNTER: AtomicBool = AtomicBool::new(false);
    let result = !COUNTER.load(Ordering::SeqCst);
    COUNTER.store(result, Ordering::SeqCst);
    log::trace!("SELENE STUB: selene_future_read_bool returning {} ===", result);
    SeleneBoolResult {
        error_code: 0,
        value: result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_future_read_u64(_instance: *mut c_void, r: u64) -> SeleneU64Result {
    log::trace!("SELENE STUB: selene_future_read_u64 called with instance={:?}, reference={} ===", _instance, r);
    SeleneU64Result {
        error_code: 0,
        value: 42
    }
}

// Gate operations
#[unsafe(no_mangle)]
pub extern "C" fn selene_rxy(_instance: *mut c_void, qubit_id: u64, theta: f64, phi: f64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_rxy called with instance={:?}, qubit={}, theta={}, phi={} ===",
             _instance, qubit_id, theta, phi);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_rz(_instance: *mut c_void, qubit_id: u64, theta: f64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_rz called with instance={:?}, qubit={}, theta={} ===",
             _instance, qubit_id, theta);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_rzz(_instance: *mut c_void, qubit_id: u64, qubit_id2: u64, theta: f64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_rzz called with instance={:?}, qubit1={}, qubit2={}, theta={} ===",
             _instance, qubit_id, qubit_id2, theta);
    SeleneVoidResult { error_code: 0 }
}

// Shot management
#[unsafe(no_mangle)]
pub extern "C" fn selene_on_shot_start(_instance: *mut c_void, shot_index: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_on_shot_start called with instance={:?}, shot_index={} ===",
             _instance, shot_index);
    CURRENT_SHOT.store(shot_index, Ordering::SeqCst);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_on_shot_end(_instance: *mut c_void) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_on_shot_end called with instance={:?} ===", _instance);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_get_current_shot(_instance: *mut c_void) -> SeleneU64Result {
    log::trace!("SELENE STUB: selene_get_current_shot called with instance={:?} ===", _instance);
    let shot = CURRENT_SHOT.load(Ordering::SeqCst);
    SeleneU64Result {
        error_code: 0,
        value: shot
    }
}

// Exit/cleanup
#[unsafe(no_mangle)]
pub extern "C" fn selene_exit(_instance: *mut c_void) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_exit called with instance={:?} ===", _instance);
    SeleneVoidResult { error_code: 0 }
}

// Time cursor
#[unsafe(no_mangle)]
pub extern "C" fn selene_get_tc(_instance: *mut c_void) -> SeleneU64Result {
    log::trace!("SELENE STUB: selene_get_tc called with instance={:?} ===", _instance);
    let tc = TIME_CURSOR.load(Ordering::SeqCst);
    SeleneU64Result {
        error_code: 0,
        value: tc
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_set_tc(_instance: *mut c_void, tc: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_set_tc called with instance={:?}, tc={} ===", _instance, tc);
    TIME_CURSOR.store(tc, Ordering::SeqCst);
    SeleneVoidResult { error_code: 0 }
}

// Barrier operations
#[unsafe(no_mangle)]
pub extern "C" fn selene_local_barrier(_instance: *mut c_void, qubit_ids: *const u64, qubit_ids_length: u64, sleep_time: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_local_barrier called with instance={:?}, num_qubits={}, sleep_time={} ===",
             _instance, qubit_ids_length, sleep_time);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_global_barrier(_instance: *mut c_void, sleep_time: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_global_barrier called with instance={:?}, sleep_time={} ===",
             _instance, sleep_time);
    SeleneVoidResult { error_code: 0 }
}

// Print functions
#[unsafe(no_mangle)]
pub extern "C" fn selene_print_bool(_instance: *mut c_void, tag: SeleneString, value: bool) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_bool called with instance={:?}, value={} ===",
             _instance, value);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_f64(_instance: *mut c_void, tag: SeleneString, value: f64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_f64 called with instance={:?}, value={} ===", _instance, value);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_u64(_instance: *mut c_void, tag: SeleneString, value: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_u64 called with instance={:?}, value={} ===", _instance, value);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_i64(_instance: *mut c_void, tag: SeleneString, value: i64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_i64 called with instance={:?}, value={} ===", _instance, value);
    SeleneVoidResult { error_code: 0 }
}

// Array print functions
#[unsafe(no_mangle)]
pub extern "C" fn selene_print_bool_array(_instance: *mut c_void, tag: SeleneString, ptr: *const bool, length: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_bool_array called with instance={:?}, length={} ===", _instance, length);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_f64_array(_instance: *mut c_void, tag: SeleneString, ptr: *const f64, length: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_f64_array called with instance={:?}, length={} ===", _instance, length);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_u64_array(_instance: *mut c_void, tag: SeleneString, ptr: *const u64, length: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_u64_array called with instance={:?}, length={} ===", _instance, length);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_i64_array(_instance: *mut c_void, tag: SeleneString, ptr: *const i64, length: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_print_i64_array called with instance={:?}, length={} ===", _instance, length);
    SeleneVoidResult { error_code: 0 }
}

// State dump
#[unsafe(no_mangle)]
pub extern "C" fn selene_dump_state(_instance: *mut c_void, message: SeleneString, qubits: *const u64, qubits_length: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_dump_state called with instance={:?}, num_qubits={} ===", _instance, qubits_length);
    SeleneVoidResult { error_code: 0 }
}

// Random number generation
#[unsafe(no_mangle)]
pub extern "C" fn selene_random_seed(_instance: *mut c_void, seed: u64) -> SeleneVoidResult {
    log::trace!("SELENE STUB: selene_random_seed called with instance={:?}, seed={} ===", _instance, seed);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_random_u32(_instance: *mut c_void) -> SeleneU32Result {
    log::trace!("SELENE STUB: selene_random_u32 called with instance={:?} ===", _instance);
    SeleneU32Result {
        error_code: 0,
        value: 12345
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_random_f64(_instance: *mut c_void) -> SeleneF64Result {
    log::trace!("SELENE STUB: selene_random_f64 called with instance={:?} ===", _instance);
    SeleneF64Result {
        error_code: 0,
        value: 0.5
    }
}

// Note: selene_custom_runtime_call is provided by selene-core dependency