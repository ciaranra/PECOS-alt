//! Bridge between Selene runtime plugins and PECOS ByteMessages
//!
//! This module provides the translation layer between Selene's runtime operations
//! and PECOS's ByteMessage format.

use crate::runtime_plugin::{RuntimeGetOperationInterface, RuntimeInstance, RuntimePlugin};
use anyhow::Result;
use log::{debug, trace};
use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;

/// Bridge that connects Selene runtime plugins to PECOS ByteMessages
pub struct SeleneRuntimeBridge {
    /// The loaded runtime plugin
    pub plugin: Arc<RuntimePlugin>,

    /// Runtime instance handle
    pub runtime_instance: RuntimeInstance,

    /// Number of qubits
    pub n_qubits: u64,

    /// ByteMessage builder for accumulating quantum operations
    pub message_builder: ByteMessageBuilder,

    /// Track measurement results
    pub measurement_results: HashMap<u64, bool>,

    /// Track which result IDs map to which measurements
    pub measurement_mapping: Vec<u64>,

    /// Flag to check if operations are pending
    pub has_pending_operations: bool,
}

impl SeleneRuntimeBridge {
    /// Create a new bridge with the specified runtime plugin
    pub fn new(plugin: Arc<RuntimePlugin>, n_qubits: u64) -> Result<Self> {
        let runtime_instance = plugin.init(n_qubits)?;

        let mut message_builder = ByteMessageBuilder::new();
        let _ = message_builder.for_quantum_operations();

        Ok(Self {
            plugin,
            runtime_instance,
            n_qubits,
            message_builder,
            measurement_results: HashMap::new(),
            measurement_mapping: Vec::new(),
            has_pending_operations: false,
        })
    }

    /// Start a new shot
    pub fn shot_start(&self, shot_id: u64, seed: u64) -> Result<()> {
        debug!("Starting shot {} with seed {}", shot_id, seed);
        self.plugin.shot_start(self.runtime_instance, shot_id, seed)
    }

    /// End the current shot
    pub fn shot_end(&self, shot_id: u64, seed: u64) -> Result<()> {
        debug!("Ending shot {}", shot_id);
        self.plugin.shot_end(self.runtime_instance, shot_id, seed)
    }

    /// Reset the bridge state for a new shot
    pub fn reset(&mut self) {
        self.message_builder = ByteMessageBuilder::new();
        let _ = self.message_builder.for_quantum_operations();
        self.measurement_results.clear();
        self.measurement_mapping.clear();
        self.has_pending_operations = false;
    }

    /// Get the accumulated ByteMessage
    pub fn get_byte_message(&mut self) -> ByteMessage {
        self.message_builder.build()
    }

    /// Process measurement results from the quantum engine
    pub fn process_measurement_results(&mut self, outcomes: Vec<u32>) {
        debug!("Processing {} measurement outcomes", outcomes.len());
        for (idx, outcome) in outcomes.iter().enumerate() {
            if let Some(&result_id) = self.measurement_mapping.get(idx) {
                self.measurement_results.insert(result_id, *outcome != 0);
                // Update the runtime with the result
                let _ = self.plugin.set_bool_result(
                    self.runtime_instance,
                    result_id,
                    *outcome != 0,
                );
            }
        }
    }

    /// Create callbacks for get_next_operations
    pub fn create_callbacks(&mut self) -> RuntimeGetOperationInterface {
        RuntimeGetOperationInterface {
            rxy_fn: Self::callback_rxy,
            rzz_fn: Self::callback_rzz,
            rz_fn: Self::callback_rz,
            measure_fn: Self::callback_measure,
            measure_leaked_fn: Self::callback_measure_leaked,
            reset_fn: Self::callback_reset,
            custom_fn: Self::callback_custom,
            set_batch_time_fn: Self::callback_set_batch_time,
        }
    }

