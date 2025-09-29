//! Selene FFI to `ByteMessage` Bridge
//!
//! This module provides the Selene FFI functions that plugins expect,
//! but instead of using Selene's simulator, it directly creates `ByteMessages`
//! and communicates with the PECOS infrastructure.
//!
//! This is NOT a stub - it's the actual implementation that bridges
//! between Selene's FFI interface and PECOS's `ByteMessage` system.

use once_cell::sync::OnceCell;
use pecos_engines::{ByteMessage, ByteMessageBuilder};
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};

// Global engine interface for communication with SeleneExecutableEngine
static ENGINE_INTERFACE: OnceCell<Arc<Mutex<dyn EngineInterface + Send + Sync>>> = OnceCell::new();

/// Interface for communicating with the `SeleneExecutableEngine`
pub trait EngineInterface {
    /// Queue an operation to be returned by `generate_commands()`
    fn queue_operation(&mut self, message: ByteMessage);

    /// Get measurement results
    fn get_measurement(&mut self, qubit: usize) -> bool;

    /// Store a measurement result for automatic QIS capture
    fn store_measurement(&mut self, qubit: usize, result: bool);
}

/// Initialize the engine interface
pub fn initialize_engine_interface(engine: Arc<Mutex<dyn EngineInterface + Send + Sync>>) {
    let _ = ENGINE_INTERFACE.set(engine);
}

/// Get the engine interface if available
pub fn get_engine_interface() -> Option<&'static Arc<Mutex<dyn EngineInterface + Send + Sync>>> {
    ENGINE_INTERFACE.get()
}

// NOTE: The Helios interface functions (___measure, ___rxy, etc.) are provided
// by pecos-qis-runtime when that crate is linked. The libhelios_selene_interface.a
// library expects these symbols to be available at runtime.

