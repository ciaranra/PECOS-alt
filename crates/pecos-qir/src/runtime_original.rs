use crate::runtime_context::{with_current_context, current_context_id};
use log::{debug, info};
use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use pecos_engines::shot_results::{Data, Shot};
use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString, c_char};
use std::io::{self, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// QIR Runtime Implementation
///
/// This file contains the implementation of the QIR runtime functions that are used
/// when executing QIR programs. It defines the C-compatible functions that are called
/// by the QIR program to perform quantum operations.
///
/// # QIR Runtime Library
///
/// This file is a key component of the QIR runtime library, which is built by the
/// `build.rs` script in the pecos-qir crate. The library is pre-built and placed
/// in the target directory to speed up QIR compilation.
///
/// When the QIR compiler runs, it first checks for a pre-built library. If found,
/// it uses that library directly. If not, it falls back to building the runtime
/// on-demand using this file and related files.
///
/// # Implementation Details
///
/// The runtime provides functions for:
/// - Quantum gate operations (H, X, Y, Z, etc.)
/// - Qubit and result allocation/release
/// - Measurement operations
/// - Classical control operations
/// - Logging and message output
///
/// # Safety
///
/// All quantum gate functions are called from C/C++ code and assume that qubit IDs
/// are valid and have been properly allocated. Calling with invalid qubit IDs may
/// lead to undefined behavior.
///
/// Helper function to get the current thread ID as a string
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Helper function to convert i64 to usize for qubit/result IDs
#[inline]
fn i64_to_usize(value: i64) -> usize {
    value as usize
}

// Global counters for qubit and result allocation
static NEXT_QUBIT_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_RESULT_ID: AtomicUsize = AtomicUsize::new(0);

// Removed global cleanup coordination mutex as it causes segfaults during library unload
// The static destructor order is non-deterministic and can cause issues

// Global message builder for quantum operations
use std::sync::LazyLock;

static MESSAGE_BUILDER: LazyLock<Mutex<ByteMessageBuilder>> =
    LazyLock::new(|| {
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        Mutex::new(builder)
    });

static RUNTIME_STATE: LazyLock<Mutex<RuntimeState>> =
    LazyLock::new(|| Mutex::new(RuntimeState::new()));

static LAST_SHOT: LazyLock<Mutex<Option<Shot>>> =
    LazyLock::new(|| Mutex::new(None));

// Structure to hold runtime state for classical registers
struct RuntimeState {
    // Measurement results by result ID
    measurement_results: HashMap<usize, bool>,
    // Classical registers by name
    classical_registers: HashMap<String, i64>,
    // Track bit positions for each register (register_name -> next_bit_position)
    register_bit_positions: HashMap<String, usize>,
    // Mapping of result IDs to register assignments (result_id -> (register_name, bit_position))
    result_mappings: HashMap<usize, (String, usize)>,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            measurement_results: HashMap::new(),
            classical_registers: HashMap::new(),
            register_bit_positions: HashMap::new(),
            result_mappings: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.measurement_results.clear();
        self.classical_registers.clear();
        self.register_bit_positions.clear();
        self.result_mappings.clear();
    }

    fn apply_mappings(&mut self) {
        // Clear existing register values
        self.classical_registers.clear();

        // Apply all result mappings to build register values
        for (result_id, (register_name, bit_position)) in &self.result_mappings {
            // Get the measurement result
            let measurement_value = self
                .measurement_results
                .get(result_id)
                .copied()
                .unwrap_or(false);

            // Get or create the register
            let register = self
                .classical_registers
                .entry(register_name.clone())
                .or_insert(0);

            // Set the bit
            if measurement_value {
                *register |= 1i64 << bit_position;
            } else {
                *register &= !(1i64 << bit_position);
            }
        }
    }

    fn export_shot(&self) -> Shot {
        let mut shot = Shot::default();

        // Export all classical registers to the shot
        for (name, &value) in &self.classical_registers {
            // Store all values as I64 for consistency with QIR standard
            shot.data.insert(name.clone(), Data::I64(value));
        }

        shot
    }
}


/// Helper function to check if we should print commands
///
/// This function checks the `QIR_RUNTIME_QUIET` environment variable
/// to determine if commands should be printed to stdout.
///
/// # Returns
///
/// * `true` - If commands should be printed
/// * `false` - If commands should not be printed
fn should_print_commands() -> bool {
    match env::var("QIR_RUNTIME_QUIET") {
        Ok(val) => val != "1",
        Err(_) => true,
    }
}

