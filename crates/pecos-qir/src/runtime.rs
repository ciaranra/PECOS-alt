/// Instance-based QIR Runtime Implementation with Convention Adapters
///
/// This runtime eliminates global state by using RuntimeRegistry to map
/// threads to their own isolated runtime states. Each worker/thread operates
/// independently without sharing state until results are combined.
///
/// The runtime is organized into three layers:
/// 1. Core runtime implementation (convention-agnostic)
/// 2. QIR convention adapter (pointer-based)
/// 3. HUGR convention adapter (integer-based)

use crate::runtime_registry::{RuntimeRegistry, initialize_registry};
use pecos_engines::byte_message::ByteMessage;
use pecos_core::errors::PecosError;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::thread;
use std::sync::{Once, Mutex, LazyLock};

// Ensure the runtime registry is initialized exactly once
static INIT: Once = Once::new();

// Circuit breaker for preventing infinite callback loops
thread_local! {
    static CALLBACK_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn ensure_runtime_initialized() {
    INIT.call_once(|| {
        initialize_registry();
    });
    // Always clear shutdown flag when ensuring runtime is initialized
    // This handles cases where Python keeps the process alive between tests
    crate::runtime_registry::clear_shutting_down();
}

/// Helper function to get the current thread ID as a string
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Helper function to convert i64 to usize for qubit/result IDs
#[inline]
fn i64_to_usize(value: i64) -> usize {
    value as usize
}

/// Helper function to check if we should print commands
fn should_print_commands() -> bool {
    match env::var("QIR_RUNTIME_QUIET") {
        Ok(val) => val != "1",
        Err(_) => true,
    }
}

// =============================================================================
// Core Runtime Implementation (Convention-Agnostic)
// =============================================================================

pub mod core_runtime {
    use super::*;
    
    /// Type alias for the interactive execution callback
    /// Takes a ByteMessage of quantum operations and returns measurement results
    pub type InteractiveCallback = Box<dyn Fn(ByteMessage) -> Result<Vec<u32>, PecosError> + Send + Sync>;
    
    /// Global callback for interactive execution
    static INTERACTIVE_CALLBACK: LazyLock<Mutex<Option<InteractiveCallback>>> = 
        LazyLock::new(|| Mutex::new(None));
    
    /// Set the interactive execution callback
    pub fn set_interactive_callback(callback: InteractiveCallback) {
        if let Ok(mut cb) = INTERACTIVE_CALLBACK.lock() {
            *cb = Some(callback);
        }
    }
    
    /// Execute with the interactive execution callback if available
    pub fn execute_with_callback<T>(f: impl FnOnce(&InteractiveCallback) -> T) -> Option<T> {
        if let Ok(cb) = INTERACTIVE_CALLBACK.lock() {
            cb.as_ref().map(|callback| f(callback))
        } else {
            None
        }
    }
    
    /// Clear the interactive execution callback
    pub fn clear_interactive_callback() {
        if let Ok(mut cb) = INTERACTIVE_CALLBACK.lock() {
            *cb = None;
        }
    }
    
    /// Reset the QIR runtime state for the current thread
    pub fn reset() {
        ensure_runtime_initialized();
        let thread_id = get_thread_id();

        // Use try_with_current_runtime to avoid auto-initialization during cleanup
        if let Some(()) = RuntimeRegistry::try_with_current_runtime(|state| {
            state.reset();
            if should_print_commands() {
                println!("[Thread {thread_id}] Reset QIR runtime state");
            }
        }) {
            // Successfully reset
        } else {
            // No runtime to reset - this is fine during cleanup
            if should_print_commands() {
                println!("[Thread {thread_id}] No runtime state to reset (already cleaned up)");
            }
        }
        
        // Also clear the interactive callback to ensure clean state
        clear_interactive_callback();
    }
    
    /// Initialize the runtime
    pub fn initialize() {
        ensure_runtime_initialized();
        
        RuntimeRegistry::with_current_runtime(|state| {
            state.reset();
        });

        if should_print_commands() {
            println!("Quantum runtime initialized");
        }
    }
    
    /// Allocate a qubit
    pub fn allocate_qubit() -> usize {
        RuntimeRegistry::with_current_runtime(|state| {
            let id = state.allocate_qubit();
            
            if should_print_commands() {
                let thread_id = get_thread_id();
                println!("[Thread {thread_id}] Allocated qubit {id}");
            }
            
            id
        }).unwrap_or(0)
    }
    
    /// Allocate a result
    pub fn allocate_result() -> usize {
        RuntimeRegistry::with_current_runtime(|state| {
            let id = state.allocate_result();
            
            if should_print_commands() {
                let thread_id = get_thread_id();
                println!("[Thread {thread_id}] Allocated result {id}");
            }
            
            id
        }).unwrap_or(0)
    }
    
    /// Release a qubit
    pub fn release_qubit(qubit_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] Released qubit {qubit_id}");
        }
    }
    
    /// Release a result
    pub fn release_result(result_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] Released result {result_id}");
        }
    }
    
    // Quantum Gate Operations
    
    pub fn h_gate(qubit_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] H gate on qubit {qubit_id}");
        }
        
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_h(&[qubit_id]);
        });
    }
    
    pub fn x_gate(qubit_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] X gate on qubit {qubit_id}");
        }
        
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_x(&[qubit_id]);
        });
    }
    
    pub fn y_gate(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_y(&[qubit_id]);
        });
    }
    
    pub fn z_gate(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_z(&[qubit_id]);
        });
    }
    
    pub fn cx_gate(control_id: usize, target_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] CX gate: control={control_id}, target={target_id}");
        }
        
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_cx(&[control_id], &[target_id]);
        });
    }
    
    pub fn cy_gate(control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_cy(&[control_id], &[target_id]);
        });
    }
    
    pub fn cz_gate(control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_cz(&[control_id], &[target_id]);
        });
    }
    
    pub fn ch_gate(control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // CH implemented as H on target, then CX
            let _ = state.message_builder_mut().add_h(&[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control_id], &[target_id]);
            let _ = state.message_builder_mut().add_h(&[target_id]);
        });
    }
    
    pub fn s_gate(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_sz(&[qubit_id]);
        });
    }
    
    pub fn sdg_gate(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_szdg(&[qubit_id]);
        });
    }
    
    pub fn t_gate(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_t(&[qubit_id]);
        });
    }
    
    pub fn tdg_gate(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_tdg(&[qubit_id]);
        });
    }
    
    pub fn rx_gate(theta: f64, qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_rx(theta, &[qubit_id]);
        });
    }
    
    pub fn ry_gate(theta: f64, qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_ry(theta, &[qubit_id]);
        });
    }
    
    pub fn rz_gate(theta: f64, qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_rz(theta, &[qubit_id]);
        });
    }
    
    pub fn r1xy_gate(theta: f64, phi: f64, qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_r1xy(theta, phi, &[qubit_id]);
        });
    }
    
    pub fn crz_gate(theta: f64, control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // CRZ implemented as CX-RZ-CX sequence
            let _ = state.message_builder_mut().add_cx(&[control_id], &[target_id]);
            let _ = state.message_builder_mut().add_rz(theta / 2.0, &[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control_id], &[target_id]);
            let _ = state.message_builder_mut().add_rz(-theta / 2.0, &[target_id]);
        });
    }
    
    pub fn ccx_gate(control1_id: usize, control2_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // CCX (Toffoli) - simplified implementation
            let _ = state.message_builder_mut().add_h(&[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control2_id], &[target_id]);
            let _ = state.message_builder_mut().add_tdg(&[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control1_id], &[target_id]);
            let _ = state.message_builder_mut().add_t(&[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control2_id], &[target_id]);
            let _ = state.message_builder_mut().add_tdg(&[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control1_id], &[target_id]);
            let _ = state.message_builder_mut().add_t(&[control2_id]);
            let _ = state.message_builder_mut().add_t(&[target_id]);
            let _ = state.message_builder_mut().add_cx(&[control1_id], &[control2_id]);
            let _ = state.message_builder_mut().add_t(&[control1_id]);
            let _ = state.message_builder_mut().add_tdg(&[control2_id]);
            let _ = state.message_builder_mut().add_cx(&[control1_id], &[control2_id]);
            let _ = state.message_builder_mut().add_h(&[target_id]);
        });
    }
    
    pub fn szz_gate(qubit1_id: usize, qubit2_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_szz(&[qubit1_id], &[qubit2_id]);
        });
    }
    
    pub fn zz_gate(qubit1_id: usize, qubit2_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // ZZ gate implementation using CZ
            let _ = state.message_builder_mut().add_cz(&[qubit1_id], &[qubit2_id]);
        });
    }
    
    pub fn rzz_gate(theta: f64, qubit1_id: usize, qubit2_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_rzz(theta, &[qubit1_id], &[qubit2_id]);
        });
    }
    
    pub fn reset_qubit(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // Reset implemented as preparation
            let _ = state.message_builder_mut().add_prep(&[qubit_id]);
        });
    }
    
    pub fn measure(qubit_id: usize, _result_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] Measuring qubit {qubit_id}");
        }
        
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state.message_builder_mut().add_measurements(&[qubit_id]);
        });
    }
    
    pub fn record_result_output(result_id: usize, name: &str) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] Recording result {result_id} as '{name}'");
        }

        RuntimeRegistry::with_current_runtime(|state| {
            // Get the next bit position for this register
            let current_bit_position = state.get_register_bit_width(name);
            
            // Store the mapping for when we get the actual measurement result
            state.map_result_to_register(result_id, name.to_string(), current_bit_position);

            if should_print_commands() {
                let thread_id = get_thread_id();
                println!(
                    "[Thread {thread_id}] Mapped result {result_id} to register '{name}' bit {current_bit_position}"
                );
            }
        });
    }
    
    
    pub fn update_measurement_results(results: &[u32]) {
        let thread_id = get_thread_id();
        
        if should_print_commands() {
            println!("[Thread {thread_id}] Updating {} measurement results", results.len() / 2);
        }

        RuntimeRegistry::with_current_runtime(|state| {
            state.update_measurement_results(results);
            
            if should_print_commands() {
                for i in (0..results.len()).step_by(2) {
                    let result_id = results[i] as usize;
                    let measurement_value = results[i + 1] != 0;
                    println!(
                        "[Thread {thread_id}] Updated measurement result {result_id} = {measurement_value}"
                    );
                }
            }
        });
    }
    
    pub fn finalize_shot() {
        let thread_id = get_thread_id();
        
        RuntimeRegistry::with_current_runtime(|state| {
            state.finalize_shot();
            
            if should_print_commands() {
                println!("[Thread {thread_id}] Finalized shot");
            }
        });
    }
}

