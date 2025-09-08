//! Instance-based LLVM Runtime State
//!
//! This module provides an instance-based runtime state for LLVM IR execution,
//! eliminating the need for global state and enabling proper concurrent execution.

use pecos_core::errors::PecosError;
use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use pecos_engines::shot_results::{Data, Shot};
use std::collections::{HashMap, VecDeque};

/// Type alias for the interactive execution callback
pub type InteractiveCallback =
    Box<dyn Fn(ByteMessage) -> Result<Vec<u32>, PecosError> + Send + Sync>;

/// LLVM Runtime State
///
/// Contains all the state needed for LLVM IR execution, previously stored in globals.
/// Each `LlvmEngine` instance will have its own `RuntimeState`.
pub struct LlvmRuntimeState {
    /// Counter for qubit allocation
    next_qubit_id: usize,

    /// Counter for result allocation
    next_result_id: usize,

    /// Message builder for quantum operations
    message_builder: ByteMessageBuilder,

    /// Measurement results indexed by `result_id`
    measurement_results: HashMap<usize, bool>,

    /// Classical registers
    classical_registers: HashMap<String, i64>,

    /// Tracks bit positions in registers
    register_bit_positions: HashMap<String, usize>,

    /// Maps result IDs to register names and bit positions
    result_mappings: HashMap<usize, (String, usize)>,

    /// Last shot result for retrieval
    last_shot: Option<Shot>,

    /// Interactive execution callback for immediate measurements
    interactive_callback: Option<InteractiveCallback>,

    /// Tuple return values from main function
    tuple_return: Vec<i32>,

    /// Maps tuple indices to result IDs for placeholder resolution
    /// When a tuple contains measurement results, this tracks which result ID
    /// each tuple index corresponds to
    tuple_placeholder_mapping: HashMap<usize, usize>,

    /// Tracks which result IDs are being accessed for tuple returns
    /// This helps us identify which tuple values need to be updated
    tuple_accessed_results: Vec<usize>,

    /// Maximum number of qubits allowed (set during initialization)
    max_qubits: Option<usize>,

    /// Pool of released qubit IDs available for reuse
    released_qubits: VecDeque<usize>,

    /// Tracks the order of measurements and their corresponding result IDs
    /// This is needed because `ByteMessage` only returns measurement outcomes in order,
    /// but we need to map them back to their allocated result IDs
    measurement_result_ids: Vec<usize>,

    /// Track how many measurements have been executed so far
    /// This is needed for interleaved measurement execution
    measurements_executed: usize,
}

impl LlvmRuntimeState {
    /// Create a new runtime state
    #[must_use]
    pub fn new() -> Self {
        let mut message_builder = ByteMessageBuilder::new();
        let _ = message_builder.for_quantum_operations();

        Self {
            next_qubit_id: 0,
            next_result_id: 0,
            message_builder,
            measurement_results: HashMap::new(),
            classical_registers: HashMap::new(),
            register_bit_positions: HashMap::new(),
            result_mappings: HashMap::new(),
            last_shot: None,
            interactive_callback: None,
            tuple_return: Vec::new(),
            tuple_placeholder_mapping: HashMap::new(),
            tuple_accessed_results: Vec::new(),
            max_qubits: None,
            released_qubits: VecDeque::new(),
            measurement_result_ids: Vec::new(),
            measurements_executed: 0,
        }
    }

    /// Reset the runtime state for a new execution
    pub fn reset(&mut self) {
        self.next_qubit_id = 0;
        self.next_result_id = 0;
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();
        self.measurement_results.clear();
        self.classical_registers.clear();
        self.register_bit_positions.clear();
        self.result_mappings.clear();
        self.last_shot = None;
        self.tuple_return.clear();
        self.tuple_placeholder_mapping.clear();
        self.tuple_accessed_results.clear();
        self.released_qubits.clear();
        self.measurement_result_ids.clear();
        self.measurements_executed = 0;
    }

    /// Allocate a new qubit and return its ID
    pub fn allocate_qubit(&mut self) -> usize {
        // BUGFIX: Disable qubit reuse to prevent measuring the same qubit multiple times
        // When qubits are released immediately after measurement but before quantum
        // operations are sent to the engine, reusing them causes the wrong qubit to
        // be measured. This was causing tuple returns like (True, False) to become (True, True).
        //
        // TODO: Re-enable qubit reuse after quantum operations have been executed
        // if let Some(reused_id) = self.released_qubits.pop_front() {
        //     return reused_id;
        // }

        // Always allocate a new qubit
        let id = self.next_qubit_id;

        // Check qubit limit if set
        if let Some(max_qubits) = self.max_qubits {
            assert!(
                (id < max_qubits),
                "Qubit allocation limit exceeded! Attempted to allocate qubit {} but max_qubits is set to {}. \
                     Increase max_qubits using .max_qubits({}) or higher when building the simulation.",
                id,
                max_qubits,
                id + 1
            );
        }

        self.next_qubit_id += 1;
        id
    }

