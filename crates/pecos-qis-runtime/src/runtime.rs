/// Instance-based LLVM Runtime Implementation for QIS (Quantum Instruction Set)
///
/// This runtime eliminates global state by using `RuntimeRegistry` to map
/// threads to their own isolated runtime states. Each worker/thread operates
/// independently without sharing state until results are combined.
///
/// The runtime implements the QIS standard used by HUGR, tket2, guppylang, and
/// other modern quantum compilers, with hardware-native gate sets (RXY/RZ/RZZ)
/// and triple underscore calling conventions.
// Submodule declarations
pub mod builder;
pub mod cleanup;
pub mod registry;
pub mod state;

// Re-export commonly used types
pub use state::LlvmRuntimeState;

// Internal use only
use registry::{RuntimeRegistry, initialize_registry};

// Internal imports
use log::{debug, error, warn};
use pecos_core::errors::PecosError;
use pecos_engines::byte_message::ByteMessage;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::sync::Once;
use std::thread;

// Ensure the runtime registry is initialized exactly once
static INIT: Once = Once::new();

// Constants for runtime behavior
const MAX_CALLBACK_DEPTH: usize = 5;
const CALLBACK_TIMEOUT_SECS: u64 = 30;

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
    self::registry::clear_shutting_down();
}

/// Helper function to get the current thread ID as a string
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Helper function to convert i64 to usize for qubit/result IDs
///
/// # Panics
///
/// Panics if the value is negative or too large for the target platform
#[inline]
fn i64_to_usize(value: i64) -> usize {
    usize::try_from(value)
        .expect("Invalid qubit/result ID: value must be non-negative and fit in usize")
}

/// Helper function to check if we should print commands
fn should_print_commands() -> bool {
    match env::var("LLVM_RUNTIME_QUIET") {
        Ok(val) => val != "1",
        Err(_) => true,
    }
}

// =============================================================================
// Core Runtime Implementation (Convention-Agnostic)
// =============================================================================

pub mod core_runtime {
    use super::{
        ByteMessage, PecosError, RuntimeRegistry, ensure_runtime_initialized, get_thread_id,
        should_print_commands,
    };
    use log::debug;

    /// Set the interactive execution callback for the current runtime
    pub fn set_interactive_callback(
        callback: Box<dyn Fn(ByteMessage) -> Result<Vec<u32>, PecosError> + Send + Sync>,
    ) {
        ensure_runtime_initialized();
        RuntimeRegistry::with_current_runtime(|state| {
            state.set_interactive_callback(callback);
        });
    }

    /// Clear the interactive execution callback for the current runtime
    pub fn clear_interactive_callback() {
        ensure_runtime_initialized();
        RuntimeRegistry::try_with_current_runtime(|state| {
            state.clear_interactive_callback();
        });
    }

    /// Reset the LLVM runtime state for the current thread
    pub fn reset() {
        ensure_runtime_initialized();
        let thread_id = get_thread_id();

        // Use try_with_current_runtime to avoid auto-initialization during cleanup
        if let Some(()) = RuntimeRegistry::try_with_current_runtime(|state| {
            state.reset();
            if should_print_commands() {
                debug!("[Thread {thread_id}] Reset LLVM runtime state");
            }
        }) {
            // Successfully reset
        } else {
            // No runtime to reset - this is fine during cleanup
            if should_print_commands() {
                debug!("[Thread {thread_id}] No runtime state to reset (already cleaned up)");
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
            debug!("Quantum runtime initialized");
        }
    }

    /// Allocate a qubit
    #[must_use]
    pub fn allocate_qubit() -> usize {
        RuntimeRegistry::with_current_runtime(|state| {
            let id = state.allocate_qubit();

            if should_print_commands() {
                let thread_id = get_thread_id();
                debug!("[Thread {thread_id}] Allocated qubit {id}");
            }

            id
        })
        .unwrap_or(0)
    }

    /// Allocate a result
    #[must_use]
    pub fn allocate_result() -> usize {
        RuntimeRegistry::with_current_runtime(|state| {
            let id = state.allocate_result();

            if should_print_commands() {
                let thread_id = get_thread_id();
                debug!("[Thread {thread_id}] Allocated result {id}");
            }

            id
        })
        .unwrap_or(0)
    }

    /// Release a qubit
    pub fn release_qubit(qubit_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!("[Thread {thread_id}] Released qubit {qubit_id}");
        }

        RuntimeRegistry::with_current_runtime(|state| {
            state.release_qubit(qubit_id);
        });
    }

