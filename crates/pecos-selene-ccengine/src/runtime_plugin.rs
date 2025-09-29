//! Selene Runtime Plugin Interface
//!
//! This module provides FFI bindings to load and interact with Selene runtime plugins.
//! Based on selene-core's runtime plugin interface.

use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void, OsStr};
use std::sync::Arc;

/// Wrapper for runtime instance pointer to make it Send/Sync
/// SAFETY: The runtime plugin is expected to be thread-safe
#[derive(Clone, Copy)]
pub struct RuntimeInstance(pub *mut c_void);

unsafe impl Send for RuntimeInstance {}
unsafe impl Sync for RuntimeInstance {}
pub type Errno = i32;

/// FFI interface for runtime operations callbacks
/// This is provided to the runtime's get_next_operations function
#[repr(C)]
pub struct RuntimeGetOperationInterface {
    pub rxy_fn: unsafe extern "C" fn(instance: *mut c_void, qubit_id: u64, theta: f64, phi: f64),
    pub rzz_fn: unsafe extern "C" fn(instance: *mut c_void, qubit_id_1: u64, qubit_id_2: u64, theta: f64),
    pub rz_fn: unsafe extern "C" fn(instance: *mut c_void, qubit_id: u64, theta: f64),
    pub measure_fn: unsafe extern "C" fn(instance: *mut c_void, qubit_id: u64, result_id: u64),
    pub measure_leaked_fn: unsafe extern "C" fn(instance: *mut c_void, qubit_id: u64, result_id: u64),
    pub reset_fn: unsafe extern "C" fn(instance: *mut c_void, qubit_id: u64),
    pub custom_fn: unsafe extern "C" fn(instance: *mut c_void, tag: u64, data: *const c_void, len: usize),
    pub set_batch_time_fn: unsafe extern "C" fn(instance: *mut c_void, start: u64, duration: u64),
}

/// Wrapper for a dynamically loaded Selene runtime plugin
pub struct RuntimePlugin {
    _lib: Arc<Library>,

    // Function pointers
    init_fn: Symbol<'static, unsafe extern "C" fn(*mut *mut c_void, u64, u64, u32, *const *const c_char) -> Errno>,
    exit_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> Errno>,
    shot_start_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, u64) -> Errno>,
    shot_end_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, u64) -> Errno>,
    get_next_operations_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, *mut c_void, *const RuntimeGetOperationInterface) -> Errno>,

    // Qubit management
    qalloc_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, *mut u64) -> Errno>,
    qfree_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64) -> Errno>,

    // Gate operations
    rxy_gate_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, f64, f64) -> Errno>,
    rzz_gate_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, u64, f64) -> Errno>,
    rz_gate_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, f64) -> Errno>,

    // Measurement operations
    measure_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, *mut u64) -> Errno>,
    measure_leaked_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, *mut u64) -> Errno>,
    reset_fn: Symbol<'static, unsafe extern "C" fn(*mut c_void, u64) -> Errno>,

    // Result operations
    get_bool_result_fn: Option<Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, *mut bool) -> Errno>>,
    set_bool_result_fn: Option<Symbol<'static, unsafe extern "C" fn(*mut c_void, u64, bool) -> Errno>>,
}

impl RuntimePlugin {
    /// Load a runtime plugin from a shared library file
    pub fn load(plugin_path: impl AsRef<OsStr>) -> Result<Arc<Self>> {
        unsafe {
            let lib = Arc::new(Library::new(plugin_path.as_ref()).map_err(|e| {
                anyhow!("Failed to load runtime plugin: {}", e)
            })?);

            // Load all required function symbols
            // We need to transmute to 'static lifetime for the symbols
            let lib_ref: &'static Library = std::mem::transmute(lib.as_ref());

            Ok(Arc::new(Self {
                init_fn: lib_ref.get(b"selene_runtime_init")?,
                exit_fn: lib_ref.get(b"selene_runtime_exit")?,
                shot_start_fn: lib_ref.get(b"selene_runtime_shot_start")?,
                shot_end_fn: lib_ref.get(b"selene_runtime_shot_end")?,
                get_next_operations_fn: lib_ref.get(b"selene_runtime_get_next_operations")?,
                qalloc_fn: lib_ref.get(b"selene_runtime_qalloc")?,
                qfree_fn: lib_ref.get(b"selene_runtime_qfree")?,
                rxy_gate_fn: lib_ref.get(b"selene_runtime_rxy_gate")?,
                rzz_gate_fn: lib_ref.get(b"selene_runtime_rzz_gate")?,
                rz_gate_fn: lib_ref.get(b"selene_runtime_rz_gate")?,
                measure_fn: lib_ref.get(b"selene_runtime_measure")?,
                measure_leaked_fn: lib_ref.get(b"selene_runtime_measure_leaked")?,
                reset_fn: lib_ref.get(b"selene_runtime_reset")?,

                // Optional functions
                get_bool_result_fn: lib_ref.get(b"selene_runtime_get_bool_result").ok(),
                set_bool_result_fn: lib_ref.get(b"selene_runtime_set_bool_result").ok(),

                _lib: lib,
            }))
        }
    }

    /// Initialize a runtime instance
    pub fn init(&self, n_qubits: u64) -> Result<RuntimeInstance> {
        let mut instance_ptr: *mut c_void = std::ptr::null_mut();
        let errno = unsafe {
            (self.init_fn)(
                &mut instance_ptr as *mut *mut c_void,
                n_qubits,
                0, // start time
                0, // argc
                std::ptr::null(), // argv
            )
        };

        if errno != 0 {
            return Err(anyhow!("Runtime init failed with error code {}", errno));
        }

        Ok(RuntimeInstance(instance_ptr))
    }

