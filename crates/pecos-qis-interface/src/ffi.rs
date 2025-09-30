//! FFI exports for linking with QIS LLVM IR programs
//!
//! This module provides the minimal set of FFI functions needed to link QIS programs
//! with Rust. These functions simply collect operations into the thread-local interface
//! without performing any simulation or complex state management.

use crate::operations::{Operation, QuantumOp};
use crate::with_interface;

/// Helper to convert i64 to usize
#[inline]
fn i64_to_usize(value: i64) -> usize {
    usize::try_from(value).expect("Invalid ID: value must be non-negative and fit in usize")
}

// =============================================================================
// Single-Qubit Gates
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::H(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::X(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::Y(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::Z(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__s__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::S(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__sdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::Sdg(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__t__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::T(qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__tdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::Tdg(qubit_id).into());
    });
}

// =============================================================================
// Two-Qubit Gates
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::CX(control_id, target_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cnot__body(control: i64, target: i64) {
    // CNOT is an alias for CX
    unsafe { __quantum__qis__cx__body(control, target) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cy__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::CY(control_id, target_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::CZ(control_id, target_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ch__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::CH(control_id, target_id).into());
    });
}

// =============================================================================
// Rotation Gates
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rx__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::RX(theta, qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ry__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::RY(theta, qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::RZ(theta, qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::RXY(theta, phi, qubit_id).into());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__crz__body(theta: f64, control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::CRZ(theta, control_id, target_id).into());
    });
}

// =============================================================================
// Three-Qubit Gates
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ccx__body(control1: i64, control2: i64, target: i64) {
    let control1_id = i64_to_usize(control1);
    let control2_id = i64_to_usize(control2);
    let target_id = i64_to_usize(target);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::CCX(control1_id, control2_id, target_id).into());
    });
}

// =============================================================================
// ZZ Interaction
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body(qubit1: i64, qubit2: i64) {
    let qubit1_id = i64_to_usize(qubit1);
    let qubit2_id = i64_to_usize(qubit2);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::ZZ(qubit1_id, qubit2_id).into());
    });
}

// =============================================================================
// Measurement and Reset
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: i64, result: i64) -> i32 {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::Measure(qubit_id, result_id).into());
    });
    // Return 0 for now - actual result will be available after runtime execution
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__reset__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::Reset(qubit_id).into());
    });
}

// =============================================================================
// Allocation and Deallocation
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> i64 {
    with_interface(|interface| {
        let id = interface.allocate_qubit();
        interface.queue_operation(Operation::AllocateQubit { id });
        i64::try_from(id).expect("Qubit ID too large for i64")
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        interface.queue_operation(Operation::ReleaseQubit { id: qubit_id });
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> i64 {
    with_interface(|interface| {
        let id = interface.allocate_result();
        interface.queue_operation(Operation::AllocateResult { id });
        i64::try_from(id).expect("Result ID too large for i64")
    })
}

// =============================================================================
// Result Retrieval (placeholder - actual implementation in runtime)
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_get_one(result: i64) -> i32 {
    let result_id = i64_to_usize(result);
    with_interface(|interface| {
        // In the minimal interface, we just return a placeholder
        // The actual result will be available after runtime execution
        interface.get_result(result_id).map_or(0, |b| if b { 1 } else { 0 })
    })
}

// =============================================================================
// Utility Functions
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__message(msg: *const std::ffi::c_char) {
    if !msg.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(msg) };
        if let Ok(rust_str) = c_str.to_str() {
            log::trace!("QIS Message: {}", rust_str);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__record(data: *const std::ffi::c_char) {
    if !data.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(data) };
        if let Ok(rust_str) = c_str.to_str() {
            log::trace!("QIS Record: {}", rust_str);
        }
    }
}