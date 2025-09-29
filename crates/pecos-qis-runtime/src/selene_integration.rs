//! Selene Runtime Integration
//!
//! This module provides integration with Selene runtime plugins when the
//! `selene-integration` feature is enabled. It forwards QIS calls to
//! external `selene_runtime_*` functions instead of building ByteMessages directly.

// External functions that should be provided by the Selene bridge
// These are implemented in pecos-selene-ccengine/src/ffi_bridge.rs
unsafe extern "C" {
    fn selene_runtime_qalloc(result: *mut u64) -> i32;
    fn selene_runtime_qfree(qubit_id: u64) -> i32;
    fn selene_runtime_rxy_gate(qubit_id: u64, theta: f64, phi: f64) -> i32;
    fn selene_runtime_rz_gate(qubit_id: u64, theta: f64) -> i32;
    fn selene_runtime_rzz_gate(qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> i32;
    fn selene_runtime_measure(qubit_id: u64, result: *mut u64) -> i32;
    fn selene_runtime_reset(qubit_id: u64) -> i32;
    fn selene_runtime_get_bool_result(result_id: u64, value: *mut bool) -> i32;
    fn selene_runtime_set_bool_result(result_id: u64, value: bool) -> i32;
}


/// Allocate a qubit through Selene runtime
pub fn allocate_qubit() -> Result<usize, String> {
    let mut qubit_id: u64 = 0;
    let result = unsafe { selene_runtime_qalloc(&mut qubit_id) };
    if result == 0 {
        Ok(qubit_id as usize)
    } else {
        Err(format!("Selene runtime qalloc failed with code {}", result))
    }
}

/// Free a qubit through Selene runtime
pub fn free_qubit(qubit_id: usize) -> Result<(), String> {
    let result = unsafe { selene_runtime_qfree(qubit_id as u64) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Selene runtime qfree failed with code {}", result))
    }
}

/// Apply RXY gate through Selene runtime
pub fn rxy_gate(qubit_id: usize, theta: f64, phi: f64) -> Result<(), String> {
    let result = unsafe { selene_runtime_rxy_gate(qubit_id as u64, theta, phi) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Selene runtime RXY gate failed with code {}", result))
    }
}

/// Apply RZ gate through Selene runtime
pub fn rz_gate(qubit_id: usize, theta: f64) -> Result<(), String> {
    let result = unsafe { selene_runtime_rz_gate(qubit_id as u64, theta) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Selene runtime RZ gate failed with code {}", result))
    }
}

/// Apply RZZ gate through Selene runtime
pub fn rzz_gate(qubit_id_1: usize, qubit_id_2: usize, theta: f64) -> Result<(), String> {
    let result = unsafe { selene_runtime_rzz_gate(qubit_id_1 as u64, qubit_id_2 as u64, theta) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Selene runtime RZZ gate failed with code {}", result))
    }
}

/// Measure a qubit through Selene runtime
pub fn measure_qubit(qubit_id: usize) -> Result<usize, String> {
    let mut result_id: u64 = 0;
    let result = unsafe { selene_runtime_measure(qubit_id as u64, &mut result_id) };
    if result == 0 {
        Ok(result_id as usize)
    } else {
        Err(format!("Selene runtime measure failed with code {}", result))
    }
}

/// Reset a qubit through Selene runtime
pub fn reset_qubit(qubit_id: usize) -> Result<(), String> {
    let result = unsafe { selene_runtime_reset(qubit_id as u64) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Selene runtime reset failed with code {}", result))
    }
}

/// Get a boolean measurement result through Selene runtime
pub fn get_bool_result(result_id: usize) -> Result<bool, String> {
    let mut value: bool = false;
    let result = unsafe { selene_runtime_get_bool_result(result_id as u64, &mut value) };
    if result == 0 {
        Ok(value)
    } else {
        Err(format!("Selene runtime get_bool_result failed with code {}", result))
    }
}

/// Set a boolean measurement result through Selene runtime
pub fn set_bool_result(result_id: usize, value: bool) -> Result<(), String> {
    let result = unsafe { selene_runtime_set_bool_result(result_id as u64, value) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("Selene runtime set_bool_result failed with code {}", result))
    }
}