/// HUGR Compatibility Layer for PECOS QIR Runtime
///
/// This module provides HUGR-specific function variants that use alternative naming
/// conventions. The standard QIR functions are already defined in runtime.rs.
///
/// HUGR may generate calls to functions with different names or signatures.
/// This module provides those alternative entry points.
use crate::runtime;

/// HUGR-specific qubit allocation (returns i16 instead of usize)
///
/// # Safety
/// This function is unsafe because it allocates quantum resources and must be
/// properly managed to avoid resource leaks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__rt__qubit_allocate() -> i16 {
    let qubit_id = unsafe { runtime::__quantum__rt__qubit_allocate() };
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        qubit_id as i16
    }
}

/// HUGR-specific Hadamard gate with i16 parameter
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// parameter must be a valid allocated qubit ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__h(qubit: i16) {
    unsafe { runtime::__quantum__qis__h__body__hugr(i64::from(qubit)) };
}

/// HUGR-specific X gate with i16 parameter
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// parameter must be a valid allocated qubit ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__x(qubit: i16) {
    unsafe { runtime::__quantum__qis__x__body__hugr(i64::from(qubit)) };
}

/// HUGR-specific Y gate with i16 parameter
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// parameter must be a valid allocated qubit ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__y(qubit: i16) {
    unsafe { runtime::__quantum__qis__y__body__hugr(i64::from(qubit)) };
}

/// HUGR-specific Z gate with i16 parameter
///
/// # Safety
/// This function is unsafe because it operates on quantum state and the qubit
/// parameter must be a valid allocated qubit ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__z(qubit: i16) {
    unsafe { runtime::__quantum__qis__z__body__hugr(i64::from(qubit)) };
}

/// HUGR-specific CNOT gate with i16 parameters
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both control
/// and target parameters must be valid allocated qubit IDs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cnot(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cx__body__hugr(i64::from(control), i64::from(target)) };
}

/// HUGR-specific CX gate with i16 parameters (alias for cnot)
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both control
/// and target parameters must be valid allocated qubit IDs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cx(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cx__body__hugr(i64::from(control), i64::from(target)) };
}

/// HUGR-specific CY gate with i16 parameters
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both control
/// and target parameters must be valid allocated qubit IDs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cy(control: i16, target: i16) {
    unsafe {
        runtime::__quantum__qis__y__body__hugr(i64::from(target));
        runtime::__quantum__qis__cx__body__hugr(i64::from(control), i64::from(target));
        runtime::__quantum__qis__y__body__hugr(i64::from(target));
    }
}

/// HUGR-specific CZ gate with i16 parameters
///
/// # Safety
/// This function is unsafe because it operates on quantum state and both control
/// and target parameters must be valid allocated qubit IDs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__cz(control: i16, target: i16) {
    unsafe { runtime::__quantum__qis__cz__body(i64::from(control), i64::from(target)) };
}

/// HUGR-specific measurement with i16 parameter
///
/// # Safety
/// This function is unsafe because it operates on quantum state, allocates result
/// resources, and the qubit parameter must be a valid allocated qubit ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__measure(qubit: i16) {
    let result_id = unsafe { runtime::__quantum__rt__result_allocate() };
    #[allow(clippy::cast_possible_wrap)]
    unsafe { runtime::__hugr__quantum__qis__m__body(i64::from(qubit), result_id as i64) };
}