// =============================================================================
// QIR Convention Adapter (Pointer-based)
// =============================================================================

/// Reset the QIR runtime state
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_reset() {
    // Clear the interactive callback first
    core_runtime::clear_interactive_callback();
    
    // Reset the core runtime (this only resets the current thread's state)
    core_runtime::reset();
    
    // Note: We DON'T call cleanup_all_runtimes() here because that would
    // interfere with other worker threads in multi-threaded execution.
    // Each thread should only reset its own runtime state.
}

/// Initialize the QIR runtime
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__initialize(_config: *const u8) {
    core_runtime::initialize();
}

/// Standard QIR qubit allocation - returns pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> *const u8 {
    core_runtime::allocate_qubit() as *const u8
}

/// Standard QIR result allocation - returns pointer  
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> *const u8 {
    core_runtime::allocate_result() as *const u8
}

/// Release a qubit (pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit_ptr: *const u8) {
    let qubit_id = qubit_ptr as usize;
    core_runtime::release_qubit(qubit_id);
}

/// Release a result (pointer version)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_release(result_ptr: *const u8) {
    let result_id = result_ptr as usize;
    core_runtime::release_result(result_id);
}

// QIR Gate Operations (Pointer Convention)

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit_ptr: *const u8) {
    let qubit_id = qubit_ptr as usize;
    core_runtime::h_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit_ptr: *const u8) {
    let qubit_id = qubit_ptr as usize;
    core_runtime::x_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: usize) {
    core_runtime::y_gate(qubit);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: usize) {
    core_runtime::z_gate(qubit);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control_ptr: *const u8, target_ptr: *const u8) {
    let control_id = control_ptr as usize;
    let target_id = target_ptr as usize;
    core_runtime::cx_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cnot__body(control_ptr: *const u8, target_ptr: *const u8) {
    let control_id = control_ptr as usize;
    let target_id = target_ptr as usize;
    core_runtime::cx_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body_usize(control: usize, target: usize) {
    core_runtime::cz_gate(control, target);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: usize) {
    core_runtime::rz_gate(theta, qubit);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: usize) {
    core_runtime::r1xy_gate(theta, phi, qubit);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rxy__body(theta: f64, phi: f64, qubit: usize) {
    core_runtime::r1xy_gate(theta, phi, qubit);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__szz__body(qubit1: usize, qubit2: usize) {
    core_runtime::szz_gate(qubit1, qubit2);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body(qubit1: usize, qubit2: usize) {
    core_runtime::zz_gate(qubit1, qubit2);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rzz__body(theta: f64, qubit1: usize, qubit2: usize) {
    core_runtime::rzz_gate(theta, qubit1, qubit2);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__reset__body(qubit: usize) {
    core_runtime::reset_qubit(qubit);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body_ptr(qubit_ptr: *const u8, result_ptr: *const u8) {
    let qubit_id = qubit_ptr as usize;
    let result_id = result_ptr as usize;
    core_runtime::measure(qubit_id, result_id);
}

// =============================================================================
// HUGR Convention Adapter (Integer-based)
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::h_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::x_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::y_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::z_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body__hugr(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cx_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cnot__body__hugr(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cx_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cy__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cy_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cz_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ch__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::ch_gate(control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__s__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::s_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__sdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::sdg_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__t__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::t_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__tdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::tdg_gate(qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rx__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::rx_gate(theta, qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ry__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::ry_gate(theta, qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body__hugr(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::rz_gate(theta, qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body__hugr(theta: f64, phi: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::r1xy_gate(theta, phi, qubit_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__crz__body(theta: f64, control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::crz_gate(theta, control_id, target_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ccx__body(control1: i64, control2: i64, target: i64) {
    let control1_id = i64_to_usize(control1);
    let control2_id = i64_to_usize(control2);
    let target_id = i64_to_usize(target);
    core_runtime::ccx_gate(control1_id, control2_id, target_id);
}

/// HUGR result allocation - returns i64 ID
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate_hugr() -> i64 {
    core_runtime::allocate_result() as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body_i64(qubit: i64, result: i64) -> u32 {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    core_runtime::measure(qubit_id, result_id);
    
    // NOTE: This function shouldn't be called directly in the new deferred model.
    // The LLVM-IR post-processor converts immediate calls to deferred calls.
    // Return 0 as fallback for any remaining immediate calls.
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__qis__m__body(qubit: i64, result: i64) {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    core_runtime::measure(qubit_id, result_id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: i64, result: i64) -> i32 {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    core_runtime::measure(qubit_id, result_id);
    
    // NOTE: This function shouldn't be called directly in the new deferred model.
    // The LLVM-IR post-processor converts immediate calls to deferred calls.
    // Return 0 as fallback for any remaining immediate calls.
    0
}

/// Get measurement result as integer (0 or 1) for deferred measurement model
/// For HUGR's immediate measurement model, this function triggers interactive execution
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_get_one(result: i64) -> i32 {
    use std::time::{Duration, Instant};
    
    let result_id = i64_to_usize(result);
    let start_time = Instant::now();
    
    if should_print_commands() {
        let thread_id = get_thread_id();
        println!("[Thread {thread_id}] ENTER __quantum__rt__result_get_one(result_id={result_id})");
    }
    
    // Circuit breaker: prevent recursive callback loops
    let current_depth = CALLBACK_DEPTH.with(|d| d.get());
    if current_depth > 5 {
        eprintln!("[Thread {}] CIRCUIT_BREAKER: Callback depth {} exceeded for result {result_id}", get_thread_id(), current_depth);
        return 0;
    }
    
    // Add a timeout to prevent infinite hangs
    let timeout = Duration::from_secs(30); // 30 second timeout
    
    let result = RuntimeRegistry::with_current_runtime(|state| {
        // Check for timeout
        if start_time.elapsed() > timeout {
            eprintln!("[Thread {}] TIMEOUT: __quantum__rt__result_get_one exceeded 30s", get_thread_id());
            return 0;
        }
        
        // Try to get the measurement result first
        if let Some(measurement_value) = state.get_measurement_result(result_id) {
            if should_print_commands() {
                println!("[Thread {}] CACHED: Found cached result {result_id} = {measurement_value}", get_thread_id());
            }
            if measurement_value { 1 } else { 0 }
        } else {
            // HUGR immediate measurement: trigger interactive execution
            if should_print_commands() {
                let thread_id = get_thread_id();
                println!("[Thread {thread_id}] INTERACTIVE: Triggering execution for result {result_id}");
            }
            
            // Check if we have accumulated operations to execute
            let has_operations = state.message_builder_mut().message_count() > 0;
            
            if !has_operations {
                if should_print_commands() {
                    println!("[Thread {}] NO_OPS: No quantum operations for result {result_id}", get_thread_id());
                }
                0
            } else {
                if should_print_commands() {
                    println!("[Thread {}] BUILDING: Building message with {} operations", get_thread_id(), state.message_builder_mut().message_count());
                }
                
                // Build the message with accumulated quantum operations
                let message = state.build_message();
                
                if should_print_commands() {
                    println!("[Thread {}] CALLBACK: Calling interactive callback", get_thread_id());
                }
                
                // Trigger interactive execution by calling the global callback
                // The QirEngine will handle this through its ControlEngine implementation
                
                // Increment callback depth to detect recursion
                CALLBACK_DEPTH.with(|d| d.set(d.get() + 1));
                
                let callback_result = core_runtime::execute_with_callback(|callback| {
                    if should_print_commands() {
                        println!("[Thread {}] EXECUTING: Starting quantum execution", get_thread_id());
                    }
                    let exec_result = callback(message);
                    if should_print_commands() {
                        println!("[Thread {}] EXECUTED: Quantum execution completed", get_thread_id());
                    }
                    exec_result
                });
                
                // Decrement callback depth after execution
                CALLBACK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
                
                if let Some(callback_result) = callback_result {
                    match callback_result {
                        Ok(measurement_results) => {
                            if should_print_commands() {
                                println!("[Thread {}] SUCCESS: Got {} measurement results", get_thread_id(), measurement_results.len());
                            }
                            
                            // Update the runtime state with the measurement results
                            state.update_measurement_results(&measurement_results);
                            
                            // Now try to get the result again
                            if let Some(measurement_value) = state.get_measurement_result(result_id) {
                                if should_print_commands() {
                                    println!("[Thread {}] FOUND: Result {result_id} = {measurement_value}", get_thread_id());
                                }
                                if measurement_value { 1 } else { 0 }
                            } else {
                                if should_print_commands() {
                                    println!("[Thread {}] MISSING: Result {result_id} still not available after execution", get_thread_id());
                                }
                                0
                            }
                        }
                        Err(e) => {
                            eprintln!("[Thread {}] ERROR: Interactive execution failed for result {result_id}: {:?}", get_thread_id(), e);
                            0
                        }
                    }
                } else {
                    if should_print_commands() {
                        println!("[Thread {}] NO_CALLBACK: No interactive callback registered for result {result_id}", get_thread_id());
                    }
                    0
                }
            }
        }
    }).unwrap_or_else(|| {
        eprintln!("[Thread {}] FATAL: No runtime state available for result {result_id}", get_thread_id());
        0
    });
    
    if should_print_commands() {
        let elapsed = start_time.elapsed();
        println!("[Thread {}] EXIT __quantum__rt__result_get_one(result_id={result_id}) = {result} in {elapsed:?}", get_thread_id());
    }
    
    result
}

// =============================================================================
// Common Runtime Functions (Used by both conventions)
// =============================================================================

/// Message printing
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__message(msg: *const c_char) {
    if !msg.is_null() {
        let c_str = unsafe { CStr::from_ptr(msg) };
        if let Ok(rust_str) = c_str.to_str() {
            println!("QIR Message: {rust_str}");
        }
    }
}

/// Record data
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__record(data: *const c_char) {
    if !data.is_null() {
        let c_str = unsafe { CStr::from_ptr(data) };
        if let Ok(rust_str) = c_str.to_str() {
            if should_print_commands() {
                println!("QIR Record: {rust_str}");
            }
        }
    }
}

/// Record a result output
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(result_ptr: *const u8, name: *const c_char) {
    let result_id = result_ptr as usize;
    
    let name_str = if name.is_null() {
        format!("result_{result_id}")
    } else {
        let c_str = unsafe { CStr::from_ptr(name) };
        c_str.to_string_lossy().into_owned()
    };

    core_runtime::record_result_output(result_id, &name_str);
}

/// Update measurement results
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_update_measurement_results(
    results_ptr: *const u32,
    results_len: usize,
) {
    if results_ptr.is_null() || results_len == 0 {
        if should_print_commands() {
            let thread_id = get_thread_id();
            println!("[Thread {thread_id}] No measurement results to update");
        }
        return;
    }

    // Convert the raw pointer to a slice (pairs of result_id, value)
    let results = unsafe { std::slice::from_raw_parts(results_ptr, results_len * 2) };
    core_runtime::update_measurement_results(results);
}

/// Finalize shot
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_finalize_shot() {
    core_runtime::finalize_shot();
}

// =============================================================================
// FFI Data Structures and Functions
// =============================================================================

/// Get binary commands for execution
#[repr(C)]
pub struct FFIByteData {
    pub data: *mut u32,
    pub word_count: usize,
    pub byte_len: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_binary_commands() -> *mut FFIByteData {
    let thread_id = get_thread_id();

    // Use try_with_current_runtime to avoid auto-initialization during cleanup
    let message = RuntimeRegistry::try_with_current_runtime(|state| {
        state.build_message()
    }).unwrap_or_else(|| {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] WARNING: No runtime state available - returning empty message");
        }
        pecos_engines::byte_message::ByteMessage::create_empty()
    });

    let bytes = message.into_bytes();
    let byte_len = bytes.len();

    let (data_ptr, word_count) = if byte_len > 0 {
        let word_count = byte_len.div_ceil(4);
        let mut aligned_data = vec![0u32; word_count];
        let aligned_bytes = bytemuck::cast_slice_mut::<u32, u8>(&mut aligned_data);
        aligned_bytes[..byte_len].copy_from_slice(&bytes);
        let data_ptr = aligned_data.as_mut_ptr();
        std::mem::forget(aligned_data);
        (data_ptr, word_count)
    } else {
        (std::ptr::null_mut(), 0)
    };

    let ffi_data = FFIByteData {
        data: data_ptr,
        word_count,
        byte_len,
    };

    let boxed_ffi = Box::new(ffi_data);
    let ptr = Box::into_raw(boxed_ffi);

    if should_print_commands() {
        println!("[Thread {thread_id}] Got binary commands: {byte_len} bytes ({word_count} words)");
    }

    ptr
}

/// Free binary commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_binary_commands(ptr: *mut FFIByteData) {
    let thread_id = get_thread_id();

    if ptr.is_null() {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Attempted to free null FFIByteData pointer");
        }
        return;
    }

    let ffi_data = unsafe { Box::from_raw(ptr) };

    if !ffi_data.data.is_null() && ffi_data.word_count > 0 {
        let _aligned_data = unsafe { 
            Vec::from_raw_parts(ffi_data.data, ffi_data.word_count, ffi_data.word_count) 
        };
    }

    if should_print_commands() {
        println!("[Thread {thread_id}] Freed FFIByteData");
    }
}

/// Get shot results
#[repr(C)]
pub struct FFIShotData {
    pub names: *mut *mut c_char,
    pub values: *mut i64,
    pub count: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_shot_results() -> *mut FFIShotData {
    let thread_id = get_thread_id();

    // Use try_with_current_runtime to avoid auto-initialization during cleanup
    let shot_opt = RuntimeRegistry::try_with_current_runtime(|state| {
        state.get_last_shot().cloned()
    }).flatten();

    if let Some(shot) = shot_opt {
        let count = shot.data.len();

        if count == 0 {
            return std::ptr::null_mut();
        }

        // Allocate arrays using Vec to ensure proper alignment
        let mut names_vec: Vec<*mut c_char> = Vec::with_capacity(count);
        let names = names_vec.as_mut_ptr();
        std::mem::forget(names_vec);

        let mut values_vec: Vec<i64> = Vec::with_capacity(count);
        let values = values_vec.as_mut_ptr();
        std::mem::forget(values_vec);

        // Populate the arrays
        for (i, (name, data)) in shot.data.iter().enumerate() {
            // Convert name to C string
            let c_name = std::ffi::CString::new(name.as_str()).unwrap();
            unsafe {
                *names.add(i) = c_name.into_raw();
            }

            // Extract value
            let value = match data {
                pecos_engines::shot_results::Data::U32(v) => i64::from(*v),
                pecos_engines::shot_results::Data::I64(v) => *v,
                _ => 0, // Default for other types
            };
            unsafe {
                *values.add(i) = value;
            }
        }

        // Create and return the FFI struct
        let ffi_data = FFIShotData {
            names,
            values,
            count,
        };

        let boxed_ffi = Box::new(ffi_data);
        let ptr = Box::into_raw(boxed_ffi);

        if should_print_commands() {
            println!("[Thread {thread_id}] Got shot results: {count} registers");
        }

        return ptr;
    }

    // Return null if no shot available
    std::ptr::null_mut()
}

/// Free shot data
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_shot_data(data: *mut FFIShotData) {
    let thread_id = get_thread_id();

    if data.is_null() {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Attempted to free null FFIShotData pointer");
        }
        return;
    }

    unsafe {
        let ffi_data = Box::from_raw(data);

        // Free the name strings
        for i in 0..ffi_data.count {
            let name_ptr = *ffi_data.names.add(i);
            if !name_ptr.is_null() {
                let _ = CString::from_raw(name_ptr);
            }
        }

        // Free the arrays by reconstructing the Vecs
        if ffi_data.count > 0 {
            let _ = Vec::from_raw_parts(ffi_data.names, ffi_data.count, ffi_data.count);
            let _ = Vec::from_raw_parts(ffi_data.values, ffi_data.count, ffi_data.count);
        }

        if should_print_commands() {
            println!("[Thread {thread_id}] Freed FFIShotData");
        }
    }
}