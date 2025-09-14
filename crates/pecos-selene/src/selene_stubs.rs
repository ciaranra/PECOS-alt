//! Minimal stub functions for Interface Plugin execution
//!
//! These functions prevent the Interface Plugin from hanging by providing
//! the symbols it expects to call. They implement the minimal interface
//! needed to let the Interface Plugin run.

use std::ffi::c_void;

// Result types that match Selene's FFI interface
#[repr(C)]
pub struct U64Result {
    pub error_code: u32,
    pub value: u64,
}

#[repr(C)]
pub struct VoidResult {
    pub error_code: u32,
}

#[repr(C)]
pub struct FutureResult {
    pub error_code: u32,
    pub reference: u64,
}

// Global counter for qubits and results
static mut NEXT_QUBIT_ID: u64 = 0;
static mut NEXT_RESULT_ID: u64 = 0;

// These are the functions the Interface Plugin expects to call
// We provide minimal implementations that allow it to run

#[unsafe(no_mangle)]
pub extern "C" fn selene_qalloc(_instance: *mut c_void) -> U64Result {
    log::trace!("STUB: selene_qalloc called");
    unsafe {
        let qubit_id = NEXT_QUBIT_ID;
        NEXT_QUBIT_ID += 1;
        log::trace!("STUB: selene_qalloc returning qubit_id {}", qubit_id);
        U64Result { error_code: 0, value: qubit_id }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qfree(_instance: *mut c_void, _qubit_id: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_measure(_instance: *mut c_void, qubit_id: u64) -> FutureResult {
    log::trace!("STUB: selene_qubit_measure called with qubit_id {}", qubit_id);
    unsafe {
        let result_id = NEXT_RESULT_ID;
        NEXT_RESULT_ID += 1;
        log::trace!("STUB: selene_qubit_measure returning result_id {}", result_id);
        FutureResult { error_code: 0, reference: result_id }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_lazy_measure(_instance: *mut c_void, qubit_id: u64) -> FutureResult {
    selene_qubit_measure(_instance, qubit_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_lazy_measure_leaked(_instance: *mut c_void, _qubit_id: u64) -> FutureResult {
    unsafe {
        let result_id = NEXT_RESULT_ID;
        NEXT_RESULT_ID += 1;
        FutureResult { error_code: 0, reference: result_id }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_on_shot_start(_instance: *mut c_void, _shot_id: u64, _seed: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_on_shot_end(_instance: *mut c_void) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_exit(_instance: *mut c_void) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_get_current_shot(_instance: *mut c_void) -> U64Result {
    U64Result { error_code: 0, value: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_future_read_bool(_instance: *mut c_void, _future_id: u64) -> u8 {
    // Return alternating values for testing
    static mut COUNTER: u8 = 0;
    unsafe {
        COUNTER = 1 - COUNTER;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_future_read_u64(_instance: *mut c_void, _future_id: u64) -> u64 {
    0
}

// Output functions
#[unsafe(no_mangle)]
pub extern "C" fn selene_print_bool(_instance: *mut c_void, value: u8) -> VoidResult {
    println!("selene_print_bool: {}", value);
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_dump_state(_instance: *mut c_void) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_local_barrier(_instance: *mut c_void, _qubits: *const u64, _n: u64, _sleep_ns: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_custom_runtime_call(_instance: *mut c_void, _tag: u64, _data: *const u8, _len: u64) -> U64Result {
    U64Result { error_code: 1, value: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_get_tc(_instance: *mut c_void) -> U64Result {
    U64Result { error_code: 0, value: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_load_config(_instance: *mut c_void, _key: *const i8) -> U64Result {
    U64Result { error_code: 1, value: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_f64(_instance: *mut c_void, value: f64) -> VoidResult {
    println!("selene_print_f64: {}", value);
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_bool_array(_instance: *mut c_void, _arr: *const u8, _len: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_print_f64_array(_instance: *mut c_void, _arr: *const f64, _len: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}