/// Helper function to store and optionally print quantum gate commands
///
/// This function stores the gate command in the current context's message builder
/// or falls back to global state for dynamic library compatibility.
///
/// # Arguments
///
/// * `gate_name` - The name of the gate for debug printing
/// * `add_to_builder` - A closure that adds the gate to the builder
fn store_gate_command<F>(gate_name: &str, add_to_builder: F)
where
    F: FnOnce(&mut ByteMessageBuilder),
{
    let thread_id = get_thread_id();

    // Check if we have a context available first
    let has_context = current_context_id().is_some();
    
    if has_context {
        // Use context-based execution
        if let Err(e) = with_current_context(|ctx| add_to_builder(&mut ctx.message_builder)) {
            eprintln!("QIR Runtime: [Thread {thread_id}] Failed to access context: {e}");
        }
    } else {
        // Use global state for dynamic library execution
        if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
            add_to_builder(&mut builder);
        } else {
            eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
        }
    }

    // Print the command if not in quiet mode
    if should_print_commands() {
        if let Some(ctx_id) = current_context_id() {
            println!("QIR Runtime: [Thread {thread_id}, Context {ctx_id}] {gate_name}");
        } else {
            println!("QIR Runtime: [Thread {thread_id}, Global] {gate_name}");
        }
    }
}

// Helper function for single-qubit gates
fn apply_single_qubit_gate(
    gate_name: &str,
    qubit: usize,
    apply_fn: impl FnOnce(&mut ByteMessageBuilder),
) {
    store_gate_command(&format!("{gate_name} {qubit}"), apply_fn);
}

// Helper function for two-qubit gates
fn apply_two_qubit_gate(
    gate_name: &str,
    qubit1: usize,
    qubit2: usize,
    apply_fn: impl FnOnce(&mut ByteMessageBuilder),
) {
    store_gate_command(&format!("{gate_name} {qubit1} {qubit2}"), apply_fn);
}

// Quantum gate operations

/// RZ gate (QIR standard - pointer-based)
///
/// Applies a rotation around the Z-axis using QIR opaque pointer.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `qubit_ptr` - The qubit pointer to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit_ptr: *const u8) {
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => {
            store_gate_command(&format!("RZ {theta} {qubit_id}"), |builder| {
                builder.add_rz(theta, &[qubit_id]);
            });
        }
        Err(err) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid qubit pointer in RZ gate: {:p}", qubit_ptr);
        }
    }
}

/// RZ gate (HUGR convention - integer-based)  
///
/// Applies a rotation around the Z-axis using direct integer qubit ID.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `qubit` - The qubit ID to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body__hugr(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    store_gate_command(&format!("RZ {theta} {qubit_id}"), |builder| {
        builder.add_rz(theta, &[qubit_id]);
    });
}

/// R1XY gate (QIR standard - pointer-based)
///
/// Applies a rotation around an axis in the ZY plane using QIR opaque pointer.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `phi` - The phase angle in radians
/// * `qubit_ptr` - The qubit pointer to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit_ptr: *const u8) {
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => {
            store_gate_command(&format!("R1XY {theta} {phi} {qubit_id}"), |builder| {
                builder.add_r1xy(theta, phi, &[qubit_id]);
            });
        }
        Err(err) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid qubit pointer in R1XY gate: {:p}", qubit_ptr);
        }
    }
}

/// R1XY gate (HUGR convention - integer-based)  
///
/// Applies a rotation around an axis in the ZY plane using direct integer qubit ID.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `phi` - The phase angle in radians
/// * `qubit` - The qubit ID to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body__hugr(theta: f64, phi: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    store_gate_command(&format!("R1XY {theta} {phi} {qubit_id}"), |builder| {
        builder.add_r1xy(theta, phi, &[qubit_id]);
    });
}

/// Alias for r1xy to match QIR standard naming (QIR standard - pointer-based)
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rxy__body(theta: f64, phi: f64, qubit_ptr: *const u8) {
    unsafe {
        __quantum__qis__r1xy__body(theta, phi, qubit_ptr);
    }
}

/// Internal helper for Hadamard gate
#[inline]
fn h_gate_internal(qubit_id: usize) {
    apply_single_qubit_gate("H", qubit_id, |builder| {
        builder.add_h(&[qubit_id]);
    });
}

/// Hadamard gate (QIR standard - pointer-based)
///
/// Applies a Hadamard gate using QIR opaque pointer.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit_ptr: *const u8) {
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => h_gate_internal(qubit_id),
        Err(err) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid qubit pointer in H gate: {:p}", qubit_ptr);
        }
    }
}

/// Hadamard gate (HUGR convention - integer-based)  
///
/// Applies a Hadamard gate using direct integer qubit ID.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    h_gate_internal(qubit_id);
}

/// Internal helper for X gate
#[inline]
fn x_gate_internal(qubit_id: usize) {
    apply_single_qubit_gate("X", qubit_id, |builder| {
        builder.add_x(&[qubit_id]);
    });
}

/// X gate (QIR standard - pointer-based)
///
/// Applies an X gate using QIR opaque pointer.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit_ptr: *const u8) {
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => x_gate_internal(qubit_id),
        Err(err) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid qubit pointer in X gate: {:p}", qubit_ptr);
        }
    }
}

/// X gate (HUGR convention - integer-based)  
///
/// Applies an X gate using direct integer qubit ID.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    x_gate_internal(qubit_id);
}

/// Internal helper for Y gate
#[inline]
fn y_gate_internal(qubit_id: usize) {
    apply_single_qubit_gate("Y", qubit_id, |builder| {
        builder.add_y(&[qubit_id]);
    });
}