    /// Release a result
    pub fn release_result(result_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!("[Thread {thread_id}] Released result {result_id}");
        }
    }

    /// Set the maximum number of qubits allowed for allocation
    pub fn set_max_qubits(max_qubits: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            state.set_max_qubits(max_qubits);
        });
    }

    /// Get the maximum number of qubits allowed for allocation
    #[must_use]
    pub fn get_max_qubits() -> Option<usize> {
        RuntimeRegistry::with_current_runtime(|state| state.get_max_qubits()).unwrap_or(None)
    }

    // Quantum Gate Operations

    // Helper macro for single-qubit gates
    macro_rules! single_qubit_gate {
        ($name:ident, $method:ident, $gate_name:expr) => {
            pub fn $name(qubit_id: usize) {
                if should_print_commands() {
                    let thread_id = get_thread_id();
                    debug!(
                        "[Thread {thread_id}] {} gate on qubit {qubit_id}",
                        $gate_name
                    );
                }

                RuntimeRegistry::with_current_runtime(|state| {
                    let _ = state.message_builder_mut().$method(&[qubit_id]);
                });
            }
        };
        // Variant for gates without debug output
        ($name:ident, $method:ident) => {
            pub fn $name(qubit_id: usize) {
                RuntimeRegistry::with_current_runtime(|state| {
                    let _ = state.message_builder_mut().$method(&[qubit_id]);
                });
            }
        };
    }

    single_qubit_gate!(h_gate, add_h, "H");
    single_qubit_gate!(x_gate, add_x, "X");
    single_qubit_gate!(y_gate, add_y);
    single_qubit_gate!(z_gate, add_z);

    pub fn cx_gate(control_id: usize, target_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!("[Thread {thread_id}] CX gate: control={control_id}, target={target_id}");
        }

        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state
                .message_builder_mut()
                .add_cx(&[control_id], &[target_id]);
        });
    }

    pub fn cy_gate(control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state
                .message_builder_mut()
                .add_cy(&[control_id], &[target_id]);
        });
    }

    pub fn cz_gate(control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state
                .message_builder_mut()
                .add_cz(&[control_id], &[target_id]);
        });
    }

    pub fn ch_gate(control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // CH implemented as H on target, then CX
            let _ = state.message_builder_mut().add_h(&[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control_id], &[target_id]);
            let _ = state.message_builder_mut().add_h(&[target_id]);
        });
    }

    single_qubit_gate!(s_gate, add_sz);
    single_qubit_gate!(sdg_gate, add_szdg);
    single_qubit_gate!(t_gate, add_t);
    single_qubit_gate!(tdg_gate, add_tdg);

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
            let _ = state
                .message_builder_mut()
                .add_r1xy(theta, phi, &[qubit_id]);
        });
    }

    pub fn crz_gate(theta: f64, control_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // CRZ implemented as CX-RZ-CX sequence
            let _ = state
                .message_builder_mut()
                .add_cx(&[control_id], &[target_id]);
            let _ = state
                .message_builder_mut()
                .add_rz(theta / 2.0, &[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control_id], &[target_id]);
            let _ = state
                .message_builder_mut()
                .add_rz(-theta / 2.0, &[target_id]);
        });
    }

    pub fn ccx_gate(control1_id: usize, control2_id: usize, target_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // CCX (Toffoli) - simplified implementation
            let _ = state.message_builder_mut().add_h(&[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control2_id], &[target_id]);
            let _ = state.message_builder_mut().add_tdg(&[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control1_id], &[target_id]);
            let _ = state.message_builder_mut().add_t(&[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control2_id], &[target_id]);
            let _ = state.message_builder_mut().add_tdg(&[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control1_id], &[target_id]);
            let _ = state.message_builder_mut().add_t(&[control2_id]);
            let _ = state.message_builder_mut().add_t(&[target_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control1_id], &[control2_id]);
            let _ = state.message_builder_mut().add_t(&[control1_id]);
            let _ = state.message_builder_mut().add_tdg(&[control2_id]);
            let _ = state
                .message_builder_mut()
                .add_cx(&[control1_id], &[control2_id]);
            let _ = state.message_builder_mut().add_h(&[target_id]);
        });
    }

    pub fn szz_gate(qubit1_id: usize, qubit2_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state
                .message_builder_mut()
                .add_szz(&[qubit1_id], &[qubit2_id]);
        });
    }

    pub fn zz_gate(qubit1_id: usize, qubit2_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // ZZ gate implementation using CZ
            let _ = state
                .message_builder_mut()
                .add_cz(&[qubit1_id], &[qubit2_id]);
        });
    }

    pub fn rzz_gate(theta: f64, qubit1_id: usize, qubit2_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            let _ = state
                .message_builder_mut()
                .add_rzz(theta, &[qubit1_id], &[qubit2_id]);
        });
    }

    pub fn reset_qubit(qubit_id: usize) {
        RuntimeRegistry::with_current_runtime(|state| {
            // Reset implemented as preparation
            let _ = state.message_builder_mut().add_prep(&[qubit_id]);
        });
    }

    pub fn measure(qubit_id: usize, result_id: usize) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!("[Thread {thread_id}] Measuring qubit {qubit_id} with result_id {result_id}");
        }

        RuntimeRegistry::with_current_runtime(|state| {
            state.add_measurement(qubit_id, result_id);
        });
    }

    pub fn record_result_output(result_id: usize, name: &str) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!("[Thread {thread_id}] Recording result {result_id} as '{name}'");
        }

        RuntimeRegistry::with_current_runtime(|state| {
            // Get the next bit position for this register
            let current_bit_position = state.get_register_bit_width(name);

            // Store the mapping for when we get the actual measurement result
            state.map_result_to_register(result_id, name.to_string(), current_bit_position);

            if should_print_commands() {
                let thread_id = get_thread_id();
                debug!(
                    "[Thread {thread_id}] Mapped result {result_id} to register '{name}' bit {current_bit_position}"
                );
            }
        });
    }

    /// Store tuple return values from a function
    pub fn store_tuple_return(values: &[i32]) {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!(
                "[Thread {thread_id}] Storing tuple return with {} values: {:?}",
                values.len(),
                values
            );
            debug!(
                "[Thread {thread_id}] Binary values: {:?}",
                values
                    .iter()
                    .map(|v| format!("{v:032b}"))
                    .collect::<Vec<_>>()
            );
        }

        RuntimeRegistry::with_current_runtime(|state| {
            state.set_tuple_return(values);
        });
    }

    /// Force execution of any pending measurements
    pub fn force_measurement_execution() {
        RuntimeRegistry::with_current_runtime(|state| {
            // Check if we have accumulated operations to execute
            let has_operations = state.message_builder_mut().message_count() > 0;

            if has_operations {
                if should_print_commands() {
                    let thread_id = get_thread_id();
                    debug!(
                        "[Thread {thread_id}] Forcing measurement execution before tuple return"
                    );
                }

                // Get the measurement result IDs before executing
                let measurement_result_ids = state.get_measurement_result_ids().to_vec();

                // Build the message with accumulated quantum operations
                let message = state.build_message();

                // Get the callback from the runtime state
                if let Some(callback) = state.interactive_callback() {
                    // Execute the measurements
                    if let Ok(measurement_outcomes) = callback(message) {
                        if should_print_commands() {
                            let thread_id = get_thread_id();
                            debug!(
                                "[Thread {thread_id}] Got {} measurement outcomes from forced execution",
                                measurement_outcomes.len()
                            );
                        }

                        // Convert outcomes to result_id/value pairs
                        // The quantum backend returns measurement outcomes in order
                        // We need to map them to the result IDs that were allocated
                        let mut paired_results = Vec::new();
                        for (idx, &outcome) in measurement_outcomes.iter().enumerate() {
                            if idx < measurement_result_ids.len() {
                                let result_id = u32::try_from(measurement_result_ids[idx])
                                    .expect("Result ID exceeds u32 range");
                                paired_results.push(result_id);
                                paired_results.push(outcome);

                                if should_print_commands() {
                                    let thread_id = get_thread_id();
                                    debug!(
                                        "[Thread {thread_id}] Mapping measurement[{idx}] outcome={outcome} to result_id={result_id}"
                                    );
                                }
                            }
                        }

                        // Update the runtime state with the properly paired results
                        state.update_measurement_results(&paired_results);
                    }
                }
            }
        });
    }

    /// Get measurement results for tuple return
    /// Returns None if no measurement results or wrong count
    #[must_use]
    pub fn get_measurement_results_for_tuple(expected_count: usize) -> Option<Vec<i32>> {
        RuntimeRegistry::with_current_runtime(|state| {
            // Get the measurement result IDs in order
            let result_ids = state.get_measurement_result_ids();

            if should_print_commands() {
                let thread_id = get_thread_id();
                debug!(
                    "[Thread {thread_id}] get_measurement_results_for_tuple: expected={expected_count}, result_ids={result_ids:?}"
                );
            }

            // Check if we have the expected number of results
            if result_ids.len() != expected_count {
                if should_print_commands() {
                    let thread_id = get_thread_id();
                    debug!(
                        "[Thread {thread_id}] Expected {} measurement results but have {}",
                        expected_count,
                        result_ids.len()
                    );
                }
                return None;
            }

            // Get the actual measurement values for each result ID
            let mut values = Vec::with_capacity(expected_count);
            for (idx, &result_id) in result_ids.iter().enumerate() {
                if let Some(measurement_value) = state.get_measurement_result(result_id) {
                    // Convert bool to i32 (false = 0, true = 1)
                    let int_val = i32::from(measurement_value);
                    values.push(int_val);
                    if should_print_commands() {
                        let thread_id = get_thread_id();
                        debug!(
                            "[Thread {thread_id}] Measurement[{idx}]: result_id={result_id} value={measurement_value} (as i32={int_val})"
                        );
                    }
                } else {
                    if should_print_commands() {
                        let thread_id = get_thread_id();
                        debug!(
                            "[Thread {thread_id}] No measurement result found for ID {result_id}"
                        );
                    }
                    return None;
                }
            }

            if should_print_commands() {
                let thread_id = get_thread_id();
                debug!(
                    "[Thread {thread_id}] Returning measurement tuple values: {values:?}"
                );
            }

            Some(values)
        }).flatten()
    }

    pub fn update_measurement_results(results: &[u32]) {
        let thread_id = get_thread_id();

        if should_print_commands() {
            debug!(
                "[Thread {thread_id}] Updating {} measurement results",
                results.len() / 2
            );
        }

        RuntimeRegistry::with_current_runtime(|state| {
            state.update_measurement_results(results);

            if should_print_commands() {
                for i in (0..results.len()).step_by(2) {
                    let result_id = results[i] as usize;
                    let measurement_value = results[i + 1] != 0;
                    debug!(
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
                debug!("[Thread {thread_id}] Finalized shot");
            }
        });
    }
}

// =============================================================================
// QIS Runtime Functions
// =============================================================================

/// Reset the LLVM runtime state
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// It performs thread-safe operations internally.
/// No preconditions are required beyond ensuring the runtime is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_reset() {
    // Clear the interactive callback first
    core_runtime::clear_interactive_callback();

    // Reset the core runtime (this only resets the current thread's state)
    core_runtime::reset();

    // Note: We DON'T call cleanup_all_runtimes() here because that would
    // interfere with other worker threads in multi-threaded execution.
    // Each thread should only reset its own runtime state.
}

