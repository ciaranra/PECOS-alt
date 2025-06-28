//! Instance-based LLVM Runtime State
//!
//! This module provides an instance-based runtime state for LLVM IR execution,
//! eliminating the need for global state and enabling proper concurrent execution.

use pecos_engines::byte_message::{ByteMessage, ByteMessageBuilder};
use pecos_engines::shot_results::{Data, Shot};
use std::collections::HashMap;

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
    }

    /// Allocate a new qubit and return its ID
    pub fn allocate_qubit(&mut self) -> usize {
        let id = self.next_qubit_id;
        self.next_qubit_id += 1;
        id
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
        self.measurement_results.insert(result_id, value);
    }

    /// Get a measurement result
    #[must_use]
    pub fn get_measurement_result(&self, result_id: usize) -> Option<bool> {
        self.measurement_results.get(&result_id).copied()
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
        // Process pairs of (result_id, measurement_value)
        for i in (0..results.len()).step_by(2) {
            if i + 1 < results.len() {
                let result_id = results[i] as usize;
                let measurement_value = results[i + 1] != 0;
                self.measurement_results
                    .insert(result_id, measurement_value);
            }
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
        let mut shot = Shot::default();

        // Add all classical registers
        for (name, value) in &self.classical_registers {
            shot.data.insert(name.clone(), Data::I64(*value));
        }

        // If no named registers, export raw measurements
        if self.classical_registers.is_empty() && !self.measurement_results.is_empty() {
            for (&result_id, &value) in &self.measurement_results {
                let name = format!("result_{result_id}");
                shot.data.insert(name, Data::I64(i64::from(value)));
            }
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
}

impl Default for LlvmRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}