/// Y gate (QIR standard - pointer-based)
///
/// Applies a Y gate using QIR opaque pointer.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit_ptr: *const u8) {
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => y_gate_internal(qubit_id),
        Err(err) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid qubit pointer in Y gate: {:p}", qubit_ptr);
        }
    }
}

/// Y gate (HUGR convention - integer-based)  
///
/// Applies a Y gate using direct integer qubit ID.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    y_gate_internal(qubit_id);
}

/// Internal helper for Z gate
#[inline]
fn z_gate_internal(qubit_id: usize) {
    apply_single_qubit_gate("Z", qubit_id, |builder| {
        builder.add_z(&[qubit_id]);
    });
}

/// Z gate (QIR standard - pointer-based)
///
/// Applies a Z gate using QIR opaque pointer.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointer
/// is valid and has been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit_ptr: *const u8) {
    match qubit_ptr_to_id(qubit_ptr) {
        Ok(qubit_id) => z_gate_internal(qubit_id),
        Err(err) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid qubit pointer in Z gate: {:p}", qubit_ptr);
        }
    }
}

/// Z gate (HUGR convention - integer-based)  
///
/// Applies a Z gate using direct integer qubit ID.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body__hugr(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    z_gate_internal(qubit_id);
}

/// Internal helper for CX gate
#[inline]
fn cx_gate_internal(control_id: usize, target_id: usize) {
    apply_two_qubit_gate("CX", control_id, target_id, |builder| {
        builder.add_cx(&[control_id], &[target_id]);
    });
}

/// CX gate (QIR standard - pointer-based)
///
/// Applies a controlled-X gate using QIR opaque pointers.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit pointers
/// are valid and have been properly allocated via __quantum__rt__qubit_allocate_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body(control_ptr: *const u8, target_ptr: *const u8) {
    match (qubit_ptr_to_id(control_ptr), qubit_ptr_to_id(target_ptr)) {
        (Ok(control_id), Ok(target_id)) => cx_gate_internal(control_id, target_id),
        (Err(err), _) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid control qubit pointer in CX gate: {:p}", control_ptr);
        }
        (_, Err(err)) => {
            eprintln!("QIR Runtime Error: {}", err);
            panic!("Invalid target qubit pointer in CX gate: {:p}", target_ptr);
        }
    }
}

/// CX gate (HUGR convention - integer-based)  
///
/// Applies a controlled-X gate using direct integer qubit IDs.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cx__body__hugr(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    cx_gate_internal(control_id, target_id);
}

/// Applies a controlled-Y gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cy__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    apply_two_qubit_gate("CY", control_id, target_id, |builder| {
        builder.add_cy(&[control_id], &[target_id]);
    });
}

/// Applies a controlled-Z gate to the specified qubits.
///
/// This is implemented as a sequence of H, CX, H gates.
///
/// # Arguments
///
/// * `control` - The control qubit index
/// * `target` - The target qubit index
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__cz__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    // Implement CZ as a sequence of H, CX, H
    store_gate_command(
        &format!("CZ {control_id} {target_id} (as H-CX-H)"),
        |builder| {
            builder.add_h(&[target_id]);
            builder.add_cx(&[control_id], &[target_id]);
            builder.add_h(&[target_id]);
        },
    );
}

/// Applies a controlled-H gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ch__body(control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    apply_two_qubit_gate("CH", control_id, target_id, |builder| {
        // CH implementation - may need custom implementation in ByteMessageBuilder
        // For now, decompose as Ry(-pi/4) CZ Ry(pi/4)
        builder.add_ry(-std::f64::consts::FRAC_PI_4, &[target_id]);
        builder.add_cz(&[control_id], &[target_id]);
        builder.add_ry(std::f64::consts::FRAC_PI_4, &[target_id]);
    });
}

/// Applies an S gate (phase gate) to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__s__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("S", qubit_id, |builder| {
        builder.add_sz(&[qubit_id]);
    });
}

/// Applies an S† gate (inverse phase gate) to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__sdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("Sdg", qubit_id, |builder| {
        builder.add_szdg(&[qubit_id]);
    });
}

/// Applies a T gate (π/8 gate) to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__t__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("T", qubit_id, |builder| {
        // T gate is RZ(π/4)
        builder.add_rz(std::f64::consts::FRAC_PI_4, &[qubit_id]);
    });
}

/// Applies a T† gate (inverse π/8 gate) to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__tdg__body(qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    apply_single_qubit_gate("Tdg", qubit_id, |builder| {
        // T† gate is RZ(-π/4)
        builder.add_rz(-std::f64::consts::FRAC_PI_4, &[qubit_id]);
    });
}

/// Applies an RX gate to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rx__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    store_gate_command(&format!("RX {theta} {qubit_id}"), |builder| {
        builder.add_rx(theta, &[qubit_id]);
    });
}

/// Applies an RY gate to the specified qubit.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ry__body(theta: f64, qubit: i64) {
    let qubit_id = i64_to_usize(qubit);
    store_gate_command(&format!("RY {theta} {qubit_id}"), |builder| {
        builder.add_ry(theta, &[qubit_id]);
    });
}