/// Setup function for QIS runtime
///
/// Called at the beginning of program execution with a seed/time cursor
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setup(seed: i64) {
    debug!("QIS: Setup with seed {seed}");
    core_runtime::initialize();
    // TODO: Use seed for random number generation if needed
}

/// Teardown function for QIS runtime
///
/// Called at the end of program execution
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn teardown() -> i64 {
    debug!("QIS: Teardown");
    // Shot finalization is handled by the runtime automatically
    // Return success status
    0
}

// Note: Standard LLVM runtime functions (with usize parameters) have been removed.
// Only HUGR convention functions (with i64 parameters) are supported.

// =============================================================================
// HUGR Convention Functions (Integer-based)
// =============================================================================

/// Apply Hadamard gate to a qubit
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::h_gate(qubit_id);
}

/// Apply Pauli-X gate to a qubit
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::x_gate(qubit_id);
}

/// Apply Pauli-Y gate to a qubit
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::y_gate(qubit_id);
}

/// Apply Pauli-Z gate to a qubit
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::z_gate(qubit_id);
}

/// Apply controlled-X (CNOT) gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both control and target must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cx_gate(control_id, target_id);
}

/// Apply CNOT gate (alias for CX)
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both control and target must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cnot__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cx_gate(control_id, target_id);
}

