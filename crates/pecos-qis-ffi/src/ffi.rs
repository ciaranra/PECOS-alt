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
    eprintln!("[FFI] __quantum__qis__h__body called with qubit={}", qubit);
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        eprintln!("[FFI] H gate: queuing operation for qubit {}", qubit_id);
        interface.queue_operation(QuantumOp::H(qubit_id).into());
        eprintln!("[FFI] H gate: operation queued, interface now has {} operations", interface.operations.len());
    });
    eprintln!("[FFI] __quantum__qis__h__body completed");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: i64) {
    eprintln!("[FFI] __quantum__qis__x__body called with qubit={}", qubit);
    let qubit_id = i64_to_usize(qubit);
    with_interface(|interface| {
        eprintln!("[FFI] X gate: queuing operation for qubit {}", qubit_id);
        interface.queue_operation(QuantumOp::X(qubit_id).into());
        eprintln!("[FFI] X gate: operation queued, interface now has {} operations", interface.operations.len());
    });
    eprintln!("[FFI] __quantum__qis__x__body completed");
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
pub unsafe extern "C" fn __quantum__qis__rzz__body(theta: f64, qubit1: i64, qubit2: i64) {
    let qubit1_id = i64_to_usize(qubit1);
    let qubit2_id = i64_to_usize(qubit2);
    with_interface(|interface| {
        interface.queue_operation(QuantumOp::RZZ(theta, qubit1_id, qubit2_id).into());
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
// QIS measurement functions
// =============================================================================

// QIS measurement functions - mz is measurement in Z basis
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__mz__body(qubit: i64) -> i32 {
    // Call our standard measurement function with result ID = qubit ID
    unsafe { __quantum__qis__m__body(qubit, qubit) }
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
    let label_slice = unsafe { std::slice::from_raw_parts(label_ptr, label_len) };

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

// =============================================================================
// Interface Management (C exports for dlsym access)
// =============================================================================

/// Reset the thread-local interface
/// Exported as C function so it can be called via dlsym from the cdylib
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pecos_qis_reset_interface() {
    crate::reset_interface();
}

/// Get a clone of the current OperationCollector
/// Exported as C function so it can be called via dlsym from the cdylib
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pecos_qis_get_operations() -> *mut crate::OperationCollector {
    let operations = with_interface(|interface| interface.clone());
    Box::into_raw(Box::new(operations))
}

/// Free an OperationCollector returned by pecos_qis_get_operations
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pecos_qis_free_operations(ptr: *mut crate::OperationCollector) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)); }
    }
}

/// Set measurement results in the thread-local interface
/// Takes a pointer to an array of (result_id, value) pairs and the array length
/// This allows pre-populating measurement outcomes for conditional execution
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pecos_qis_set_measurements(
    pairs_ptr: *const (usize, bool),
    count: usize
) {
    if pairs_ptr.is_null() {
        return;
    }

    let pairs = unsafe { std::slice::from_raw_parts(pairs_ptr, count) };
    let mut measurements = std::collections::HashMap::new();

    for &(result_id, value) in pairs {
        measurements.insert(result_id, value);
    }

    with_interface(|interface| {
        interface.set_measurement_results(measurements);
    });
}

// =============================================================================
// Heap Management Functions (Selene compatibility)
// =============================================================================

/// Allocate heap memory
///
/// This is used by Guppy/HUGR for array allocation and other heap operations.
/// Following Selene's approach, we use libc malloc/free which handle size tracking.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn heap_alloc(size: u64) -> *mut u8 {
    if size == 0 {
        // Return null for zero-sized allocations (standard malloc behavior)
        return std::ptr::null_mut();
    }

    // Use libc malloc which tracks allocation sizes internally
    let ptr = unsafe { libc::malloc(size as libc::size_t) as *mut u8 };

    if ptr.is_null() {
        panic!("heap_alloc: failed to allocate {} bytes", size);
    }

    ptr
}

/// Free heap memory
///
/// This is used by Guppy/HUGR to deallocate arrays and other heap objects.
/// Following Selene's approach, we use libc free which matches malloc.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn heap_free(ptr: *mut u8) {
    if ptr.is_null() {
        // Ignore null pointer frees (standard free behavior)
        return;
    }

    // Use libc free which pairs with malloc
    unsafe { libc::free(ptr as *mut libc::c_void) };
}