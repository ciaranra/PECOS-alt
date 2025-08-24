//! FFI bridge that provides Selene functions to Interface Plugins
//!
//! This module provides the selene_* functions that Interface Plugins expect to call.
//! It uses Selene's own infrastructure as much as possible.

use std::sync::Mutex;
use std::ptr;
use std::ffi::c_void;
use selene_core::runtime::RuntimeInterface;
use selene_core::runtime::plugin::RuntimePluginInterface;
use anyhow::Result;

// We need to store the runtime instance globally since FFI functions can't have context
// Instead of Runtime, we'll store a Box<dyn RuntimeInterface>
static RUNTIME_INSTANCE: Mutex<Option<Box<dyn RuntimeInterface>>> = Mutex::new(None);

/// Initialize the FFI bridge with a runtime
pub fn initialize_ffi_bridge(runtime_plugin_path: &std::path::Path, n_qubits: u64) -> Result<()> {
    // Load the runtime plugin
    let runtime_plugin = RuntimePluginInterface::new_from_file(runtime_plugin_path)?;
    
    // Create runtime instance using the plugin factory
    use std::sync::Arc;
    let factory = Arc::new(runtime_plugin);
    let runtime = factory.init(
        n_qubits,
        selene_core::time::Instant::from(0),
        &Vec::<String>::new(),
    )?;
    
    // Store it globally
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    *guard = Some(runtime);
    
    Ok(())
}

/// Clear the runtime instance
pub fn cleanup_ffi_bridge() {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    *guard = None;
}

// FFI Result types matching Selene's conventions
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

// Provide the selene_* functions that Interface Plugins expect

#[no_mangle]
pub extern "C" fn selene_qalloc(_instance: *mut c_void) -> U64Result {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.qalloc() {
            Ok(qubit_id) => U64Result { error_code: 0, value: qubit_id },
            Err(e) => {
                log::error!("qalloc failed: {}", e);
                U64Result { error_code: 1, value: 0 }
            }
        }
    } else {
        log::error!("No runtime instance for qalloc");
        U64Result { error_code: 1, value: 0 }
    }
}

#[no_mangle]
pub extern "C" fn selene_qfree(_instance: *mut c_void, qubit_id: u64) -> VoidResult {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.qfree(qubit_id) {
            Ok(()) => VoidResult { error_code: 0 },
            Err(e) => {
                log::error!("qfree failed: {}", e);
                VoidResult { error_code: 1 }
            }
        }
    } else {
        log::error!("No runtime instance for qfree");
        VoidResult { error_code: 1 }
    }
}

#[no_mangle]
pub extern "C" fn selene_qubit_measure(_instance: *mut c_void, qubit_id: u64) -> FutureResult {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.measure(qubit_id) {
            Ok(result_id) => FutureResult { error_code: 0, reference: result_id },
            Err(e) => {
                log::error!("measure failed: {}", e);
                FutureResult { error_code: 1, reference: 0 }
            }
        }
    } else {
        log::error!("No runtime instance for measure");
        FutureResult { error_code: 1, reference: 0 }
    }
}

#[no_mangle]
pub extern "C" fn selene_qubit_lazy_measure(_instance: *mut c_void, qubit_id: u64) -> FutureResult {
    // For now, just forward to regular measure
    selene_qubit_measure(_instance, qubit_id)
}

#[no_mangle]
pub extern "C" fn selene_qubit_lazy_measure_leaked(_instance: *mut c_void, qubit_id: u64) -> FutureResult {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.measure_leaked(qubit_id) {
            Ok(result_id) => FutureResult { error_code: 0, reference: result_id },
            Err(e) => {
                log::error!("measure_leaked failed: {}", e);
                FutureResult { error_code: 1, reference: 0 }
            }
        }
    } else {
        log::error!("No runtime instance for measure_leaked");
        FutureResult { error_code: 1, reference: 0 }
    }
}

#[no_mangle]
pub extern "C" fn selene_on_shot_start(_instance: *mut c_void, shot_id: u64, seed: u64) -> VoidResult {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.shot_start(shot_id, seed) {
            Ok(()) => VoidResult { error_code: 0 },
            Err(e) => {
                log::error!("shot_start failed: {}", e);
                VoidResult { error_code: 1 }
            }
        }
    } else {
        log::error!("No runtime instance for shot_start");
        VoidResult { error_code: 1 }
    }
}

#[no_mangle]
pub extern "C" fn selene_on_shot_end(_instance: *mut c_void) -> VoidResult {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.shot_end() {
            Ok(()) => VoidResult { error_code: 0 },
            Err(e) => {
                log::error!("shot_end failed: {}", e);
                VoidResult { error_code: 1 }
            }
        }
    } else {
        log::error!("No runtime instance for shot_end");
        VoidResult { error_code: 1 }
    }
}

#[no_mangle]
pub extern "C" fn selene_exit(_instance: *mut c_void) -> VoidResult {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.exit() {
            Ok(()) => VoidResult { error_code: 0 },
            Err(e) => {
                log::error!("exit failed: {}", e);
                VoidResult { error_code: 1 }
            }
        }
    } else {
        log::error!("No runtime instance for exit");
        VoidResult { error_code: 1 }
    }
}

// Stub implementations for other functions the Interface Plugin might call
#[no_mangle]
pub extern "C" fn selene_get_current_shot(_instance: *mut c_void) -> U64Result {
    U64Result { error_code: 0, value: 0 }
}

#[no_mangle]
pub extern "C" fn selene_future_read_bool(_instance: *mut c_void, future_id: u64) -> u8 {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        // Force the result
        let _ = runtime.force_result(future_id);
        
        // Get the result
        match runtime.get_bool_result(future_id) {
            Ok(Some(value)) => value as u8,
            _ => 0
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn selene_future_read_u64(_instance: *mut c_void, future_id: u64) -> u64 {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        // Force the result
        let _ = runtime.force_result(future_id);
        
        // Get the result
        match runtime.get_u64_result(future_id) {
            Ok(Some(value)) => value,
            _ => 0
        }
    } else {
        0
    }
}

// More stub functions - these are needed for linking but won't be called in our simple tests
#[no_mangle]
pub extern "C" fn selene_print_bool(_instance: *mut c_void, _value: u8) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[no_mangle]
pub extern "C" fn selene_dump_state(_instance: *mut c_void) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[no_mangle]
pub extern "C" fn selene_local_barrier(_instance: *mut c_void, _qubits: *const u64, _n: u64, _sleep_ns: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}

// Additional stubs for functions we saw in the undefined symbols
#[no_mangle]
pub extern "C" fn selene_custom_runtime_call(_instance: *mut c_void, _tag: u64, _data: *const u8, _len: u64) -> U64Result {
    U64Result { error_code: 1, value: 0 }
}

#[no_mangle]
pub extern "C" fn selene_get_tc(_instance: *mut c_void) -> U64Result {
    U64Result { error_code: 0, value: 0 }
}

#[no_mangle]
pub extern "C" fn selene_load_config(_instance: *mut c_void, _key: *const i8) -> U64Result {
    U64Result { error_code: 1, value: 0 }
}

#[no_mangle]
pub extern "C" fn selene_print_f64(_instance: *mut c_void, _value: f64) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[no_mangle]
pub extern "C" fn selene_print_bool_array(_instance: *mut c_void, _arr: *const u8, _len: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}

#[no_mangle]
pub extern "C" fn selene_print_f64_array(_instance: *mut c_void, _arr: *const f64, _len: u64) -> VoidResult {
    VoidResult { error_code: 0 }
}