    /// Get next operations from the runtime and convert to ByteMessage
    pub fn get_next_operations(&mut self) -> Result<bool> {
        // Reset the message builder for new operations
        self.message_builder = ByteMessageBuilder::new();
        let _ = self.message_builder.for_quantum_operations();

        let callbacks = self.create_callbacks();
        let bridge_ptr = self as *mut Self as *mut c_void;

        // Call runtime's get_next_operations
        // This will trigger callbacks that build up the ByteMessage
        let has_ops = self.plugin.get_next_operations(
            self.runtime_instance,
            bridge_ptr,
            &callbacks,
        )?;

        self.has_pending_operations = has_ops;
        Ok(has_ops)
    }

    // ===== Callback implementations =====
    // These are called by the runtime plugin during get_next_operations

    unsafe extern "C" fn callback_rxy(instance: *mut c_void, qubit_id: u64, theta: f64, phi: f64) {
        let bridge = unsafe { &mut *(instance as *mut SeleneRuntimeBridge) };
        trace!("RXY gate on qubit {} with theta={}, phi={}", qubit_id, theta, phi);

        // Convert to PECOS's R1XY gate
        let _ = bridge.message_builder.add_r1xy(theta, phi, &[qubit_id as usize]);
    }

    unsafe extern "C" fn callback_rzz(instance: *mut c_void, qubit_id_1: u64, qubit_id_2: u64, theta: f64) {
        let bridge = unsafe { &mut *(instance as *mut SeleneRuntimeBridge) };
        trace!("RZZ gate on qubits {}, {} with theta={}", qubit_id_1, qubit_id_2, theta);

        let _ = bridge.message_builder.add_rzz(
            theta,
            &[qubit_id_1 as usize],
            &[qubit_id_2 as usize],
        );
    }

    unsafe extern "C" fn callback_rz(instance: *mut c_void, qubit_id: u64, theta: f64) {
        let bridge = unsafe { &mut *(instance as *mut SeleneRuntimeBridge) };
        trace!("RZ gate on qubit {} with theta={}", qubit_id, theta);

        let _ = bridge.message_builder.add_rz(theta, &[qubit_id as usize]);
    }

    unsafe extern "C" fn callback_measure(instance: *mut c_void, qubit_id: u64, result_id: u64) {
        let bridge = unsafe { &mut *(instance as *mut SeleneRuntimeBridge) };
        trace!("Measure qubit {} -> result {}", qubit_id, result_id);

        // Track the mapping from measurement index to result ID
        bridge.measurement_mapping.push(result_id);

        let _ = bridge.message_builder.add_measurements(&[qubit_id as usize]);
    }

    unsafe extern "C" fn callback_measure_leaked(instance: *mut c_void, qubit_id: u64, result_id: u64) {
        let bridge = unsafe { &mut *(instance as *mut SeleneRuntimeBridge) };
        trace!("Measure leaked on qubit {} -> result {}", qubit_id, result_id);

        // Track the mapping
        bridge.measurement_mapping.push(result_id);

        // Use regular measurement for now (PECOS doesn't have separate leaked measurement)
        let _ = bridge.message_builder.add_measurements(&[qubit_id as usize]);
    }

    unsafe extern "C" fn callback_reset(instance: *mut c_void, qubit_id: u64) {
        let bridge = unsafe { &mut *(instance as *mut SeleneRuntimeBridge) };
        trace!("Reset qubit {}", qubit_id);

        // PECOS uses prep for reset
        let _ = bridge.message_builder.add_prep(&[qubit_id as usize]);
    }

    unsafe extern "C" fn callback_custom(
        _instance: *mut c_void,
        tag: u64,
        _data: *const c_void,
        _len: usize,
    ) {
        trace!("Custom operation with tag {}", tag);
        // Custom operations can be added later if needed
    }

    unsafe extern "C" fn callback_set_batch_time(_instance: *mut c_void, start: u64, duration: u64) {
        trace!("Set batch time: start={}, duration={}", start, duration);
        // Timing information - can be used for scheduling if needed
    }
}

impl Drop for SeleneRuntimeBridge {
    fn drop(&mut self) {
        // Clean up the runtime instance
        let _ = self.plugin.exit(self.runtime_instance);
    }
}