// FFI types matching Selene's interface
#[repr(C)]
pub struct SeleneInstance {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SeleneVoidResult {
    pub error_code: u32,
}

#[repr(C)]
pub struct SeleneU64Result {
    pub error_code: u32,
    pub value: u64,
}

#[repr(C)]
pub struct SeleneBoolResult {
    pub error_code: u32,
    pub value: bool,
}

// Qubit allocation tracking
static NEXT_QUBIT: OnceCell<Mutex<u64>> = OnceCell::new();

fn get_next_qubit() -> u64 {
    let counter = NEXT_QUBIT.get_or_init(|| Mutex::new(0));
    let mut val = counter.lock().unwrap();
    let current = *val;
    *val += 1;
    current
}

/// Queue a `ByteMessage` operation to the engine
fn queue_operation(message: ByteMessage) {
    if let Some(engine) = ENGINE_INTERFACE.get()
        && let Ok(mut engine) = engine.lock()
    {
        engine.queue_operation(message);
    } else {
        log::warn!("Failed to queue operation - no engine interface available");
    }
}

// ===== Core Selene FFI Functions =====

#[unsafe(no_mangle)]
pub extern "C" fn selene_qalloc(_instance: *mut SeleneInstance) -> SeleneU64Result {
    let qubit = get_next_qubit();

    // Prepare qubit in |0⟩ state
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_prep(&[usize::try_from(qubit).unwrap_or(usize::MAX)]);
    queue_operation(builder.build());

    SeleneU64Result {
        error_code: 0,
        value: qubit,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qfree(_instance: *mut SeleneInstance, _qubit: u64) -> SeleneVoidResult {
    // No operation needed for qubit deallocation in PECOS
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_reset(
    _instance: *mut SeleneInstance,
    qubit: u64,
) -> SeleneVoidResult {
    // Prepare qubit in |0⟩ state
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_prep(&[usize::try_from(qubit).unwrap_or(usize::MAX)]);
    queue_operation(builder.build());

    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_rxy(
    _instance: *mut SeleneInstance,
    qubit: u64,
    theta: f64,
    phi: f64,
) -> SeleneVoidResult {
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_r1xy(theta, phi, &[usize::try_from(qubit).unwrap_or(usize::MAX)]);
    queue_operation(builder.build());

    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_rz(
    _instance: *mut SeleneInstance,
    qubit: u64,
    theta: f64,
) -> SeleneVoidResult {
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_rz(theta, &[usize::try_from(qubit).unwrap_or(usize::MAX)]);
    queue_operation(builder.build());

    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_rzz(
    _instance: *mut SeleneInstance,
    q1: u64,
    q2: u64,
    theta: f64,
) -> SeleneVoidResult {
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_rzz(
        theta,
        &[usize::try_from(q1).unwrap_or(usize::MAX)],
        &[usize::try_from(q2).unwrap_or(usize::MAX)],
    );
    queue_operation(builder.build());

    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_measure(
    _instance: *mut SeleneInstance,
    qubit: u64,
) -> SeleneBoolResult {
    // Queue measurement
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_measurements(&[usize::try_from(qubit).unwrap_or(usize::MAX)]);
    queue_operation(builder.build());

    // Get result from engine
    let result = if let Some(engine) = ENGINE_INTERFACE.get() {
        if let Ok(mut engine) = engine.lock() {
            engine.get_measurement(usize::try_from(qubit).unwrap_or(usize::MAX))
        } else {
            false
        }
    } else {
        false
    };

    SeleneBoolResult {
        error_code: 0,
        value: result,
    }
}

// Lazy measurement (returns a future reference)
#[unsafe(no_mangle)]
pub extern "C" fn selene_qubit_lazy_measure(
    _instance: *mut SeleneInstance,
    qubit: u64,
) -> SeleneU64Result {
    // Queue measurement
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_measurements(&[usize::try_from(qubit).unwrap_or(usize::MAX)]);
    queue_operation(builder.build());

    // Return the qubit ID as the reference (simple mapping)
    SeleneU64Result {
        error_code: 0,
        value: qubit,
    }
}

// QIS measurement recording function for automatic result capture
#[unsafe(no_mangle)]
pub extern "C" fn __quantum__qis__record_measurement(qubit: i64, result: bool) {
    log::debug!("Recording QIS measurement: qubit {} = {}", qubit, result);

    // Store the measurement in the engine's measurement_results
    if let Some(engine) = ENGINE_INTERFACE.get() {
        if let Ok(mut engine) = engine.lock() {
            engine.store_measurement(usize::try_from(qubit).unwrap_or(usize::MAX), result);
        }
    }
}

// Time cursor functions (no-op for now)
#[unsafe(no_mangle)]
pub extern "C" fn selene_get_tc(_instance: *mut SeleneInstance) -> SeleneU64Result {
    SeleneU64Result {
        error_code: 0,
        value: 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_set_tc(_instance: *mut SeleneInstance, _tc: u64) -> SeleneVoidResult {
    SeleneVoidResult { error_code: 0 }
}

// Configuration and lifecycle functions
/// Load a configuration file for Selene
///
/// # Safety
///
/// - `instance` must be a valid pointer to a mutable pointer
/// - The caller must ensure the instance pointer remains valid for the lifetime of the Selene instance
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_load_config(
    instance: *mut *mut SeleneInstance,
    _config_file: *const std::ffi::c_char,
) -> SeleneVoidResult {
    unsafe {
        *instance = std::ptr::dangling_mut::<SeleneInstance>(); // Non-null dummy pointer
        SeleneVoidResult { error_code: 0 }
    }
}

/// Signal the start of a new shot
///
/// # Panics
///
/// Panics if the measurement counter lock is poisoned
#[unsafe(no_mangle)]
pub extern "C" fn selene_on_shot_start(
    _instance: *mut SeleneInstance,
    _shot: u64,
) -> SeleneVoidResult {
    // Reset qubit counter for new shot
    if let Some(counter) = NEXT_QUBIT.get() {
        *counter.lock().unwrap() = 0;
    }
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_on_shot_end(_instance: *mut SeleneInstance) -> SeleneVoidResult {
    SeleneVoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_shot_count(_instance: *mut SeleneInstance) -> SeleneU64Result {
    SeleneU64Result {
        error_code: 0,
        value: 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn selene_exit(_instance: *mut SeleneInstance) -> SeleneVoidResult {
    SeleneVoidResult { error_code: 0 }
}
