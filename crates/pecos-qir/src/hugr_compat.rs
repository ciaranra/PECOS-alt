/// HUGR Compatibility Layer for PECOS QIR Runtime
///
/// This module provides compatibility functions to bridge HUGR-generated LLVM IR
/// with the PECOS QIR runtime. HUGR uses i16 for qubit IDs while PECOS uses usize.
///
/// These functions provide the exact signatures that HUGR-generated code expects.

use crate::runtime;

/// Allocate a qubit (HUGR expects i16 return type)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> i16 {
    let qubit_id = unsafe { runtime::__quantum__rt__qubit_allocate() };
    qubit_id as i16
}

/// Apply Hadamard gate (HUGR expects i16 qubit)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit: i16) {
    unsafe { runtime::__quantum__qis__h__body(qubit as usize) };
}

/// Apply X gate (HUGR expects i16 qubit)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: i16) {
    unsafe { runtime::__quantum__qis__x__body(qubit as usize) };
}

/// Apply Y gate (HUGR expects i16 qubit)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: i16) {
    unsafe { runtime::__quantum__qis__y__body(qubit as usize) };
}

/// Apply Z gate (HUGR expects i16 qubit)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: i16) {
    unsafe { runtime::__quantum__qis__z__body(qubit as usize) };
}

/// Apply CNOT gate (HUGR expects i16 qubits)
/// HUGR uses "cnot" while PECOS uses "cx"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cnot__body(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cx__body(control as usize, target as usize) };
}

/// Apply CX gate (alias for CNOT, HUGR expects i16 qubits)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cx__body(control as usize, target as usize) };
}

/// Apply CY gate (HUGR expects i16 qubits)
/// Note: PECOS doesn't have a native CY, so we implement it as: Y(target) CX(control, target) Y(target)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cy__body(control: i16, target: i16) {
    unsafe { 
        runtime::__quantum__qis__y__body(target as usize);
        runtime::__quantum__qis__cx__body(control as usize, target as usize);
        runtime::__quantum__qis__y__body(target as usize);
    }
}

/// Apply CZ gate (HUGR expects i16 qubits)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cz__body(control as usize, target as usize) };
}

/// Measure qubit in Z basis (HUGR expects i16 qubit and bool return)
/// HUGR uses "mz" while PECOS uses "m"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__mz__body(qubit: i16) -> bool {
    let result_id = unsafe { runtime::__quantum__rt__result_allocate() };
    let measurement = unsafe { runtime::__quantum__qis__m__body(qubit as usize, result_id) };
    
    // PECOS returns 0 or 1 as u32, convert to bool
    measurement != 0
}