/// Apply controlled-Y gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both control and target must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cy__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cy_gate(control_id, target_id);
}

/// Apply controlled-Z gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both control and target must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::cz_gate(control_id, target_id);
}

/// Apply controlled-H gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both control and target must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ch__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::ch_gate(control_id, target_id);
}

/// Apply S gate (phase gate)
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__s__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::s_gate(qubit_id);
}

/// Apply S-dagger gate (conjugate of S gate)
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__sdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::sdg_gate(qubit_id);
}

/// Apply T gate (π/8 gate)
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__t__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::t_gate(qubit_id);
}

/// Apply T-dagger gate (conjugate of T gate)
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__tdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::tdg_gate(qubit_id);
}

/// Reset a qubit to |0⟩
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__reset__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::reset_qubit(qubit_id);
}

/// Release (discard) a qubit
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::release_qubit(qubit_id);
}

/// Apply rotation around X-axis
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
/// The theta parameter must be a finite floating-point value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rx__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::rx_gate(theta, qubit_id);
}

/// Apply rotation around Y-axis
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
/// The theta parameter must be a finite floating-point value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ry__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::ry_gate(theta, qubit_id);
}

/// Apply rotation around Z-axis
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
/// The theta parameter must be a finite floating-point value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::rz_gate(theta, qubit_id);
}

/// Apply R1XY gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The qubit parameter must be a valid qubit ID previously allocated.
/// Both theta and phi parameters must be finite floating-point values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    core_runtime::r1xy_gate(theta, phi, qubit_id);
}

/// Apply controlled rotation around Z-axis
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both control and target must be valid qubit IDs previously allocated.
/// The theta parameter must be a finite floating-point value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__crz__body(theta: f64, control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    core_runtime::crz_gate(theta, control_id, target_id);
}

/// Apply Toffoli (CCX) gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// All control1, control2, and target must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ccx__body(control1: i64, control2: i64, target: i64) {
    let control1_id = i64_to_usize(control1);
    let control2_id = i64_to_usize(control2);
    let target_id = i64_to_usize(target);
    core_runtime::ccx_gate(control1_id, control2_id, target_id);
}

/// Apply ZZ gate
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both qubit1 and qubit2 must be valid qubit IDs previously allocated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body(qubit1: i64, qubit2: i64) {
    let qubit1_id = i64_to_usize(qubit1);
    let qubit2_id = i64_to_usize(qubit2);
    core_runtime::zz_gate(qubit1_id, qubit2_id);
}

/// Allocate a result for HUGR convention - returns i64 ID
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The function is thread-safe and will always return a valid result ID.
///
/// # Panics
///
/// Panics if the result ID is too large to fit in an i64 (extremely unlikely).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> i64 {
    i64::try_from(core_runtime::allocate_result()).expect("Result ID too large for i64")
}

/// HUGR-style qubit allocation - returns integer ID instead of pointer
///
/// This function allocates a new qubit and returns its ID as an i64.
/// Used by HUGR-generated LLVM IR that expects integer-based qubit handling.
///
/// # Safety
/// This function is marked unsafe as it's called from C/FFI context.
/// The function is thread-safe and will always return a valid qubit ID.
///
/// # Panics
///
/// Panics if the qubit ID is too large to fit in an i64 (extremely unlikely).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> i64 {
    i64::try_from(core_runtime::allocate_qubit()).expect("Qubit ID too large for i64")
}

/// Measure a qubit (i64 version for compatibility)
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both qubit and result must be valid IDs previously allocated.
/// Returns 0 as this uses deferred measurement model.
/// Measure a qubit
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Both qubit and result must be valid IDs previously allocated.
/// Returns 0 as this uses deferred measurement model.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: i64, result: i64) -> i32 {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    core_runtime::measure(qubit_id, result_id);

    // In the deferred measurement model, measurement results are not available immediately.
    // This function records the measurement for later execution and always returns 0.
    // The actual measurement result will be available through __quantum__rt__result_get_one.
    0
}

