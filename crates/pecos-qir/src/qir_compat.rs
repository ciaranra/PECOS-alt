/// HUGR Compatibility Layer for PECOS QIR Runtime
///
/// This module provides HUGR-specific function variants that use alternative naming
/// conventions. The standard QIR functions are already defined in runtime.rs.
///
/// HUGR may generate calls to functions with different names or signatures.
/// This module provides those alternative entry points.

use crate::runtime;

/// HUGR-specific qubit allocation (returns i16 instead of usize)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__rt__qubit_allocate() -> i16 {
    let qubit_id = unsafe { runtime::__quantum__rt__qubit_allocate() };
    qubit_id as i16
}

/// HUGR-specific Hadamard gate with i16 parameter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__h(qubit: i16) {
    unsafe { runtime::__quantum__qis__h__body(qubit as usize) };
}

/// HUGR-specific X gate with i16 parameter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__x(qubit: i16) {
    unsafe { runtime::__quantum__qis__x__body(qubit as usize) };
}

/// HUGR-specific Y gate with i16 parameter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__y(qubit: i16) {
    unsafe { runtime::__quantum__qis__y__body(qubit as usize) };
}

/// HUGR-specific Z gate with i16 parameter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__z(qubit: i16) {
    unsafe { runtime::__quantum__qis__z__body(qubit as usize) };
}

/// HUGR-specific CNOT gate with i16 parameters
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cnot(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cx__body(control as usize, target as usize) };
}

/// HUGR-specific CX gate with i16 parameters (alias for cnot)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cx(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cx__body(control as usize, target as usize) };
}

/// HUGR-specific CY gate with i16 parameters
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cy(control: i16, target: i16) {
    unsafe { 
        runtime::__quantum__qis__y__body(target as usize);
        runtime::__quantum__qis__cx__body(control as usize, target as usize);
        runtime::__quantum__qis__y__body(target as usize);
    }
}

/// HUGR-specific CZ gate with i16 parameters
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cz(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cz__body(control as usize, target as usize) };
}

/// HUGR-specific measurement with i16 parameter and bool return
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__measure(qubit: i16) -> bool {
    let result_id = unsafe { runtime::__quantum__rt__result_allocate() };
    let measurement = unsafe { runtime::__quantum__qis__m__body(qubit as usize, result_id) };
    
    // PECOS returns 0 or 1 as u32, convert to bool
    measurement != 0
}