    /// Set the maximum number of qubits allowed
    pub fn set_max_qubits(&mut self, max_qubits: usize) {
        self.max_qubits = Some(max_qubits);
    }

    /// Get the maximum number of qubits allowed
    #[must_use]
    pub fn get_max_qubits(&self) -> Option<usize> {
        self.max_qubits
    }

    /// Release a qubit for reuse
    pub fn release_qubit(&mut self, _qubit_id: usize) {
        // BUGFIX: Don't add to released pool since we're not reusing qubits
        // See allocate_qubit for details on why qubit reuse is disabled
        // self.released_qubits.push_back(qubit_id);

        // Just track that the qubit was released for debugging
    }

    /// Check if there are any released qubits available for reuse
    #[must_use]
    pub fn has_released_qubits(&self) -> bool {
        !self.released_qubits.is_empty()
    }

    /// Allocate a new result and return its ID
    pub fn allocate_result(&mut self) -> usize {
        let id = self.next_result_id;
        self.next_result_id += 1;
        id
    }

    /// Get a mutable reference to the message builder
    pub fn message_builder_mut(&mut self) -> &mut ByteMessageBuilder {
        &mut self.message_builder
    }

    /// Build and return the current message
    pub fn build_message(&mut self) -> ByteMessage {
        self.message_builder.build()
    }

    /// Store a measurement result
    pub fn store_measurement(&mut self, result_id: usize, value: bool) {
        use log::debug;
        debug!("store_measurement: result_id={result_id} value={value}");
        self.measurement_results.insert(result_id, value);
    }

    /// Get a measurement result
    #[must_use]
    pub fn get_measurement_result(&self, result_id: usize) -> Option<bool> {
        use log::debug;
        let result = self.measurement_results.get(&result_id).copied();
        debug!("get_measurement_result: result_id={result_id} -> {result:?}");
        debug!(
            "get_measurement_result: all results = {:?}",
            self.measurement_results
        );
        result
    }

    /// Get the tuple accessed results for debugging
    #[must_use]
    pub fn get_tuple_accessed_results(&self) -> &[usize] {
        &self.tuple_accessed_results
    }

    /// Get all measurement results for debugging
    #[must_use]
    pub fn get_all_measurement_results(&self) -> &HashMap<usize, bool> {
        &self.measurement_results
    }

    /// Track that a result ID is being accessed for a tuple return
    pub fn track_tuple_access(&mut self, result_id: usize) {
        use log::debug;
        debug!(
            "track_tuple_access: result_id={}, current list={:?}",
            result_id, self.tuple_accessed_results
        );
        debug!(
            "track_tuple_access: measurement_result_ids={:?}",
            self.measurement_result_ids
        );
        if !self.tuple_accessed_results.contains(&result_id) {
            self.tuple_accessed_results.push(result_id);
            debug!(
                "track_tuple_access: added result_id={}, new list={:?}",
                result_id, self.tuple_accessed_results
            );
        }
    }

    /// Add a measurement with its result ID to track the order
    pub fn add_measurement(&mut self, qubit_id: usize, result_id: usize) {
        // Add the measurement to the message
        let _ = self.message_builder.add_measurements(&[qubit_id]);
        // Track the result ID for this measurement
        self.measurement_result_ids.push(result_id);
    }

    /// Map a result to a register and bit position
    pub fn map_result_to_register(
        &mut self,
        result_id: usize,
        register_name: String,
        bit_position: usize,
    ) {
        self.result_mappings
            .insert(result_id, (register_name.clone(), bit_position));

        // Update the maximum bit position for this register
        self.register_bit_positions
            .entry(register_name)
            .and_modify(|pos| *pos = (*pos).max(bit_position))
            .or_insert(bit_position);
    }