/// Get measurement result as integer (0 or 1) for deferred measurement model
/// For HUGR's immediate measurement model, this function triggers interactive execution
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The result parameter must be a valid result ID previously allocated.
/// May trigger quantum execution if measurements haven't been performed yet.
///
/// # Panics
///
/// Panics if the result ID cannot be converted to usize (negative or too large).
#[allow(clippy::too_many_lines)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_get_one(result: i64) -> i32 {
    use std::time::{Duration, Instant};

    let result_id = i64_to_usize(result);
    let start_time = Instant::now();

    if should_print_commands() {
        let thread_id = get_thread_id();
        debug!("[Thread {thread_id}] ENTER __quantum__rt__result_get_one(result_id={result_id})");
    }

    // Safety mechanism: prevent infinite recursion in callback chains
    // This is a proper safety feature, not a workaround. It protects against
    // malformed quantum programs that might create circular dependencies.
    let current_depth = CALLBACK_DEPTH.with(std::cell::Cell::get);
    assert!(
        current_depth <= MAX_CALLBACK_DEPTH,
        "Quantum program error: Circular dependency detected in measurement results. \
             Result {result_id} depends on itself through a chain of {current_depth} callbacks. \
             Maximum allowed callback depth is {MAX_CALLBACK_DEPTH}."
    );

    // Add a timeout to prevent infinite hangs
    let timeout = Duration::from_secs(CALLBACK_TIMEOUT_SECS);

    let result = RuntimeRegistry::with_current_runtime(|state| {
        // Check for timeout
        if start_time.elapsed() > timeout {
            error!("[Thread {}] TIMEOUT: __quantum__rt__result_get_one exceeded 30s", get_thread_id());
            return 0;
        }

        // Track ALL tuple accesses, not just unexecuted ones
        // This is important for proper tuple index mapping
        if state.find_result_id_index(result_id).is_some() {
            state.track_tuple_access(result_id);
            if should_print_commands() {
                debug!("[Thread {}] Tracked tuple access for result_id={}", get_thread_id(), result_id);
                debug!("[Thread {}] Current tuple accessed results: {:?}", get_thread_id(),
                       state.get_tuple_accessed_results());
            }
        }

        // DEBUG: Log exactly what we're about to return
        if should_print_commands() {
            debug!("[Thread {}] __quantum__rt__result_get_one: Checking result_id={}", get_thread_id(), result_id);
            debug!("[Thread {}] Current measurement_results: {:?}", get_thread_id(), state.get_all_measurement_results());
        }

        // Check if this is a measurement that hasn't been executed yet
        if state.find_result_id_index(result_id).is_some() && state.get_measurement_result(result_id).is_none() {
            if should_print_commands() {
                debug!("[Thread {}] PLACEHOLDER: Returning 0 for unexecuted measurement {result_id}", get_thread_id());
            }
            // Return 0 as placeholder for unexecuted measurements
            // This is safe because 0 = false in bool context
            // We'll track which values need updating separately
            return 0;
        }

        // Try to get the measurement result first
        if let Some(measurement_value) = state.get_measurement_result(result_id) {
            if should_print_commands() {
                debug!("[Thread {}] CACHED: Found cached result {result_id} = {measurement_value}", get_thread_id());
            }
            // Note: We don't increment measurements_executed here because this measurement
            // was already executed previously (that's why it's cached)
            let return_value = i32::from(measurement_value);
            debug!("[Thread {}] CACHED: Returning {} for result_id={}", get_thread_id(), return_value, result_id);
            return_value
        } else {
            // HUGR immediate measurement: trigger interactive execution
            if should_print_commands() {
                let thread_id = get_thread_id();
                debug!("[Thread {thread_id}] INTERACTIVE: Triggering execution for result {result_id}");
            }

            // Check if we have accumulated operations to execute
            let has_operations = state.message_builder_mut().message_count() > 0;

            if has_operations {
                if should_print_commands() {
                    debug!("[Thread {}] BUILDING: Building message with {} operations", get_thread_id(), state.message_builder_mut().message_count());
                }

                // Build the message with accumulated quantum operations
                let message = state.build_message();

                if should_print_commands() {
                    debug!("[Thread {}] CALLBACK: Calling interactive callback", get_thread_id());
                }

                // Get the callback from the runtime state
                let callback_result = if let Some(callback) = state.interactive_callback() {
                    // Increment callback depth to detect recursion
                    CALLBACK_DEPTH.with(|d| d.set(d.get() + 1));

                    if should_print_commands() {
                        debug!("[Thread {}] EXECUTING: Starting quantum execution via EngineSystem", get_thread_id());
                    }
                    let exec_result = callback(message);
                    if should_print_commands() {
                        debug!("[Thread {}] EXECUTED: Quantum execution completed via EngineSystem", get_thread_id());
                    }

                    // Decrement callback depth after execution
                    CALLBACK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));

                    Some(exec_result)
                } else {
                    None
                };

                if let Some(callback_result) = callback_result {
                    match callback_result {
                        Ok(measurement_outcomes) => {
                            if should_print_commands() {
                                debug!("[Thread {}] SUCCESS: Got {} measurement outcomes", get_thread_id(), measurement_outcomes.len());
                            }

                            // Get info about measurements
                            let measurement_result_ids = state.get_measurement_result_ids().to_vec();
                            let previously_executed = state.get_measurements_executed();

                            // Find which measurement index we need for this result_id
                            let needed_index = state.find_result_id_index(result_id);

                            if should_print_commands() {
                                debug!("[Thread {}] Result {} is at index {:?}, previously executed: {}",
                                       get_thread_id(), result_id, needed_index, previously_executed);
                            }

                            // Convert outcomes to result_id/value pairs
                            // Only process the new measurements (skip previously executed ones)
                            let mut paired_results = Vec::new();
                            let outcomes_to_process = measurement_outcomes.len();

                            for (idx, &outcome) in measurement_outcomes.iter().enumerate().take(outcomes_to_process) {
                                let actual_idx = previously_executed + idx;
                                if actual_idx < measurement_result_ids.len() {
                                    let mapped_result_id = u32::try_from(measurement_result_ids[actual_idx])
                                        .expect("Result ID exceeds u32 range");
                                    paired_results.push(mapped_result_id);
                                    paired_results.push(outcome);

                                    if should_print_commands() {
                                        debug!("[Thread {}] Mapping measurement[{}] outcome={} to result_id={}",
                                               get_thread_id(), actual_idx, outcome, mapped_result_id);
                                    }
                                }
                            }

                            // Update the runtime state with the properly paired results
                            state.update_measurement_results(&paired_results);

                            // DON'T update the executed count - this causes issues with result mapping
                            // Keep it simple: each measurement is considered executed only when explicitly requested

                            // Now try to get the result again
                            if let Some(measurement_value) = state.get_measurement_result(result_id) {
                                if should_print_commands() {
                                    debug!("[Thread {}] FOUND: Result {result_id} = {measurement_value}", get_thread_id());
                                }
                                i32::from(measurement_value)
                            } else {
                                if should_print_commands() {
                                    debug!("[Thread {}] MISSING: Result {result_id} still not available after execution", get_thread_id());
                                }
                                0
                            }
                        }
                        Err(e) => {
                            error!("[Thread {}] ERROR: Interactive execution failed for result {result_id}: {:?}", get_thread_id(), e);
                            0
                        }
                    }
                } else {
                    if should_print_commands() {
                        debug!("[Thread {}] NO_CALLBACK: No interactive callback registered for result {result_id}", get_thread_id());
                    }
                    0
                }
            } else {
                if should_print_commands() {
                    debug!("[Thread {}] NO_OPS: No quantum operations for result {result_id}", get_thread_id());
                }
                0
            }
        }
    }).unwrap_or_else(|| {
        error!("[Thread {}] FATAL: No runtime state available for result {result_id}", get_thread_id());
        0
    });

    if should_print_commands() {
        let elapsed = start_time.elapsed();
        debug!(
            "[Thread {}] EXIT __quantum__rt__result_get_one(result_id={result_id}) = {result} in {elapsed:?}",
            get_thread_id()
        );

        // Extra debug for the specific case we're investigating
        debug!(
            "[Thread {}] RETURN VALUE: result_id={} -> {}",
            get_thread_id(),
            result_id,
            result
        );
    }

    result
}

