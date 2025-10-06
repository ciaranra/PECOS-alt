//! JIT-specific FFI functions
//!
//! This module contains FFI functions that are specific to JIT execution,
//! including the JIT interface pointer management and measurement futures.

use pecos_qis_ffi::{OperationCollector, Operation, QuantumOp};
use crate::measurement_manager::with_measurement_manager_mut;

/// Helper to convert i64 to usize
#[inline]
fn i64_to_usize(value: i64) -> usize {
    usize::try_from(value).expect("Invalid ID: value must be non-negative and fit in usize")
}

// =============================================================================
// JIT Interface Pointer Management
// =============================================================================

// Thread-local interface pointer for JIT execution
// This is set by the JIT executor before running code in each thread
thread_local! {
    static JIT_INTERFACE_PTR: std::cell::Cell<*mut OperationCollector> = std::cell::Cell::new(std::ptr::null_mut());
}

/// Set the thread-local interface pointer for JIT execution
/// SAFETY: This must only be called before JIT execution in each thread
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_set_jit_interface(interface_ptr: *mut OperationCollector) {
    JIT_INTERFACE_PTR.with(|ptr| ptr.set(interface_ptr));
}

/// Get the thread-local interface pointer for JIT execution
/// SAFETY: This must only be called after __pecos_set_jit_interface
unsafe fn get_jit_interface() -> &'static mut OperationCollector {
    let ptr = JIT_INTERFACE_PTR.with(|ptr| ptr.get());
    if ptr.is_null() {
        panic!("JIT interface not set - call __pecos_set_jit_interface first");
    }
    unsafe { &mut *ptr }
}

// =============================================================================
// JIT-safe versions of quantum operations
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_qalloc() -> i64 {
    let interface = unsafe { get_jit_interface() };
    let id = interface.allocate_qubit();
    interface.queue_operation(Operation::AllocateQubit { id });
    i64::try_from(id).expect("Qubit ID too large for i64")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_h(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::H(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_x(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::X(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_y(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::Y(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_z(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::Z(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_s(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::S(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_sdg(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::Sdg(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_t(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::T(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_tdg(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::Tdg(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_cx(control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CX(control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_cnot(control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CX(control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_cy(control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CY(control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_cz(control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CZ(control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_ch(control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CH(control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_lazy_measure(qubit: i64) -> i64 {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);

    // Allocate a future ID from the runtime for tracking
    let future_id = with_measurement_manager_mut(|manager| manager.allocate_future());

    // Use the future ID as the result ID for consistency
    let result_id = future_id as usize;

    // Queue the measurement operations
    interface.queue_operation(Operation::AllocateResult { id: result_id });
    interface.queue_operation(QuantumOp::Measure(qubit_id, result_id).into());

    // Return the future ID for use with ___read_future_bool
    future_id
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_rz(qubit: i64, theta: f64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::RZ(theta, qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_rxy(qubit: i64, theta: f64, phi: f64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::RXY(theta, phi, qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_rzz(qubit1: i64, qubit2: i64, theta: f64) {
    let interface = unsafe { get_jit_interface() };
    let qubit1_id = i64_to_usize(qubit1);
    let qubit2_id = i64_to_usize(qubit2);
    interface.queue_operation(QuantumOp::RZZ(theta, qubit1_id, qubit2_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_rx(theta: f64, qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::RX(theta, qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_ry(theta: f64, qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::RY(theta, qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_r1xy(theta: f64, phi: f64, qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::RXY(theta, phi, qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_crz(theta: f64, control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CRZ(theta, control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_ccx(control1: i64, control2: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control1_id = i64_to_usize(control1);
    let control2_id = i64_to_usize(control2);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CCX(control1_id, control2_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_zz(qubit1: i64, qubit2: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit1_id = i64_to_usize(qubit1);
    let qubit2_id = i64_to_usize(qubit2);
    interface.queue_operation(QuantumOp::ZZ(qubit1_id, qubit2_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_reset(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(QuantumOp::Reset(qubit_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_qfree(qubit: i64) {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    interface.queue_operation(Operation::ReleaseQubit { id: qubit_id });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_m(qubit: i64, result: i64) -> i32 {
    let interface = unsafe { get_jit_interface() };
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    interface.queue_operation(QuantumOp::Measure(qubit_id, result_id).into());
    // Return 0 for now - actual result will be available after runtime execution
    0
}

// =============================================================================
// Measurement future functions (Selene-style)
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___lazy_measure(qubit: i64) -> i64 {
    // Allocate a future ID from the runtime
    let future_id = with_measurement_manager_mut(|manager| manager.allocate_future());

    // Queue the measurement operation with the future ID
    let qubit_id = i64_to_usize(qubit);
    pecos_qis_ffi::with_interface(|interface| {
        // Store the future ID as a result ID for later processing
        let result_id = future_id as usize;
        interface.queue_operation(Operation::AllocateResult { id: result_id });
        interface.queue_operation(QuantumOp::Measure(qubit_id, result_id).into());
    });

    // Return the future ID
    future_id
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___read_future_bool(future_id: i64) -> bool {
    use crate::measurement_manager::with_measurement_manager;

    // Use the measurement manager to get the measurement result
    // In collection mode: returns false to follow default path
    // In simulation mode: returns the actual measurement result
    with_measurement_manager(|manager| manager.read_future_bool(future_id))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_read_future_bool(future_id: i64) -> bool {
    use crate::measurement_manager::with_measurement_manager;

    // Same as ___read_future_bool but explicitly marked for JIT use
    with_measurement_manager(|manager| manager.read_future_bool(future_id))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___inc_future_refcount(future_id: i64) {
    // Increment reference count for the future
    // For now, this is a no-op since we're not managing actual futures
    let _ = future_id;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___dec_future_refcount(future_id: i64) {
    // Decrement reference count for the future
    // For now, this is a no-op since we're not managing actual futures
    let _ = future_id;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_dec_future_refcount(future_id: i64) {
    // No-op for JIT execution
    let _ = future_id;
}
