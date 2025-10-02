//! JIT measurement management for handling quantum conditionals
//!
//! This module provides utilities for managing measurement futures during JIT execution.
//! It is NOT related to the QisRuntime trait - it's a JIT-specific helper for handling
//! quantum conditionals through a two-phase execution model.

use std::collections::HashMap;

/// Manager for quantum measurement futures during JIT execution
/// This allows handling of quantum conditionals by managing measurement results
/// across collection and simulation phases of JIT execution.
/// Note: This is NOT a QisRuntime trait implementation - it's a JIT-specific helper.
#[derive(Clone)]
pub struct JitMeasurementManager {
    /// Maps future IDs to their measurement results
    /// None means the measurement hasn't been performed yet
    measurements: HashMap<i64, Option<bool>>,

    /// Next available future ID
    next_future_id: i64,

    /// Whether we're in simulation mode (measurements available) or collection mode
    simulation_mode: bool,
}

impl JitMeasurementManager {
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            next_future_id: 0,
            simulation_mode: false,
        }
    }

    /// Allocate a new future ID for a lazy measurement
    pub fn allocate_future(&mut self) -> i64 {
        let id = self.next_future_id;
        self.next_future_id += 1;

        // Register the future as pending
        self.measurements.insert(id, None);

        id
    }

    /// Read a future's boolean value
    /// In collection mode: returns false (default path)
    /// In simulation mode: returns the actual measurement result
    pub fn read_future_bool(&self, future_id: i64) -> bool {
        if !self.simulation_mode {
            // Collection mode: return false to follow default path
            // This allows us to discover all quantum operations
            false
        } else {
            // Simulation mode: return actual measurement result
            self.measurements.get(&future_id)
                .and_then(|opt| *opt)
                .unwrap_or(false)
        }
    }

    /// Set a measurement result for a future
    pub fn set_measurement_result(&mut self, future_id: i64, result: bool) {
        self.measurements.insert(future_id, Some(result));
    }

    /// Enable simulation mode where measurements return actual results
    pub fn enable_simulation_mode(&mut self) {
        self.simulation_mode = true;
    }

    /// Reset for a new shot
    pub fn reset(&mut self) {
        self.measurements.clear();
        self.next_future_id = 0;
        self.simulation_mode = false;
    }
}

use std::cell::RefCell;

// Thread-local measurement manager for the current JIT execution
// Each thread gets its own instance, avoiding synchronization overhead
thread_local! {
    static MEASUREMENT_MANAGER: RefCell<JitMeasurementManager> = RefCell::new(JitMeasurementManager::new());
}

/// Execute a function with the thread-local measurement manager
pub fn with_measurement_manager<F, R>(f: F) -> R
where
    F: FnOnce(&JitMeasurementManager) -> R,
{
    MEASUREMENT_MANAGER.with(|manager| f(&manager.borrow()))
}

/// Execute a mutable function with the thread-local measurement manager
pub fn with_measurement_manager_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut JitMeasurementManager) -> R,
{
    MEASUREMENT_MANAGER.with(|manager| f(&mut manager.borrow_mut()))
}

/// Reset the thread-local measurement manager
pub fn reset_measurement_manager() {
    with_measurement_manager_mut(|manager| manager.reset());
}

// Compatibility functions that work with the thread-local manager
pub fn get_measurement_manager() -> JitMeasurementManager {
    MEASUREMENT_MANAGER.with(|manager| manager.borrow().clone())
}

pub fn clear_measurement_manager() {
    reset_measurement_manager();
}

// Deprecated aliases for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use `get_measurement_manager` instead")]
pub fn get_runtime() -> JitMeasurementManager {
    get_measurement_manager()
}

#[deprecated(since = "0.2.0", note = "Use `clear_measurement_manager` instead")]
pub fn clear_runtime() {
    clear_measurement_manager()
}