// =============================================================================
// Common Runtime Functions (Used by both conventions)
// =============================================================================

/// Message printing
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The msg parameter must be a valid null-terminated C string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__message(msg: *const c_char) {
    if !msg.is_null() {
        let c_str = unsafe { CStr::from_ptr(msg) };
        if let Ok(rust_str) = c_str.to_str() {
            debug!("LLVM Message: {rust_str}");
        }
    }
}

/// Record data
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The data parameter must be a valid null-terminated C string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__record(data: *const c_char) {
    if !data.is_null() {
        let c_str = unsafe { CStr::from_ptr(data) };
        if let Ok(rust_str) = c_str.to_str()
            && should_print_commands()
        {
            debug!("LLVM Record: {rust_str}");
        }
    }
}

/// Record a result output
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The `result_ptr` is treated as a result ID (cast from pointer).
/// The name parameter must be a valid null-terminated C string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(
    result_ptr: *const u8,
    name: *const c_char,
) {
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
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The `results_ptr` must point to a valid array of u32 values of size `results_len`*2.
/// The array contains pairs of (`result_id`, value) for each measurement.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_update_measurement_results(
    results_ptr: *const u32,
    results_len: usize,
) {
    if results_ptr.is_null() || results_len == 0 {
        if should_print_commands() {
            let thread_id = get_thread_id();
            debug!("[Thread {thread_id}] No measurement results to update");
        }
        return;
    }

    // Convert the raw pointer to a slice (pairs of result_id, value)
    let results = unsafe { std::slice::from_raw_parts(results_ptr, results_len * 2) };
    core_runtime::update_measurement_results(results);
}

/// Finalize shot
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Must be called after all measurements for a shot have been recorded.
/// No preconditions beyond having an initialized runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_finalize_shot() {
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

/// Get binary commands for execution
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Returns a heap-allocated `FFIByteData` structure that must be freed with
/// `llvm_runtime_free_binary_commands`. The data field within may be null if empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_get_binary_commands() -> *mut FFIByteData {
    let thread_id = get_thread_id();

    // Use try_with_current_runtime to avoid auto-initialization during cleanup
    let message = RuntimeRegistry::try_with_current_runtime(|state| {
        state.build_message()
    }).unwrap_or_else(|| {
        if should_print_commands() {
            warn!("[Thread {thread_id}] WARNING: No runtime state available - returning empty message");
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
        debug!("[Thread {thread_id}] Got binary commands: {byte_len} bytes ({word_count} words)");
    }

    ptr
}

/// Free binary commands
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The ptr must be a valid pointer returned by `llvm_runtime_get_binary_commands`
/// or null. After calling this function, the pointer becomes invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_free_binary_commands(ptr: *mut FFIByteData) {
    let thread_id = get_thread_id();

    if ptr.is_null() {
        if should_print_commands() {
            error!("[Thread {thread_id}] ERROR: Attempted to free null FFIByteData pointer");
        }
        return;
    }

    let ffi_data = unsafe { Box::from_raw(ptr) };

    if !ffi_data.data.is_null() && ffi_data.word_count > 0 {
        let _aligned_data =
            unsafe { Vec::from_raw_parts(ffi_data.data, ffi_data.word_count, ffi_data.word_count) };
    }

    if should_print_commands() {
        debug!("[Thread {thread_id}] Freed FFIByteData");
    }
}

/// Get shot results
#[repr(C)]
pub struct FFIShotData {
    pub names: *mut *mut c_char,
    pub values: *mut i64,
    pub count: usize,
}

