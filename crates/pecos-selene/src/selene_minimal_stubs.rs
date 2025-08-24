//! Minimal FFI stub functions that match Selene's exact interface
//! Only includes the essential functions needed by Interface Plugins

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::ffi::c_void;

// Global counters for generating unique IDs
static NEXT_QUBIT_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_RESULT_ID: AtomicU64 = AtomicU64::new(0);
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
pub struct SeleneFutureResult {
    pub error_code: u32,
    pub reference: u64,
}

// Only the essential quantum operations that we know are called
#[unsafe(no_mangle)]
pub extern "C" fn selene_qalloc(_instance: *mut c_void) -> SeleneU64Result {
    println!("=== SELENE STUB: selene_qalloc called with instance={:?} ===", _instance);
    let qubit_id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    println!("=== SELENE STUB: selene_qalloc returning qubit_id={} ===", qubit_id);
    SeleneU64Result { 
        error_code: 0, 
        value: qubit_id 
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qfree(_instance: *mut c_void, q: u64) -> SeleneVoidResult {
    println!("=== SELENE STUB: selene_qfree called with instance={:?}, qubit={} ===", _instance, q);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_reset(_instance: *mut c_void, q: u64) -> SeleneVoidResult {
    println!("=== SELENE STUB: selene_qubit_reset called with instance={:?}, qubit={} ===", _instance, q);
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_lazy_measure(_instance: *mut c_void, q: u64) -> SeleneFutureResult {
    println!("=== SELENE STUB: selene_qubit_lazy_measure called with instance={:?}, qubit={} ===", _instance, q);
    let reference = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    println!("=== SELENE STUB: selene_qubit_lazy_measure returning reference={} ===", reference);
    SeleneFutureResult { 
        error_code: 0, 
        reference 
    }
}

// Note: selene_get_tc and selene_set_tc are provided by selene-core dependency
// #[unsafe(no_mangle)]
// pub extern "C" fn selene_get_tc(_instance: *mut c_void) -> SeleneU64Result {
//     println!("=== SELENE STUB: selene_get_tc called with instance={:?} ===", _instance);
//     let tc = TIME_CURSOR.load(Ordering::SeqCst);
//     SeleneU64Result { 
//         error_code: 0, 
//         value: tc 
//     }
// }

// #[unsafe(no_mangle)]
// pub extern "C" fn selene_set_tc(_instance: *mut c_void, tc: u64) -> SeleneVoidResult {
//     println!("=== SELENE STUB: selene_set_tc called with instance={:?}, tc={} ===", _instance, tc);
//     TIME_CURSOR.store(tc, Ordering::SeqCst);
//     SeleneVoidResult { error_code: 0 }
// }