    /// Update measurement results from external data
    pub fn update_measurement_results(&mut self, results: &[u32]) {
        use log::debug;

        // Process pairs of (result_id, measurement_value)
        debug!(
            "update_measurement_results: Processing {} values",
            results.len()
        );
        debug!("update_measurement_results: Raw input: {results:?}");
        debug!(
            "update_measurement_results: Current measurement_results before update: {:?}",
            self.measurement_results
        );
        for i in (0..results.len()).step_by(2) {
            if i + 1 < results.len() {
                let result_id = results[i] as usize;
                let raw_value = results[i + 1];
                let measurement_value = raw_value != 0;
                debug!(
                    "store_measurement: pair[{}] result_id={} raw_value={} bool_value={}",
                    i / 2,
                    result_id,
                    raw_value,
                    measurement_value
                );
                self.measurement_results
                    .insert(result_id, measurement_value);
            }
        }
        debug!(
            "update_measurement_results: Final measurement_results: {:?}",
            self.measurement_results
        );

        // Check if we have tuple return values to update with measurement results

        if !self.tuple_return.is_empty() && !self.tuple_placeholder_mapping.is_empty() {
            debug!(
                "Updating tuple with placeholder mapping: {:?}",
                self.tuple_placeholder_mapping
            );
            debug!("Current tuple values: {:?}", self.tuple_return);
            debug!("Available measurements: {:?}", self.measurement_results);
            let mut updated_tuple = self.tuple_return.clone();
            let mut updates_made = false;

            // Update each tuple position that maps to a measurement result
            for (&tuple_idx, &result_id) in &self.tuple_placeholder_mapping {
                debug!("Checking tuple_idx={tuple_idx} -> result_id={result_id}");
                if let Some(&measurement) = self.measurement_results.get(&result_id) {
                    let new_value = i32::from(measurement);
                    if tuple_idx < updated_tuple.len() {
                        let old_value = updated_tuple[tuple_idx];
                        updated_tuple[tuple_idx] = new_value;
                        debug!(
                            "Updating tuple[{tuple_idx}]: old={old_value}, new={new_value} (result_id={result_id}, measurement={measurement})"
                        );
                        if old_value != new_value {
                            updates_made = true;
                        }
                    } else {
                        debug!(
                            "ERROR: tuple_idx {} out of bounds (tuple len={})",
                            tuple_idx,
                            updated_tuple.len()
                        );
                    }
                } else {
                    debug!(
                        "WARNING: No measurement found for result_id={result_id} (tuple_idx={tuple_idx})"
                    );
                    debug!(
                        "Available measurements: {:?}",
                        self.measurement_results.keys().collect::<Vec<_>>()
                    );
                }
            }

            // Always update (for debugging)
            debug!("Final tuple return values: {updated_tuple:?} (updates_made={updates_made})");
            debug!("Original tuple was: {:?}", self.tuple_return);
            self.tuple_return = updated_tuple;
        }
    }

    /// Apply result mappings to build register values
    pub fn apply_mappings(&mut self) {
        // Clear existing register values
        self.classical_registers.clear();

        // First, initialize all registers that will be used to 0
        for (register_name, _) in self.result_mappings.values() {
            self.classical_registers.insert(register_name.clone(), 0);
        }

        // Apply all result mappings to build register values
        for (result_id, (register_name, bit_position)) in &self.result_mappings {
            // Get the measurement result
            let measurement_value = self
                .measurement_results
                .get(result_id)
                .copied()
                .unwrap_or(false);

            // Get the register (we know it exists now)
            if let Some(register) = self.classical_registers.get_mut(register_name) {
                // Set or clear the bit
                if measurement_value {
                    *register |= 1i64 << bit_position;
                } else {
                    // Since we initialized to 0, we don't need to clear bits
                    // But we'll keep this for clarity
                    *register &= !(1i64 << bit_position);
                }
            }
        }
    }

    /// Export the current state as a Shot
    #[must_use]
    pub fn export_shot(&self) -> Shot {
        use log::debug;
        let mut shot = Shot::default();

        // First priority: If we have tuple return values from the main function,
        // export them as "result"
        if !self.tuple_return.is_empty() {
            debug!(
                "Exporting shot with tuple return values: {:?}",
                self.tuple_return
            );
            shot.data.insert(
                "result".to_string(),
                Data::from_i32_vec(self.tuple_return.clone()),
            );
            return shot;
        }

        // Second priority: Export all classical registers with their actual names
        // The LLVM code is responsible for naming and building these values
        if !self.classical_registers.is_empty() {
            // Check if we have a pattern of "_result_N" registers that should be combined
            let mut result_registers: Vec<(usize, i64)> = Vec::new();
            let mut other_registers: Vec<(String, i64)> = Vec::new();

            for (name, value) in &self.classical_registers {
                if let Some(stripped) = name.strip_prefix("_result_") {
                    if let Ok(index) = stripped.parse::<usize>() {
                        result_registers.push((index, *value));
                    } else {
                        other_registers.push((name.clone(), *value));
                    }
                } else {
                    other_registers.push((name.clone(), *value));
                }
            }

            // If we only have _result_N registers, combine them into a single "result" array
            if !result_registers.is_empty() && other_registers.is_empty() {
                // Sort by index to ensure correct ordering
                result_registers.sort_by_key(|(idx, _)| *idx);

                // Convert to i32 values
                let values: Vec<i32> = result_registers
                    .iter()
                    .map(|(_, value)| *value as i32)
                    .collect();

                debug!("Combining _result_N registers into result array: {values:?}");
                shot.data
                    .insert("result".to_string(), Data::from_i32_vec(values));
                return shot;
            }

            // Otherwise, export all registers as-is
            for (name, value) in &self.classical_registers {
                shot.data.insert(name.clone(), Data::I64(*value));
            }
            return shot;
        }

        // Last resort: If no named registers and no tuple return,
        // collect all measurements into a single "result" array
        if !self.measurement_results.is_empty() {
            // Sort by result_id to ensure consistent ordering
            let mut sorted_results: Vec<_> = self.measurement_results.iter().collect();
            sorted_results.sort_by_key(|(id, _)| *id);

            // Convert to i32 values (0 or 1)
            let values: Vec<i32> = sorted_results
                .iter()
                .map(|(_, value)| i32::from(**value))
                .collect();

            debug!("Exporting measurements as result array: {values:?}");
            shot.data
                .insert("result".to_string(), Data::from_i32_vec(values));
        }

        shot
    }