/// Get shot results
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Returns a heap-allocated `FFIShotData` structure that must be freed with
/// `llvm_runtime_free_shot_data`. Returns null if no shot data is available.
/// The names and values fields are heap-allocated arrays that will be freed together.
///
/// # Panics
///
/// Panics if a register name contains null bytes (invalid for C strings).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_get_shot_results() -> *mut FFIShotData {
    let thread_id = get_thread_id();

    // Use try_with_current_runtime to avoid auto-initialization during cleanup
    let shot_opt =
        RuntimeRegistry::try_with_current_runtime(|state| state.get_last_shot().cloned()).flatten();

    if let Some(shot) = shot_opt {
        let count = shot.data.len();

        if should_print_commands() {
            debug!("[Thread {thread_id}] Shot has {count} registers");
            for (k, v) in &shot.data {
                debug!("[Thread {thread_id}] - {k}: {v:?}");
            }
        }

        if count == 0 {
            return std::ptr::null_mut();
        }

        // Allocate arrays using Vec to ensure proper alignment and initialization
        let mut names_vec: Vec<*mut c_char> = vec![std::ptr::null_mut(); count];
        let names = names_vec.as_mut_ptr();
        std::mem::forget(names_vec);

        let mut values_vec: Vec<i64> = vec![0; count];
        let values = values_vec.as_mut_ptr();
        std::mem::forget(values_vec);

        // Populate the arrays - sort by key name for deterministic order
        let mut sorted_data: Vec<_> = shot.data.iter().collect();
        sorted_data.sort_by_key(|(name, _)| name.as_str());

        for (i, (name, data)) in sorted_data.into_iter().enumerate() {
            // Convert name to C string
            let c_name = std::ffi::CString::new(name.as_str()).unwrap();
            unsafe {
                *names.add(i) = c_name.into_raw();
            }

            // Extract value
            let value = match data {
                pecos_engines::shot_results::Data::U32(v) => i64::from(*v),
                pecos_engines::shot_results::Data::I64(v) => *v,
                pecos_engines::shot_results::Data::Bool(b) => i64::from(*b),
                pecos_engines::shot_results::Data::Vec(vec) => {
                    // For vector data, encode as bit pattern
                    let mut encoded = 0i64;
                    for (idx, item) in vec.iter().enumerate() {
                        if let pecos_engines::shot_results::Data::I32(val) = item
                            && *val != 0
                        {
                            encoded |= 1i64 << idx;
                        }
                    }
                    encoded
                }
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
            debug!("[Thread {thread_id}] Got shot results: {count} registers");
        }

        return ptr;
    }

    // Return null if no shot available
    std::ptr::null_mut()
}

/// Free shot data
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The data must be a valid pointer returned by `llvm_runtime_get_shot_results`
/// or null. This frees the `FFIShotData` structure, all name strings, and arrays.
/// After calling this function, the pointer and all contained data become invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_free_shot_data(data: *mut FFIShotData) {
    let thread_id = get_thread_id();

    if data.is_null() {
        if should_print_commands() {
            error!("[Thread {thread_id}] ERROR: Attempted to free null FFIShotData pointer");
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
            debug!("[Thread {thread_id}] Freed FFIShotData");
        }
    }
}

/// FFI structure for returning measurement result IDs
#[repr(C)]
pub struct FFIResultIds {
    pub ids: *mut usize,
    pub count: usize,
}

/// Get how many measurements have been executed
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Returns the number of measurements that have been executed so far.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_get_measurements_executed() -> usize {
    RuntimeRegistry::try_with_current_runtime(|state| state.get_measurements_executed())
        .unwrap_or(0)
}

/// Get measurement result IDs in order
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// Returns a heap-allocated array of result IDs that must be freed by the caller.
/// Returns null if no measurements have been recorded.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_get_measurement_result_ids() -> *mut FFIResultIds {
    let thread_id = get_thread_id();

    let result_ids = RuntimeRegistry::try_with_current_runtime(|state| {
        state.get_measurement_result_ids().to_vec()
    })
    .unwrap_or_default();

    if result_ids.is_empty() {
        return std::ptr::null_mut();
    }

    if should_print_commands() {
        debug!(
            "[Thread {thread_id}] Returning {} measurement result IDs",
            result_ids.len()
        );
    }

    // Allocate arrays
    let count = result_ids.len();
    let ids_array = Box::into_raw(result_ids.into_boxed_slice()).cast::<usize>();

    // Create the FFI struct
    let ffi_data = Box::new(FFIResultIds {
        ids: ids_array,
        count,
    });

    Box::into_raw(ffi_data)
}

// =============================================================================
// QIS (Quantum Instruction Set) Functions
// =============================================================================
// These functions implement the QIS standard used by HUGR, tket2, guppylang,
// and other modern quantum compilers. QIS uses hardware-native gate sets
// (RXY/RZ/RZZ) and triple underscore calling conventions.

// -----------------------------------------------------------------------------
// QIS Memory Management
// -----------------------------------------------------------------------------

/// Allocate a new qubit in |0⟩ state
///
/// Returns a unique qubit identifier
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___qalloc() -> i64 {
    unsafe { __quantum__rt__qubit_allocate() }
}

/// Free a qubit
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___qfree(qubit: i64) {
    unsafe { __quantum__rt__qubit_release(qubit) }
}

/// Reset a qubit to |0⟩ state
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___reset(qubit: i64) {
    unsafe { __quantum__qis__reset__body(qubit) }
}

// -----------------------------------------------------------------------------
// QIS Measurement Functions
// -----------------------------------------------------------------------------

/// Perform immediate measurement on a qubit
///
/// Returns the measurement result as a boolean
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___measure(qubit: i64) -> bool {
    debug!("QIS: Immediate measure qubit {qubit}");

    let result = unsafe { __quantum__rt__result_allocate() };
    unsafe { __quantum__qis__m__body(qubit, result) };
    let int_result = unsafe { __quantum__rt__result_get_one(result) };

    int_result != 0
}