/// Applies a controlled-RZ gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__crz__body(theta: f64, control: i64, target: i64) {
    let control_id = i64_to_usize(control);
    let target_id = i64_to_usize(target);
    store_gate_command(&format!("CRZ {theta} {control_id} {target_id}"), |builder| {
        // CRZ implementation - decompose as CX RZ CX RZ
        builder.add_rz(theta / 2.0, &[target_id]);
        builder.add_cx(&[control_id], &[target_id]);
        builder.add_rz(-theta / 2.0, &[target_id]);
        builder.add_cx(&[control_id], &[target_id]);
    });
}

/// Applies a Toffoli (CCX) gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__ccx__body(control1: i64, control2: i64, target: i64) {
    let control1_id = i64_to_usize(control1);
    let control2_id = i64_to_usize(control2);
    let target_id = i64_to_usize(target);
    store_gate_command(
        &format!("CCX {control1_id} {control2_id} {target_id}"),
        |builder| {
            // Toffoli gate decomposition using CNOT and single-qubit gates
            // This is a standard decomposition into 6 CNOTs and single-qubit gates
            builder.add_h(&[target_id]);
            builder.add_cx(&[control2_id], &[target_id]);
            builder.add_tdg(&[target_id]);
            builder.add_cx(&[control1_id], &[target_id]);
            builder.add_t(&[target_id]);
            builder.add_cx(&[control2_id], &[target_id]);
            builder.add_tdg(&[target_id]);
            builder.add_cx(&[control1_id], &[target_id]);
            builder.add_t(&[control2_id]);
            builder.add_t(&[target_id]);
            builder.add_h(&[target_id]);
            builder.add_cx(&[control1_id], &[control2_id]);
            builder.add_t(&[control1_id]);
            builder.add_tdg(&[control2_id]);
            builder.add_cx(&[control1_id], &[control2_id]);
        },
    );
}

/// Applies a SZZ gate to the specified qubits.
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__szz__body(qubit1: usize, qubit2: usize) {
    apply_two_qubit_gate("SZZ", qubit1, qubit2, |builder| {
        builder.add_szz(&[qubit1], &[qubit2]);
    });
}

/// Alias for szz to match QIR standard naming
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__zz__body(qubit1: usize, qubit2: usize) {
    unsafe {
        __quantum__qis__szz__body(qubit1, qubit2);
    }
}

/// Applies a RZZ gate to the specified qubits.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `qubit1` - The first qubit index
/// * `qubit2` - The second qubit index
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rzz__body(theta: f64, qubit1: usize, qubit2: usize) {
    store_gate_command(&format!("RZZ {theta} {qubit1} {qubit2}"), |builder| {
        builder.add_rzz(theta, &[qubit1], &[qubit2]);
    });
}

/// Internal helper for measurement
#[inline]
fn measure_internal(qubit_id: usize, result_id: usize) -> u32 {
    store_gate_command(&format!("M {qubit_id}"), |builder| {
        builder.add_measurements(&[qubit_id]);
    });

    // Store a placeholder measurement result
    // In a real implementation, this would be populated by the quantum engine
    // For now, we'll set it when processing measurement results
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Mark that this result ID is associated with a measurement
        // The actual value will be populated later by process_measurement_results
        state.measurement_results.insert(result_id, false);
    }

    // In the real QIR runtime, this would return the actual measurement result
    // For this implementation, we return 0 (will be updated later)
    0
}

/// Measures a qubit and stores the result (Standard QIR - void return).
///
/// # Arguments
///
/// * `qubit` - The qubit pointer to measure  
/// * `result` - The result pointer to store the measurement result
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID and result ID
/// are valid and have been properly allocated. Calling with invalid IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: *const u8, result: *const u8) {
    let qubit_id = qubit as usize;
    let result_id = result as usize;
    let _ = measure_internal(qubit_id, result_id);
    // Standard QIR expects void return, not the measurement result
}

/// Integer-based measurement for HUGR use
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body_i64(qubit: i64, result: i64) -> u32 {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    measure_internal(qubit_id, result_id)
}

/// HUGR-specific measurement that returns void (for compatibility)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __hugr__quantum__qis__m__body(qubit: i64, result: i64) {
    let qubit_id = i64_to_usize(qubit);
    let result_id = i64_to_usize(result);
    let _ = measure_internal(qubit_id, result_id);
}

/// Prepares a qubit in the |0⟩ state.
///
/// # Arguments
///
/// * `qubit` - The qubit index to prepare
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__reset__body(qubit: usize) {
    store_gate_command(&format!("PREP {qubit}"), |builder| {
        builder.add_prep(&[qubit]);
    });
}