    /// Finalize the current shot and store it
    pub fn finalize_shot(&mut self) {
        self.apply_mappings();
        let shot = self.export_shot();
        self.last_shot = Some(shot);
    }

    /// Get the last shot result
    #[must_use]
    pub fn get_last_shot(&self) -> Option<&Shot> {
        self.last_shot.as_ref()
    }

    /// Get the bit width of a register
    #[must_use]
    pub fn get_register_bit_width(&self, register_name: &str) -> usize {
        self.register_bit_positions
            .get(register_name)
            .map_or(0, |&max_pos| max_pos + 1)
    }

    /// Set the interactive callback for immediate measurements
    pub fn set_interactive_callback(&mut self, callback: InteractiveCallback) {
        self.interactive_callback = Some(callback);
    }

    /// Get a reference to the interactive callback
    #[must_use]
    pub fn interactive_callback(&self) -> Option<&InteractiveCallback> {
        self.interactive_callback.as_ref()
    }

    /// Clear the interactive callback
    pub fn clear_interactive_callback(&mut self) {
        self.interactive_callback = None;
    }

    /// Set tuple return values from a function
    pub fn set_tuple_return(&mut self, values: &[i32]) {
        use log::debug;

        debug!("Setting tuple return values: {values:?}");
        debug!("Tuple accessed results: {:?}", self.tuple_accessed_results);
        debug!("Measurement result IDs: {:?}", self.measurement_result_ids);

        self.tuple_return = values.to_vec();
        self.tuple_placeholder_mapping.clear();

        // Map accessed results to tuple positions in order
        // The assumption is that result IDs are accessed in the same order
        // as they appear in the tuple
        if !self.tuple_accessed_results.is_empty() {
            debug!(
                "Mapping {} accessed results to tuple positions",
                self.tuple_accessed_results.len()
            );

            // Map each accessed result to its corresponding tuple position
            // We need to map ALL accessed results, not just placeholders
            // This is because some measurements might be executed before the tuple is constructed
            for (idx, &result_id) in self.tuple_accessed_results.iter().enumerate() {
                if idx < values.len() {
                    self.tuple_placeholder_mapping.insert(idx, result_id);
                    debug!(
                        "Mapped tuple index {} to result_id {} (current value={})",
                        idx, result_id, values[idx]
                    );
                }
            }
        }

        debug!(
            "Final tuple_placeholder_mapping: {:?}",
            self.tuple_placeholder_mapping
        );

        // Clear the accessed results for next time
        self.tuple_accessed_results.clear();
    }

    /// Get the measurement result IDs in order
    #[must_use]
    pub fn get_measurement_result_ids(&self) -> &[usize] {
        &self.measurement_result_ids
    }

    /// Get how many measurements have been executed
    #[must_use]
    pub fn get_measurements_executed(&self) -> usize {
        self.measurements_executed
    }

    /// Set how many measurements have been executed
    pub fn set_measurements_executed(&mut self, count: usize) {
        self.measurements_executed = count;
    }

    /// Increment measurements executed count
    pub fn increment_measurements_executed(&mut self) {
        self.measurements_executed += 1;
    }

    /// Find the index of a result ID in the measurement order
    /// Returns None if the result ID hasn't been queued for measurement yet
    #[must_use]
    pub fn find_result_id_index(&self, result_id: usize) -> Option<usize> {
        self.measurement_result_ids
            .iter()
            .position(|&id| id == result_id)
    }
}

impl Default for LlvmRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}
