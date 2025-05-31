use log::info;
use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use std::env;
use std::ffi::{CStr, c_char};
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
/// Helper function to get the current thread ID as a string
fn get_thread_id() -> String {
    format!("{:?}", thread::current().id())
}

// Global counters for qubit and result allocation
static NEXT_QUBIT_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_RESULT_ID: AtomicUsize = AtomicUsize::new(0);

// Global message builder for quantum operations
static MESSAGE_BUILDER: std::sync::LazyLock<Mutex<ByteMessageBuilder>> =
    std::sync::LazyLock::new(|| {
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        Mutex::new(builder)
    });

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
/// This function stores the gate command in the global message builder
/// and optionally prints it to stdout for debugging.
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

    // Add the gate to the global message builder
    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        add_to_builder(&mut *builder);
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
    }

    // Print the command if not in quiet mode
    if should_print_commands() {
        println!("QIR Runtime: [Thread {thread_id}] {gate_name}");
    }
}

// Quantum gate operations

/// Applies a rotation around the Z-axis to the specified qubit.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__rz__body(theta: f64, qubit: usize) {
    store_gate_command(
        &format!("RZ {} {}", theta, qubit),
        |builder| { builder.add_rz(theta, &[qubit]); }
    );
}

/// Applies a rotation around an axis in the ZY plane to the specified qubit.
///
/// # Arguments
///
/// * `theta` - The rotation angle in radians
/// * `phi` - The phase angle in radians
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: usize) {
    store_gate_command(
        &format!("R1XY {} {} {}", theta, phi, qubit),
        |builder| { builder.add_r1xy(theta, phi, &[qubit]); }
    );
}

/// Applies a Hadamard gate to the specified qubit.
///
/// # Arguments
///
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__h__body(qubit: usize) {
    store_gate_command(
        &format!("H {}", qubit),
        |builder| { builder.add_h(&[qubit]); }
    );
}

/// Applies an X gate to the specified qubit.
///
/// # Arguments
///
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__x__body(qubit: usize) {
    store_gate_command(
        &format!("X {}", qubit),
        |builder| { builder.add_x(&[qubit]); }
    );
}

/// Applies a Y gate to the specified qubit.
///
/// # Arguments
///
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__y__body(qubit: usize) {
    store_gate_command(
        &format!("Y {}", qubit),
        |builder| { builder.add_y(&[qubit]); }
    );
}

/// Applies a Z gate to the specified qubit.
///
/// # Arguments
///
/// * `qubit` - The qubit index to apply the gate to
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID is valid
/// and has been properly allocated. Calling with an invalid qubit ID may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__z__body(qubit: usize) {
    store_gate_command(
        &format!("Z {}", qubit),
        |builder| { builder.add_z(&[qubit]); }
    );
}

/// Applies a controlled-X gate to the specified qubits.
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
pub unsafe extern "C" fn __quantum__qis__cx__body(control: usize, target: usize) {
    store_gate_command(
        &format!("CX {} {}", control, target),
        |builder| { builder.add_cx(&[control], &[target]); }
    );
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
pub unsafe extern "C" fn __quantum__qis__cz__body(control: usize, target: usize) {
    // Implement CZ as a sequence of H, CX, H
    store_gate_command(
        &format!("CZ {} {} (as H-CX-H)", control, target),
        |builder| { 
            builder.add_h(&[target]); 
            builder.add_cx(&[control], &[target]); 
            builder.add_h(&[target]); 
        }
    );
}

/// Applies a SZZ gate to the specified qubits.
///
/// # Arguments
///
/// * `qubit1` - The first qubit index
/// * `qubit2` - The second qubit index
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit IDs are valid
/// and have been properly allocated. Calling with invalid qubit IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__szz__body(qubit1: usize, qubit2: usize) {
    store_gate_command(
        &format!("SZZ {} {}", qubit1, qubit2),
        |builder| { builder.add_szz(&[qubit1], &[qubit2]); }
    );
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
    store_gate_command(
        &format!("RZZ {} {} {}", theta, qubit1, qubit2),
        |builder| { builder.add_rzz(theta, &[qubit1], &[qubit2]); }
    );
}

/// Measures a qubit and stores the result.
///
/// # Arguments
///
/// * `qubit` - The qubit index to measure
/// * `result` - The result ID to store the measurement result
///
/// # Safety
///
/// This function is called from C/C++ code and assumes that the qubit ID and result ID
/// are valid and have been properly allocated. Calling with invalid IDs may lead to
/// undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__qis__m__body(qubit: usize, _result: usize) -> u32 {
    store_gate_command(
        &format!("M {}", qubit),
        |builder| { builder.add_measurements(&[qubit]); }
    );
    // In the real QIR runtime, this would return the actual measurement result
    // For this implementation, we just return 0
    0
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
    store_gate_command(
        &format!("PREP {}", qubit),
        |builder| { builder.add_prep(&[qubit]); }
    );
}

/// Allocates a new qubit.
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
    let qubit_id = NEXT_QUBIT_ID.fetch_add(1, Ordering::SeqCst);
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Allocated qubit {qubit_id}");
    }

    qubit_id
}