/// Perform lazy measurement on a qubit
///
/// Returns a future reference to the measurement result
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___lazy_measure(qubit: i64) -> i64 {
    debug!("QIS: Lazy measure qubit {qubit}");

    let result = unsafe { __quantum__rt__result_allocate() };
    unsafe { __quantum__qis__m__body(qubit, result) };
    result
}

/// Perform lazy measurement with leakage detection
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___lazy_measure_leaked(qubit: i64) -> i64 {
    debug!("QIS: Lazy measure with leakage detection on qubit {qubit}");
    // TODO: Add leakage detection when backend supports it
    unsafe { ___lazy_measure(qubit) }
}

/// Perform lazy measurement and reset
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___lazy_measure_reset(qubit: i64) -> i64 {
    debug!("QIS: Lazy measure and reset qubit {qubit}");

    let result = unsafe { ___lazy_measure(qubit) };
    unsafe { ___reset(qubit) };
    result
}

// -----------------------------------------------------------------------------
// QIS Gate Functions
// -----------------------------------------------------------------------------

/// Apply an XY rotation (`PhasedX` gate)
///
/// RXY(theta, phi) = RZ(phi) RX(theta) RZ(-phi)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___rxy(qubit: i64, theta: f64, phi: f64) {
    debug!("QIS: RXY on qubit {qubit} with theta={theta}, phi={phi}");

    // Check if this is a Hadamard gate (specific angles)
    const PI_2: f64 = std::f64::consts::PI / 2.0;
    const EPSILON: f64 = 1e-10;

    if (theta - PI_2).abs() < EPSILON && (phi + PI_2).abs() < EPSILON {
        debug!("QIS: Recognized as Hadamard gate");
        unsafe { __quantum__qis__h__body(qubit) };
    } else {
        // General rotation: RXY(theta, phi) = RZ(-phi) RY(theta) RZ(phi)
        unsafe {
            __quantum__qis__rz__body(-phi, qubit);
            __quantum__qis__ry__body(theta, qubit);
            __quantum__qis__rz__body(phi, qubit);
        }
    }
}

/// Apply a Z rotation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___rz(qubit: i64, theta: f64) {
    debug!("QIS: RZ on qubit {qubit} with theta={theta}");
    unsafe { __quantum__qis__rz__body(theta, qubit) };
}

/// Apply a ZZ rotation (two-qubit gate)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___rzz(qubit1: i64, qubit2: i64, theta: f64) {
    debug!("QIS: RZZ on qubits {qubit1} and {qubit2} with theta={theta}");
    // RZZ(theta) = CNOT(q1,q2) RZ(theta)(q2) CNOT(q1,q2)
    unsafe {
        __quantum__qis__cnot__body(qubit1, qubit2);
        __quantum__qis__rz__body(theta, qubit2);
        __quantum__qis__cnot__body(qubit1, qubit2);
    }
}

// -----------------------------------------------------------------------------
// QIS Future Reference Management
// -----------------------------------------------------------------------------

/// Increment reference count for a future
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___inc_future_refcount(reference: i64) {
    debug!("QIS: Increment refcount for future {reference}");
    // Future references in PECOS are managed automatically
    // This is a no-op for now but could be used for reference tracking
}

/// Decrement reference count for a future
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___dec_future_refcount(reference: i64) {
    debug!("QIS: Decrement refcount for future {reference}");
    // Future references in PECOS are managed automatically
    // This is a no-op for now but could be used for cleanup
}

/// Read a boolean value from a future reference
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___read_future_bool(reference: i64) -> bool {
    debug!("QIS: Read boolean from future {reference}");
    let result = unsafe { __quantum__rt__result_get_one(reference) };
    result != 0
}

/// Read an unsigned integer value from a future reference
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ___read_future_uint(reference: i64) -> u64 {
    debug!("QIS: Read uint from future {reference}");
    let result = unsafe { __quantum__rt__result_get_one(reference) };
    result as u64
}

// -----------------------------------------------------------------------------
// QIS Error Handling
// -----------------------------------------------------------------------------

/// Panic function for error handling
///
/// Error codes < 1000 end the current shot, >= 1000 terminate the program
#[unsafe(no_mangle)]
pub unsafe extern "C" fn panic(code: i32, message: *const i8) -> ! {
    use std::ffi::CStr;

    let msg = if message.is_null() {
        "Unknown error".to_string()
    } else {
        unsafe { CStr::from_ptr(message) }
            .to_str()
            .unwrap_or("Invalid error message")
            .to_string()
    };

    eprintln!("QIS PANIC: Code {code}: {msg}");

    // Error codes >= 1000 are fatal and terminate the program
    // Error codes < 1000 should just end the current shot
    if code >= 1000 {
        std::process::exit(code - 1000);
    } else {
        // For now, still exit, but this should eventually just end the shot
        // TODO: Implement proper shot termination
        std::process::exit(code);
    }
}

/// Free result IDs data
///
/// # Safety
///
/// This function is marked unsafe as it's called from C/FFI context.
/// The data parameter must be a valid pointer returned by `llvm_runtime_get_measurement_result_ids`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn llvm_runtime_free_result_ids(data: *mut FFIResultIds) {
    if !data.is_null() {
        unsafe {
            let ffi_data = Box::from_raw(data);
            if !ffi_data.ids.is_null() {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ffi_data.ids, ffi_data.count));
            }
        }
    }
}
