//! FFI Bridge for QIS to Selene Runtime
//!
//! This module provides the selene_runtime_* functions that QIS programs
//! (linked with pecos-qis-runtime) can call. These forward to the actual
//! runtime plugin.

use crate::runtime_plugin::RuntimeInstance;
use std::sync::Mutex;

// Global state for the current runtime instance
// In a real implementation, this would be thread-local or passed differently
static RUNTIME_INSTANCE: Mutex<Option<RuntimeInstance>> = Mutex::new(None);
static RUNTIME_PLUGIN: Mutex<Option<std::sync::Arc<crate::runtime_plugin::RuntimePlugin>>> = Mutex::new(None);

/// Set the current runtime instance and plugin
pub fn set_runtime(instance: RuntimeInstance, plugin: std::sync::Arc<crate::runtime_plugin::RuntimePlugin>) {
    let mut inst = RUNTIME_INSTANCE.lock().unwrap();
    *inst = Some(instance);

    let mut plug = RUNTIME_PLUGIN.lock().unwrap();
    *plug = Some(plugin);
}

/// Setup global bridge from engine (for testing)
pub fn setup_global_bridge(_engine: &crate::engine::SeleneClassicalControlEngine) -> Result<(), String> {
    // For now, this is a placeholder since we need access to the engine's bridge
    // In a real implementation, we'd need to extract the runtime instance and plugin
    // from the engine and set them up globally
    println!("Setting up global bridge for QIS program execution...");

    // TODO: Extract runtime instance from engine and call set_runtime()
    // This would require exposing the bridge from the engine

    Ok(())
}

/// Clear the runtime instance
pub fn clear_runtime() {
    let mut inst = RUNTIME_INSTANCE.lock().unwrap();
    *inst = None;

    let mut plug = RUNTIME_PLUGIN.lock().unwrap();
    *plug = None;
}

// ===== Selene Runtime FFI Functions =====
// These are called by QIS programs via pecos-qis-runtime

/// Allocate a qubit
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_qalloc(result: *mut u64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.qalloc(*instance) {
            Ok(qubit_id) => {
                unsafe { *result = qubit_id };
                0
            }
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Free a qubit
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_qfree(qubit_id: u64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.qfree(*instance, qubit_id) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Apply RXY gate
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_rxy_gate(qubit_id: u64, theta: f64, phi: f64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.rxy_gate(*instance, qubit_id, theta, phi) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Apply RZ gate
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_rz_gate(qubit_id: u64, theta: f64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.rz_gate(*instance, qubit_id, theta) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Apply RZZ gate
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_rzz_gate(qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.rzz_gate(*instance, qubit_id_1, qubit_id_2, theta) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Measure a qubit
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_measure(qubit_id: u64, result: *mut u64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.measure(*instance, qubit_id) {
            Ok(result_id) => {
                unsafe { *result = result_id };
                0
            }
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Reset a qubit
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_reset(qubit_id: u64) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.reset(*instance, qubit_id) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Get a boolean result
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_get_bool_result(result_id: u64, value: *mut bool) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.get_bool_result(*instance, result_id) {
            Ok(Some(val)) => {
                unsafe { *value = val };
                0
            }
            Ok(None) => -1, // Result not ready
            Err(_) => -1,
        }
    } else {
        -1
    }
}

/// Set a boolean result
#[unsafe(no_mangle)]
pub extern "C" fn selene_runtime_set_bool_result(result_id: u64, value: bool) -> i32 {
    let inst = RUNTIME_INSTANCE.lock().unwrap();
    let plug = RUNTIME_PLUGIN.lock().unwrap();

    if let (Some(instance), Some(plugin)) = (inst.as_ref(), plug.as_ref()) {
        match plugin.set_bool_result(*instance, result_id, value) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}