    /// Start a shot
    pub fn shot_start(&self, instance: RuntimeInstance, shot_id: u64, seed: u64) -> Result<()> {
        let errno = unsafe { (self.shot_start_fn)(instance.0, shot_id, seed) };
        if errno != 0 {
            return Err(anyhow!("Shot start failed with error code {}", errno));
        }
        Ok(())
    }

    /// End a shot
    pub fn shot_end(&self, instance: RuntimeInstance, shot_id: u64, seed: u64) -> Result<()> {
        let errno = unsafe { (self.shot_end_fn)(instance.0, shot_id, seed) };
        if errno != 0 {
            return Err(anyhow!("Shot end failed with error code {}", errno));
        }
        Ok(())
    }

    /// Get next operations from the runtime
    pub fn get_next_operations(
        &self,
        instance: RuntimeInstance,
        callback_instance: *mut c_void,
        callbacks: &RuntimeGetOperationInterface,
    ) -> Result<bool> {
        let errno = unsafe {
            (self.get_next_operations_fn)(
                instance.0,
                callback_instance,
                callbacks as *const RuntimeGetOperationInterface,
            )
        };

        if errno != 0 {
            return Err(anyhow!("Get next operations failed with error code {}", errno));
        }

        // TODO: How to determine if there are more operations?
        // For now, assume errno 0 means success and operations were retrieved
        Ok(true)
    }

    /// Allocate a qubit
    pub fn qalloc(&self, instance: RuntimeInstance) -> Result<u64> {
        let mut qubit_id: u64 = 0;
        let errno = unsafe { (self.qalloc_fn)(instance.0, &mut qubit_id) };
        if errno != 0 {
            return Err(anyhow!("Qalloc failed with error code {}", errno));
        }
        Ok(qubit_id)
    }

    /// Free a qubit
    pub fn qfree(&self, instance: RuntimeInstance, qubit_id: u64) -> Result<()> {
        let errno = unsafe { (self.qfree_fn)(instance.0, qubit_id) };
        if errno != 0 {
            return Err(anyhow!("Qfree failed with error code {}", errno));
        }
        Ok(())
    }

    /// Apply RXY gate
    pub fn rxy_gate(&self, instance: RuntimeInstance, qubit: u64, theta: f64, phi: f64) -> Result<()> {
        let errno = unsafe { (self.rxy_gate_fn)(instance.0, qubit, theta, phi) };
        if errno != 0 {
            return Err(anyhow!("RXY gate failed with error code {}", errno));
        }
        Ok(())
    }

    /// Apply RZ gate
    pub fn rz_gate(&self, instance: RuntimeInstance, qubit: u64, theta: f64) -> Result<()> {
        let errno = unsafe { (self.rz_gate_fn)(instance.0, qubit, theta) };
        if errno != 0 {
            return Err(anyhow!("RZ gate failed with error code {}", errno));
        }
        Ok(())
    }

    /// Apply RZZ gate
    pub fn rzz_gate(&self, instance: RuntimeInstance, q1: u64, q2: u64, theta: f64) -> Result<()> {
        let errno = unsafe { (self.rzz_gate_fn)(instance.0, q1, q2, theta) };
        if errno != 0 {
            return Err(anyhow!("RZZ gate failed with error code {}", errno));
        }
        Ok(())
    }

    /// Measure a qubit
    pub fn measure(&self, instance: RuntimeInstance, qubit: u64) -> Result<u64> {
        let mut result_id: u64 = 0;
        let errno = unsafe { (self.measure_fn)(instance.0, qubit, &mut result_id) };
        if errno != 0 {
            return Err(anyhow!("Measure failed with error code {}", errno));
        }
        Ok(result_id)
    }

    /// Reset a qubit
    pub fn reset(&self, instance: RuntimeInstance, qubit: u64) -> Result<()> {
        let errno = unsafe { (self.reset_fn)(instance.0, qubit) };
        if errno != 0 {
            return Err(anyhow!("Reset failed with error code {}", errno));
        }
        Ok(())
    }

    /// Set a boolean measurement result
    pub fn set_bool_result(&self, instance: RuntimeInstance, result_id: u64, value: bool) -> Result<()> {
        if let Some(ref set_fn) = self.set_bool_result_fn {
            let errno = unsafe { set_fn(instance.0, result_id, value) };
            if errno != 0 {
                return Err(anyhow!("Set bool result failed with error code {}", errno));
            }
        }
        Ok(())
    }

    /// Get a boolean measurement result
    pub fn get_bool_result(&self, instance: RuntimeInstance, result_id: u64) -> Result<Option<bool>> {
        if let Some(ref get_fn) = self.get_bool_result_fn {
            let mut value: bool = false;
            let errno = unsafe { get_fn(instance.0, result_id, &mut value) };
            if errno != 0 {
                return Err(anyhow!("Get bool result failed with error code {}", errno));
            }
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Exit and cleanup
    pub fn exit(&self, instance: RuntimeInstance) -> Result<()> {
        let errno = unsafe { (self.exit_fn)(instance.0) };
        if errno != 0 {
            return Err(anyhow!("Exit failed with error code {}", errno));
        }
        Ok(())
    }
}