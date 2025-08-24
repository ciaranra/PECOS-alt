//! Bridge between Interface Plugin and Runtime Plugin
//!
//! This module provides the FFI functions that the Interface Plugin expects to call.
//! It forwards these calls to our runtime implementation.

use std::sync::Mutex;
use selene_core::runtime::{RuntimeInterface, Operation, BatchOperation};
use anyhow::Result;

// Global runtime instance that will handle the operations
static RUNTIME_INSTANCE: Mutex<Option<Box<dyn RuntimeInterface>>> = Mutex::new(None);

/// Set the runtime instance that will handle operations
pub fn set_runtime_instance(runtime: Box<dyn RuntimeInterface>) {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    *guard = Some(runtime);
}

/// Clear the runtime instance
pub fn clear_runtime_instance() {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    *guard = None;
}

// FFI functions that the Interface Plugin will call
// These need to match the signatures that the Interface Plugin expects

#[no_mangle]
pub extern "C" fn __quantum__qis__qalloc__body() -> u64 {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.qalloc() {
            Ok(qubit_id) => qubit_id,
            Err(e) => {
                log::error!("qalloc failed: {}", e);
                u64::MAX // Indicates failure
            }
        }
    } else {
        log::error!("No runtime instance available for qalloc");
        u64::MAX
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__qfree__body(qubit_id: u64) {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        if let Err(e) = runtime.qfree(qubit_id) {
            log::error!("qfree failed: {}", e);
        }
    } else {
        log::error!("No runtime instance available for qfree");
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__h__body(qubit_id: u64) {
    // Hadamard gate - convert to RXY(pi/2, 0) followed by RZ(pi)
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        // H = RY(pi/2) * RZ(pi) = RXY(pi/2, pi/2) * RZ(pi)
        if let Err(e) = runtime.rxy_gate(qubit_id, std::f64::consts::PI / 2.0, std::f64::consts::PI / 2.0) {
            log::error!("H gate (RXY part) failed: {}", e);
        }
        if let Err(e) = runtime.rz_gate(qubit_id, std::f64::consts::PI) {
            log::error!("H gate (RZ part) failed: {}", e);
        }
    } else {
        log::error!("No runtime instance available for H gate");
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__x__body(qubit_id: u64) {
    // X gate - RXY(pi, 0)
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        if let Err(e) = runtime.rxy_gate(qubit_id, std::f64::consts::PI, 0.0) {
            log::error!("X gate failed: {}", e);
        }
    } else {
        log::error!("No runtime instance available for X gate");
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__y__body(qubit_id: u64) {
    // Y gate - RXY(pi, pi/2)
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        if let Err(e) = runtime.rxy_gate(qubit_id, std::f64::consts::PI, std::f64::consts::PI / 2.0) {
            log::error!("Y gate failed: {}", e);
        }
    } else {
        log::error!("No runtime instance available for Y gate");
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__z__body(qubit_id: u64) {
    // Z gate - RZ(pi)
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        if let Err(e) = runtime.rz_gate(qubit_id, std::f64::consts::PI) {
            log::error!("Z gate failed: {}", e);
        }
    } else {
        log::error!("No runtime instance available for Z gate");
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__cnot__body(control: u64, target: u64) {
    // CNOT gate - this needs to be decomposed or handled specially
    // For now, log it as unsupported
    log::warn!("CNOT gate not yet implemented in runtime bridge: control={}, target={}", control, target);
}

#[no_mangle]
pub extern "C" fn __quantum__qis__cx__body(control: u64, target: u64) {
    // CX is same as CNOT
    __quantum__qis__cnot__body(control, target);
}

#[no_mangle]
pub extern "C" fn __quantum__qis__measure__body(qubit_id: u64) -> u64 {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        match runtime.measure(qubit_id) {
            Ok(result_id) => result_id,
            Err(e) => {
                log::error!("Measure failed: {}", e);
                u64::MAX
            }
        }
    } else {
        log::error!("No runtime instance available for measure");
        u64::MAX
    }
}

#[no_mangle]
pub extern "C" fn __quantum__qis__reset__body(qubit_id: u64) {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        if let Err(e) = runtime.reset(qubit_id) {
            log::error!("Reset failed: {}", e);
        }
    } else {
        log::error!("No runtime instance available for reset");
    }
}

// Result handling functions
#[no_mangle]
pub extern "C" fn __quantum__qis__read_result__body(result_id: u64) -> bool {
    let mut guard = RUNTIME_INSTANCE.lock().unwrap();
    if let Some(runtime) = guard.as_mut() {
        // First force the result to be available
        if let Err(e) = runtime.force_result(result_id) {
            log::error!("Force result failed: {}", e);
        }
        
        // Then get the result
        match runtime.get_bool_result(result_id) {
            Ok(Some(value)) => value,
            Ok(None) => {
                log::warn!("Result {} not yet available", result_id);
                false
            }
            Err(e) => {
                log::error!("Get result failed: {}", e);
                false
            }
        }
    } else {
        log::error!("No runtime instance available for read_result");
        false
    }
}

// Output functions for results
#[no_mangle]
pub extern "C" fn __quantum__qis__result_record_output(result_id: u64, tag: *const i8) {
    // This would output the result with a tag
    // For now, just log it
    let tag_str = if tag.is_null() {
        "unnamed"
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(tag)
                .to_str()
                .unwrap_or("invalid")
        }
    };
    
    let value = __quantum__qis__read_result__body(result_id);
    log::info!("Result output: {} = {}", tag_str, value as i32);
}