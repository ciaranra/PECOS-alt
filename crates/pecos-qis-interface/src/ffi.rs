//! FFI exports for linking with QIS LLVM IR programs
//!
//! This module provides the minimal set of FFI functions needed to link QIS programs
//! with Rust. These functions simply collect operations into the thread-local interface
//! without performing any simulation or complex state management.

use crate::operations::{Operation, QuantumOp};
use crate::{with_interface, QisInterface};

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
    eprintln!("DEBUG FFI: __quantum__rt__qubit_allocate() called!");
    with_interface(|interface| {
        let id = interface.allocate_qubit();
        eprintln!("DEBUG FFI: Allocated qubit with ID {}", id);
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

// =============================================================================
// Selene-style FFI Functions
//
// These functions match the naming convention used by Selene's hugr-qis compiler.
// They provide the same functionality as the QIS-style functions above but with
// different names to support Selene-generated LLVM IR.
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___reset(qubit: i64) {
    // Delegate to the QIS-style function
    unsafe { __quantum__qis__reset__body(qubit) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___rxy(qubit: i64, theta: f64, phi: f64) {
    // Delegate to the QIS-style function
    unsafe { __quantum__qis__r1xy__body(theta, phi, qubit) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___rz(qubit: i64, theta: f64) {
    // Delegate to the QIS-style function
    unsafe { __quantum__qis__rz__body(theta, qubit) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___lazy_measure(qubit: i64) -> i64 {
    use crate::runtime::with_measurement_manager_mut;

    // Allocate a future ID from the runtime
    let future_id = with_measurement_manager_mut(|manager| manager.allocate_future());

    // Queue the measurement operation with the future ID
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        // Store the future ID as a result ID for later processing
        let result_id = future_id as usize;
        interface.queue_operation(Operation::AllocateResult { id: result_id });
        interface.queue_operation(QuantumOp::Measure(qubit_id, result_id).into());
    });

    // Return the future ID
    future_id
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___qalloc() -> i64 {
    // Delegate to the QIS-style function
    unsafe { __quantum__rt__qubit_allocate() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___qfree(qubit: i64) {
    // Delegate to the QIS-style function
    unsafe { __quantum__rt__qubit_release(qubit) };
}

/// Setup function (called at program start)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setup(_arg: i64) {
    // Nothing to do for now - the thread-local interface is automatically initialized
}

/// H gate function (Selene-style)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___h(qubit: i64) {
    // Delegate to the QIS-style function
    unsafe { __quantum__qis__h__body(qubit) };
}

/// CX gate function (Selene-style)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___cx(control: i64, target: i64) {
    // Delegate to the QIS-style function
    unsafe { __quantum__qis__cx__body(control, target) };
}

/// Teardown function (called at program end)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn teardown() -> i64 {
    // Return success
    0
}

/// Panic function (called on program errors)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn panic(code: i32, message: *const i8) {
    let msg = if message.is_null() {
        "Unknown error".to_string()
    } else {
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(message);
            cstr.to_string_lossy().to_string()
        }
    };
    std::panic!("QIS program panic: code={}, message={}", code, msg);
}

/// Record measurement result output (for compatibility with QIR)
/// This is typically used to record measurement results to classical registers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(result_id: i64, register_name: *const i8) {
    // For now, this is a no-op since we're collecting operations rather than executing them
    // In a real implementation, this would record the measurement result to the specified register
    // The actual measurement results are handled by the runtime during execution

    // We could potentially add this as metadata to the interface if needed
    // For debugging, we can at least validate the inputs
    let _result_id = i64_to_usize(result_id);

    if !register_name.is_null() {
        // Mark the unsafe operation explicitly
        let _register = unsafe { std::ffi::CStr::from_ptr(register_name) }.to_string_lossy();
        // In the future, we might want to record this information
    }
}

// =============================================================================
// JIT-Safe FFI Functions
//
// These functions avoid thread-local storage and instead take a direct pointer
// to a QisInterface. This is safer for JIT execution contexts where thread-local
// storage may not work reliably.
// =============================================================================

// Thread-local interface pointer for JIT execution
// This is set by the JIT executor before running code in each thread
thread_local! {
    static JIT_INTERFACE_PTR: std::cell::Cell<*mut QisInterface> = std::cell::Cell::new(std::ptr::null_mut());
}

/// Set the thread-local interface pointer for JIT execution
/// SAFETY: This must only be called before JIT execution in each thread
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_set_jit_interface(interface_ptr: *mut QisInterface) {
    JIT_INTERFACE_PTR.with(|ptr| ptr.set(interface_ptr));
}

/// Get the thread-local interface pointer for JIT execution
/// SAFETY: This must only be called after __pecos_set_jit_interface
unsafe fn get_jit_interface() -> &'static mut QisInterface {
    let ptr = JIT_INTERFACE_PTR.with(|ptr| ptr.get());
    if ptr.is_null() {
        panic!("JIT interface not set - call __pecos_set_jit_interface first");
    }
    unsafe { &mut *ptr }
}

// JIT-safe versions of Selene-style FFI functions

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
pub unsafe extern "C" fn __pecos_jit_cx(control: i64, target: i64) {
    let interface = unsafe { get_jit_interface() };
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    interface.queue_operation(QuantumOp::CX(control_id, target_id).into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_lazy_measure(qubit: i64) -> i64 {
    use crate::runtime::with_measurement_manager_mut;

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


// QIS measurement functions - mz is measurement in Z basis
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__mz__body(qubit: i64) -> i32 {
    // Call our standard measurement function with result ID = qubit ID
    unsafe { __quantum__qis__m__body(qubit, qubit) }
}


// Future-related FFI functions for quantum measurement results
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___read_future_bool(future_id: i64) -> bool {
    use crate::runtime::with_measurement_manager;

    // Use the measurement manager to get the measurement result
    // In collection mode: returns false to follow default path
    // In simulation mode: returns the actual measurement result
    with_measurement_manager(|manager| manager.read_future_bool(future_id))
}

// JIT-safe version that can be called during JIT execution
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __pecos_jit_read_future_bool(future_id: i64) -> bool {
    use crate::runtime::with_measurement_manager;

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

// =============================================================================
// Result printing functions
// =============================================================================

/// Print a boolean result with a label
///
/// This function is called by QIS programs to output measurement results
/// with labels like "measurement_0", "measurement_1", etc.
///
/// # Arguments
/// * `label_ptr` - Pointer to the label string
/// * `label_len` - Length of the label string
/// * `value` - Boolean value to print
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_bool(label_ptr: *const u8, label_len: i64, value: bool) {
    // Convert the C string to a Rust string for debugging
    let label_len = label_len as usize;
    let label_slice = std::slice::from_raw_parts(label_ptr, label_len);

    // For now, just log the print operation - this prevents segfaults
    // while allowing the program to run
    if let Ok(label_str) = std::str::from_utf8(label_slice) {
        // Log the measurement for debugging
        log::debug!("print_bool called: {} = {}", label_str, value);
    }

    // TODO: Properly integrate with measurement storage system
    // The current QisInterface uses numeric IDs, but Guppy uses string names
    // This mismatch needs to be resolved in a future update
}