/// Allocates a new result.
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
    let result_id = NEXT_RESULT_ID.fetch_add(1, Ordering::SeqCst);
    let thread_id = get_thread_id();

    if should_print_commands() {
        println!("[Thread {thread_id}] Allocated result {result_id}");
    }

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

    // Try to parse the data string as structured record data
    let parts: Vec<&str> = data_str.split_whitespace().collect();
    if parts.len() >= 2 && parts[0] == "RECORD" {
        if let Ok(result_id) = parts[1].parse::<usize>() {
            // This is a result record
            let label = if parts.len() >= 3 {
                Some(parts[2])
            } else {
                None
            };
            
            if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                builder.add_result_record(result_id, label);
            } else {
                eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
            }
            
            if should_print_commands() {
                if let Some(label_str) = label {
                    println!("QIR Runtime: [Thread {thread_id}] RECORD {} {}", result_id, label_str);
                } else {
                    println!("QIR Runtime: [Thread {thread_id}] RECORD {}", result_id);
                }
            }
        } else if parts.len() >= 3 {
            // Try to parse as a key-value record
            if let Ok(value) = parts[2].parse::<f64>() {
                if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                    builder.add_record_data(parts[1], value);
                } else {
                    eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
                }
                
                if should_print_commands() {
                    println!("QIR Runtime: [Thread {thread_id}] RECORD {} {}", parts[1], value);
                }
            } else {
                // Fall back to debug message
                if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                    builder.add_debug_message(&data_str);
                } else {
                    eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
                }
                
                if should_print_commands() {
                    println!("QIR Runtime: [Thread {thread_id}] RECORD (raw): {}", data_str);
                }
            }
        } else {
            // Fall back to debug message
            if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
                builder.add_debug_message(&data_str);
            } else {
                eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
            }
            
            if should_print_commands() {
                println!("QIR Runtime: [Thread {thread_id}] RECORD (raw): {}", data_str);
            }
        }
    } else {
        // Fall back to debug message
        if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
            builder.add_debug_message(&data_str);
        } else {
            eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
        }
        
        if should_print_commands() {
            println!("QIR Runtime: [Thread {thread_id}] RECORD (raw): {}", data_str);
        }
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
            eprintln!("[Thread {thread_id}] ERROR: Failed to lock message builder mutex during reset");
            io::stderr().flush().unwrap_or_default();
        }
    }

    // Reset qubit and result counters
    NEXT_QUBIT_ID.store(0, Ordering::SeqCst);
    NEXT_RESULT_ID.store(0, Ordering::SeqCst);

    if should_print_commands() {
        println!("[Thread {thread_id}] Reset QIR runtime (reset counters)");
    }
}

/// Gets the binary commands generated by the QIR runtime as a ByteMessage.
///
/// # Returns
///
/// A pointer to a ByteMessage containing the commands.
/// The caller is responsible for freeing the ByteMessage using `qir_runtime_free_binary_commands`.
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qir_runtime_get_binary_commands() -> *mut FFIByteData {
    let thread_id = get_thread_id();

    // Build the message from the global message builder
    let message = if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        // Build and return the current message
        builder.build()
    } else {
        // If we can't lock the mutex, return an empty message
        if should_print_commands() {
            eprintln!(
                "[Thread {thread_id}] ERROR: Failed to lock message builder mutex during get_binary_commands"
            );
            io::stderr().flush().unwrap_or_default();
        }
        ByteMessage::create_flush()
    };

    // Extract the aligned data directly from the message
    let bytes = message.into_bytes();
    let byte_len = bytes.len();

    // Transfer aligned u32 data across FFI boundary 
    let (data_ptr, word_count) = if byte_len > 0 {
        // Calculate word count (round up)
        let word_count = (byte_len + 3) / 4;
        
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
        println!("[Thread {thread_id}] Got binary commands as {} bytes ({} words)", byte_len, word_count);
    }

    ptr
}

/// Frees a ByteMessage allocated by `qir_runtime_get_binary_commands`.
///
/// # Arguments
///
/// * `ptr` - The pointer to the ByteMessage to free
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
        let _aligned_data = unsafe { 
            Vec::from_raw_parts(ffi_data.data, ffi_data.word_count, ffi_data.word_count)
        };
        // _aligned_data will be dropped here, properly deallocating the memory
    }

    if should_print_commands() {
        println!("[Thread {thread_id}] Freed FFIByteData with {} bytes ({} words)", ffi_data.byte_len, ffi_data.word_count);
    }
}

/// Records a result output.
///
/// # Arguments
///
/// * `result` - The result ID to record
/// * `name` - The name to record the result as, or null for default naming
///
/// # Safety
///
/// This function is called from C/C++ code. It is safe to call but marked as unsafe
/// due to the FFI boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __quantum__rt__result_record_output(result: usize, name: *const c_char) {
    let thread_id = get_thread_id();

    // Generate a name for the result
    let name_str = if name.is_null() {
        // If name is null, use a default name based on the result ID
        format!("result_{result}")
    } else {
        // Convert C string to Rust string
        let c_str = unsafe { CStr::from_ptr(name) };
        c_str.to_string_lossy().into_owned()
    };

    if should_print_commands() {
        println!("[Thread {thread_id}] Recording result {result} as '{name_str}'");
    }

    if let Ok(mut builder) = MESSAGE_BUILDER.lock() {
        builder.add_result_record(result, Some(&name_str));
    } else {
        eprintln!("QIR Runtime: [Thread {thread_id}] Failed to lock message builder mutex");
    }
}