/// Initialize the quantum runtime.
///
/// This function is called at the beginning of QIR programs to set up the runtime.
///
/// # Arguments
///
/// * `config` - Configuration string (currently unused, can be null)
///
/// # Safety
///
/// This function is called from C/C++ code. The config parameter can be null.
///
/// # Panics
///
/// This function will panic if the `MESSAGE_BUILDER` mutex is poisoned (i.e., if another
/// thread panicked while holding the lock).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__initialize(_config: *const u8) {
    // Force initialization of all LazyLock statics early to avoid destructor ordering issues
    let _ = &*MESSAGE_BUILDER;
    let _ = &*RUNTIME_STATE;
    let _ = &*LAST_SHOT;

    // Reset global state for new program execution - multiple attempts for robustness
    for _ in 0..3 {
        NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
        NEXT_RESULT_ID.store(0, Ordering::SeqCst);
    }

    // Reset the message builder to clear any existing commands
    // Handle potential mutex poisoning with multiple attempts
    for attempt in 0..3 {
        match MESSAGE_BUILDER.lock() {
            Ok(mut builder) => {
                *builder = ByteMessageBuilder::new();
                let _ = builder.for_quantum_operations();
                break;
            }
            Err(poisoned) => {
                if attempt == 2 {
                    // Last attempt - clear the poison and try to recover
                    let mut builder = poisoned.into_inner();
                    *builder = ByteMessageBuilder::new();
                    let _ = builder.for_quantum_operations();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    // Reset runtime state with error recovery
    for attempt in 0..3 {
        match RUNTIME_STATE.lock() {
            Ok(mut state) => {
                state.reset();
                break;
            }
            Err(poisoned) => {
                if attempt == 2 {
                    let mut state = poisoned.into_inner();
                    state.reset();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    // Clear the last shot with error recovery
    for attempt in 0..3 {
        match LAST_SHOT.lock() {
            Ok(mut last_shot) => {
                *last_shot = None;
                break;
            }
            Err(poisoned) => {
                if attempt == 2 {
                    let mut last_shot = poisoned.into_inner();
                    *last_shot = None;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    if should_print_commands() {
        println!("Quantum runtime initialized");
    }
}

/// Allocates a new qubit using context isolation
///
/// # Returns
///
/// The ID of the newly allocated qubit
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate() -> usize {
    let qubit_id = match with_current_context(|ctx| ctx.allocate_qubit()) {
        Ok(id) => {
            if should_print_commands() {
                if let Some(ctx_id) = current_context_id() {
                    println!("QIR Runtime: [Context {ctx_id}] Allocated qubit {id}");
                }
            }
            id
        }
        Err(_) => {
            // Fallback: Use global state for dynamic library execution
            let id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
            if should_print_commands() {
                println!("QIR Runtime: [Global] Allocated qubit {id}");
            }
            id
        }
    };
    
    qubit_id
}

/// Allocates a new result using context isolation
///
/// # Returns
///
/// The ID of the newly allocated result
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate() -> usize {
    let result_id = match with_current_context(|ctx| ctx.allocate_result()) {
        Ok(id) => {
            if should_print_commands() {
                if let Some(ctx_id) = current_context_id() {
                    println!("QIR Runtime: [Context {ctx_id}] Allocated result {id}");
                }
            }
            id
        }
        Err(_) => {
            // Fallback: Use global state for dynamic library execution
            let id = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
            if should_print_commands() {
                println!("QIR Runtime: [Global] Allocated result {id}");
            }
            id
        }
    };
    
    result_id
}

/// Releases a qubit.
///
/// # Arguments
///
/// * `qubit` - The qubit ID to release
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_release(qubit: usize) {
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Released qubit {qubit}");
    }

    // We don't actually do anything with the qubit ID
    // In a real implementation, we would recycle the ID
}

/// Releases a result.
///
/// # Arguments
///
/// * `result` - The result ID to release
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_release(result: usize) {
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Released result {result}");
    }

    // We don't actually do anything with the result ID
    // In a real implementation, we would recycle the ID
}

/// Records a message using Rust logging.
///
/// # Arguments
///
/// * `msg` - The message to record
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__message(msg: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(msg) };
    let msg_str = c_str.to_string_lossy();
    let thread_id = get_thread_id();

    // Use proper Rust logging instead of storing as QuantumCmd
    info!("QIR Message [Thread {}]: {}", thread_id, msg_str);
}

/// Records data.
///
/// # Arguments
///
/// * `data` - The data to record
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__record(data: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(data) };
    let data_str = c_str.to_string_lossy().into_owned();
    let thread_id = get_thread_id();

    // Log the record command
    debug!("QIR Runtime [Thread {}]: Record: {}", thread_id, data_str);

    if should_print_commands() {
        println!("QIR Runtime: [Thread {thread_id}] RECORD: {data_str}");
    }
}

/// Resets the QIR runtime.
///
/// This function clears all commands and measurement results.
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_reset() {
    let thread_id = get_thread_id();

    // Reset the message builder
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        builder.reset();
        let _ = builder.for_quantum_operations();

        if should_print_commands() {
            println!("[Thread {thread_id}] Reset QIR runtime (reset message builder)");
        }
    } else {
        // If we can't lock the mutex, print an error
        if should_print_commands() {
            eprintln!(
                "[Thread {thread_id}] ERROR: Failed to lock message builder mutex during reset"
            );
            io::stderr().flush().unwrap_or_default();
        }
    }

    // Reset qubit and result counters
    NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
    NEXT_RESULT_ID.store(0, Ordering::SeqCst);

    // Reset runtime state
    if let Ok(mut state) = RUNTIME_STATE.lock() {
        state.reset();
        if should_print_commands() {
            println!("[Thread {thread_id}] Reset runtime state (classical registers cleared)");
        }
    } else if should_print_commands() {
        eprintln!("[Thread {thread_id}] ERROR: Failed to lock runtime state mutex during reset");
    }

    // Clear the last shot
    if let Ok(mut last_shot) = LAST_SHOT.lock() {
        *last_shot = None;
    }

    if should_print_commands() {
        println!("[Thread {thread_id}] Reset QIR runtime (reset counters)");
    }
}

/// Gets the binary commands generated by the QIR runtime as a `ByteMessage`.
///
/// # Returns
///
/// A pointer to a `ByteMessage` containing the commands.
/// The caller is responsible for freeing the `ByteMessage` using `qir_runtime_free_binary_commands`.
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[repr(C)]
pub struct FFIByteData {
    pub data: *mut u32,
    pub word_count: usize,
    pub byte_len: usize,
}

/// # Safety
///
/// This function is unsafe because it returns a raw pointer to allocated memory that must be
/// properly freed by the caller using the appropriate deallocation function. The caller is
/// responsible for ensuring the returned pointer is not used after being freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_binary_commands() -> *mut FFIByteData {
    let thread_id = get_thread_id();

    // Build the message from the current context's message builder, or fallback to global
    let message = match with_current_context(|ctx| ctx.message_builder.build()) {
        Ok(msg) => msg,
        Err(_) => {
            // Fallback: Use global state for dynamic library execution
            if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                builder.build()
            } else {
                if should_print_commands() {
                    eprintln!(
                        "[Thread {thread_id}] ERROR: Failed to lock message builder mutex during get_binary_commands"
                    );
                    io::stderr().flush().unwrap_or_default();
                }
                ByteMessage::create_empty()
            }
        }
    };

    // Extract the aligned data directly from the message
    let bytes = message.into_bytes();
    let byte_len = bytes.len();

    // Transfer aligned u32 data across FFI boundary
    let (data_ptr, word_count) = if byte_len > 0 {
        // Calculate word count (round up)
        let word_count = byte_len.div_ceil(4);

        // Create aligned storage
        let mut aligned_data = vec![0u32; word_count];

        // Copy bytes into aligned storage using bytemuck
        let aligned_bytes = bytemuck::cast_slice_mut::<u32, u8>(&mut aligned_data);
        aligned_bytes[..byte_len].copy_from_slice(&bytes);

        // Convert to raw pointer
        let data_ptr = aligned_data.as_mut_ptr();
        std::mem::forget(aligned_data); // Don't drop, will be freed on other side

        (data_ptr, word_count)
    } else {
        (std::ptr::null_mut(), 0)
    };

    // Create the FFI structure
    let ffi_data = FFIByteData {
        data: data_ptr,
        word_count,
        byte_len,
    };

    // Allocate the FFI structure on the heap
    let boxed_ffi = Box::new(ffi_data);
    let ptr = Box::into_raw(boxed_ffi);

    if should_print_commands() {
        println!(
            "[Thread {thread_id}] Got binary commands as {byte_len} bytes ({word_count} words)"
        );
    }

    ptr
}

/// Frees a `ByteMessage` allocated by `qir_runtime_get_binary_commands`.
///
/// # Arguments
///
/// * `ptr` - The pointer to the `ByteMessage` to free
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_binary_commands(ptr: *mut FFIByteData) {
    let thread_id = get_thread_id();

    if ptr.is_null() {
        if should_print_commands() {
            eprintln!("[Thread {thread_id}] ERROR: Attempted to free null FFIByteData pointer");
            io::stderr().flush().unwrap_or_default();
        }
        return;
    }

    // Reconstruct the Box to get the FFIByteData
    let ffi_data = unsafe { Box::from_raw(ptr) };

    // Free the u32 data if it exists
    if !ffi_data.data.is_null() && ffi_data.word_count > 0 {
        // Reconstruct the Vec<u32> to properly deallocate
        let _aligned_data =
            unsafe { Vec::from_raw_parts(ffi_data.data, ffi_data.word_count, ffi_data.word_count) };
        // _aligned_data will be dropped here, properly deallocating the memory
    }

    if should_print_commands() {
        println!(
            "[Thread {thread_id}] Freed FFIByteData with {} bytes ({} words)",
            ffi_data.byte_len, ffi_data.word_count
        );
    }
}

/// Records a result output.
///
/// # Arguments
///
/// * `result` - The result pointer to record
/// * `name` - The name to record the result as, or null for default naming
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(result: *const u8, name: *const c_char) {
    let thread_id = get_thread_id();

    let result_id = result as usize;
    
    // Generate a name for the result
    let name_str = if name.is_null() {
        // If name is null, use a default name based on the result ID
        format!("result_{result_id}")
    } else {
        // Convert C string to Rust string
        let c_str = unsafe { CStr::from_ptr(name) };
        c_str.to_string_lossy().into_owned()
    };

    if should_print_commands() {
        println!("[Thread {thread_id}] Recording result {result_id} as '{name_str}'");
    }

    // Try to record in the current context, fallback to global state
    let context_available = with_current_context(|ctx| {
        // For now, just record the result with a simple mapping
        // In a real quantum system, this would get the actual measurement result
        // For demo purposes, simulate a result based on result_id
        let measurement_value = result_id % 2 == 0;
        ctx.record_measurement(result_id, measurement_value);
        ctx.set_register(name_str.clone(), if measurement_value { 1 } else { 0 });

        if should_print_commands() {
            println!(
                "QIR Runtime: [Context {}] Recorded result {result_id} as '{name_str}' = {measurement_value}", 
                ctx.context_id
            );
        }
    }).is_ok();
    
    if !context_available {
        // Fallback: Use global state for dynamic library execution
        if let Ok(mut state) = RUNTIME_STATE.lock() {
            // Simulate measurement result for demo
            let measurement_value = result_id % 2 == 0;
            state.measurement_results.insert(result_id, measurement_value);
            
            // Get the next bit position for this register
            let current_bit_position = {
                let bit_position = state
                    .register_bit_positions
                    .entry(name_str.clone())
                    .or_insert(0);
                let pos = *bit_position;
                *bit_position += 1;
                pos
            };

            // Store the mapping for when we get the actual measurement result
            state
                .result_mappings
                .insert(result_id, (name_str.clone(), current_bit_position));

            if should_print_commands() {
                println!(
                    "QIR Runtime: [Global] Recorded result {result_id} as '{name_str}' bit {current_bit_position} = {measurement_value}"
                );
            }
        }
    }
}

/// Updates the measurement results in the runtime state.
///
/// This function should be called by the QIR engine after processing measurements
/// from the quantum system.
///
/// # Arguments
///
/// * `results` - A slice of (`result_id`, `measurement_value`) pairs
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_update_measurement_results(
    results_ptr: *const u32,
    results_len: usize,
) {
    let thread_id = get_thread_id();

    if results_ptr.is_null() || results_len == 0 {
        return;
    }

    // Convert the raw pointer to a slice (pairs of result_id, value)
    let results = unsafe { std::slice::from_raw_parts(results_ptr, results_len * 2) };

    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Process pairs of (result_id, measurement_value)
        for i in (0..results.len()).step_by(2) {
            let result_id = results[i] as usize;
            let measurement_value = results[i + 1] != 0;

            state
                .measurement_results
                .insert(result_id, measurement_value);

            if should_print_commands() {
                println!(
                    "[Thread {thread_id}] Updated measurement result {result_id} = {measurement_value}"
                );
            }
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock runtime state mutex");
    }
}

/// Finalizes the QIR program execution and exports the shot results.
///
/// This function should be called when the QIR program's main function returns.
/// It exports the classical registers to a Shot and stores it for retrieval.
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_finalize_shot() {
    let thread_id = get_thread_id();

    if let Ok(mut state) = RUNTIME_STATE.lock() {
        // Apply the result mappings to build register values
        state.apply_mappings();

        let shot = state.export_shot();

        if should_print_commands() {
            println!(
                "[Thread {thread_id}] Finalizing shot with {} registers",
                state.classical_registers.len()
            );
            for (name, value) in &state.classical_registers {
                println!("[Thread {thread_id}]   Register '{name}' = {value}");
            }
        }

        // Store the shot for retrieval
        if let Ok(mut last_shot) = LAST_SHOT.lock() {
            *last_shot = Some(shot);
        } else {
            eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock last shot mutex");
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock runtime state mutex");
    }
}

/// Representation of a shot result for FFI
#[repr(C)]
pub struct FFIShotData {
    /// Pointer to register names (null-terminated C strings)
    names: *mut *mut c_char,
    /// Pointer to register values
    values: *mut i64,
    /// Number of registers
    count: usize,
}

/// Gets the shot results from the last finalized execution.
///
/// # Returns
///
/// A pointer to an `FFIShotData` structure containing the shot results,
/// or null if no shot is available.
///
/// # Safety
///
/// This function allocates memory that must be freed by calling `qir_runtime_free_shot_data`.
///
/// # Panics
///
/// This function may panic if:
/// - The array layout cannot be created (e.g., size overflow)
/// - Creating a C string from the register name fails (e.g., contains null bytes)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_shot_results() -> *mut FFIShotData {
    let thread_id = get_thread_id();

    if let Ok(last_shot) = LAST_SHOT.lock() {
        if let Some(shot) = last_shot.as_ref() {
            let count = shot.data.len();

            // Allocate arrays using Vec to ensure proper alignment
            let mut names_vec: Vec<*mut c_char> = Vec::with_capacity(count);
            let names = names_vec.as_mut_ptr();
            std::mem::forget(names_vec); // Prevent deallocation, we'll manage it manually

            let mut values_vec: Vec<i64> = Vec::with_capacity(count);
            let values = values_vec.as_mut_ptr();
            std::mem::forget(values_vec); // Prevent deallocation, we'll manage it manually

            // Populate the arrays
            for (i, (name, data)) in shot.data.iter().enumerate() {
                // Convert name to C string
                let c_name = std::ffi::CString::new(name.as_str()).unwrap();
                unsafe {
                    *names.add(i) = c_name.into_raw();
                }

                // Extract value
                let value = match data {
                    Data::U32(v) => i64::from(*v),
                    Data::I64(v) => *v,
                    _ => 0, // Default for other types
                };
                unsafe {
                    *values.add(i) = value;
                }
            }

            // Create and return the FFI structure
            let ffi_data = Box::new(FFIShotData {
                names,
                values,
                count,
            });

            if should_print_commands() {
                println!("[Thread {thread_id}] Exported shot with {count} registers");
            }

            Box::into_raw(ffi_data)
        } else {
            if should_print_commands() {
                println!("[Thread {thread_id}] No shot results available");
            }
            std::ptr::null_mut()
        }
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock last shot mutex");
        std::ptr::null_mut()
    }
}

/// Frees the shot data allocated by `qir_runtime_get_shot_results`.
///
/// # Arguments
///
/// * `data` - The pointer to the `FFIShotData` to free
///
/// # Safety
///
/// This function should only be called with a valid pointer returned by
/// `qir_runtime_get_shot_results`. Calling with an invalid pointer will
/// result in undefined behavior.
///
/// # Panics
///
/// This function may panic if the array layout cannot be created (e.g., size overflow).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_free_shot_data(data: *mut FFIShotData) {
    if data.is_null() {
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
            // Reconstruct Vec to properly deallocate - fix: use correct length parameter
            let _ = Vec::from_raw_parts(ffi_data.names, ffi_data.count, ffi_data.count);
            let _ = Vec::from_raw_parts(ffi_data.values, ffi_data.count, ffi_data.count);
        }

        // Box automatically frees the FFIShotData
    }
}

//
// QIR Convention Support
//
// The runtime supports both HUGR and QIR conventions:
// - HUGR: Uses integer-based qubit IDs directly (legacy behavior)
// - QIR: Uses opaque pointers that map to internal qubit IDs
//

/// Global mapping from QIR pointers to internal qubit IDs
static QUBIT_POINTER_MAP: LazyLock<Mutex<HashMap<usize, usize>>> = 
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Global mapping from QIR pointers to internal result IDs  
static RESULT_POINTER_MAP: LazyLock<Mutex<HashMap<usize, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Convert a qubit pointer to an internal qubit ID
/// This looks up the pointer in our mapping table
fn qubit_ptr_to_id(qubit_ptr: *const u8) -> Result<usize, String> {
    // In proper QIR, pointers are used as direct qubit indices via inttoptr
    // %Qubit* null = qubit 0
    // inttoptr (i64 1 to %Qubit*) = qubit 1, etc.
    let qubit_id = qubit_ptr as usize;
    Ok(qubit_id)
}

/// Convert a result pointer to an internal result ID
/// In proper QIR, pointers are used as direct result indices via inttoptr
fn result_ptr_to_id(result_ptr: *const u8) -> Result<usize, String> {
    // In proper QIR, result pointers are used as direct result indices via inttoptr
    // %Result* inttoptr (i64 0 to %Result*) = result 0
    // inttoptr (i64 1 to %Result*) = result 1, etc.
    let result_id = result_ptr as usize;
    Ok(result_id)
}

/// QIR-compatible qubit allocator that returns an opaque pointer
/// This allocates a qubit internally and maps it to a unique pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__qubit_allocate_ptr() -> *const u8 {
    // Allocate a qubit using the existing system
    let qubit_id = unsafe { __quantum__rt__qubit_allocate() };
    
    // Create a unique pointer for this qubit (using the qubit_id as the pointer value)
    let qubit_ptr = (qubit_id + 1) as *const u8; // +1 to avoid null pointer
    
    // Store the mapping
    {
        let mut map = QUBIT_POINTER_MAP.lock().unwrap();
        map.insert(qubit_ptr as usize, qubit_id);
    }
    
    if should_print_commands() {
        println!("QIR Runtime: Allocated qubit {} -> pointer {:p}", qubit_id, qubit_ptr);
    }
    
    qubit_ptr
}

/// QIR-compatible result allocator that returns an opaque pointer
/// This allocates a result internally and maps it to a unique pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_allocate_ptr() -> *const u8 {
    // Allocate a result using the existing system
    let result_id = unsafe { __quantum__rt__result_allocate() };
    
    // Create a unique pointer for this result (using a different offset to avoid conflicts)
    let result_ptr = (result_id + 0x10000) as *const u8; // Offset to avoid qubit pointer conflicts
    
    // Store the mapping
    {
        let mut map = RESULT_POINTER_MAP.lock().unwrap();
        map.insert(result_ptr as usize, result_id);
    }
    
    if should_print_commands() {
        println!("QIR Runtime: Allocated result {} -> pointer {:p}", result_id, result_ptr);
    }
    
    result_ptr
}

// Standard QIR function implementations with pointer interfaces
// These call the existing integer-based implementations












