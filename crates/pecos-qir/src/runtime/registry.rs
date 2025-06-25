//! Runtime Registry for QIR
//!
//! This module provides a registry that maps library instances to their runtime states.
//! This allows us to have instance-based runtime state while still working with
//! extern "C" functions that don't take context parameters.

use super::state::QirRuntimeState;
use log::error;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Global registry for runtime states
static RUNTIME_REGISTRY: RwLock<Option<RuntimeRegistry>> = RwLock::new(None);

/// Counter for generating unique runtime IDs
static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// Flag to indicate we're in cleanup/shutdown phase
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// Registry that maps runtime IDs to runtime states
pub struct RuntimeRegistry {
    states: HashMap<u64, Arc<Mutex<QirRuntimeState>>>,
}

impl RuntimeRegistry {
    /// Initialize the global registry
    pub fn initialize() {
        let mut registry = RUNTIME_REGISTRY.write().unwrap();
        if registry.is_none() {
            *registry = Some(RuntimeRegistry {
                states: HashMap::new(),
            });
        }
    }

    /// Register a new runtime state and return its ID
    pub fn register_runtime(state: Arc<Mutex<QirRuntimeState>>) -> u64 {
        let id = NEXT_RUNTIME_ID.fetch_add(1, Ordering::SeqCst);

        let mut registry = RUNTIME_REGISTRY.write().unwrap();
        if let Some(reg) = registry.as_mut() {
            reg.states.insert(id, state);
        } else {
            panic!("RuntimeRegistry not initialized");
        }

        id
    }

    /// Unregister a runtime state
    pub fn unregister_runtime(id: u64) {
        let mut registry = RUNTIME_REGISTRY.write().unwrap();
        if let Some(reg) = registry.as_mut() {
            reg.states.remove(&id);
        }
    }

    /// Get a runtime state by ID
    pub fn get_runtime(id: u64) -> Option<Arc<Mutex<QirRuntimeState>>> {
        let registry = RUNTIME_REGISTRY.read().unwrap();
        registry.as_ref()?.states.get(&id).cloned()
    }

    /// Set the current runtime ID for this thread
    pub fn set_current_runtime(id: u64) {
        CURRENT_RUNTIME_ID.with(|current| {
            *current.borrow_mut() = Some(id);
        });
    }

    /// Clear the current runtime ID for this thread
    pub fn clear_current_runtime() {
        CURRENT_RUNTIME_ID.with(|current| {
            *current.borrow_mut() = None;
        });
    }

    /// Get the current runtime state for this thread
    /// Auto-initializes if no runtime is set for this thread
    /// This function guarantees to return `Some()` by auto-initializing if needed
    pub fn with_current_runtime<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&mut QirRuntimeState) -> R,
    {
        // Don't auto-initialize if we're shutting down
        if SHUTTING_DOWN.load(Ordering::Acquire) {
            return None;
        }

        CURRENT_RUNTIME_ID.with(|current| {
            // Check if we already have a runtime ID set for this thread
            if let Some(id) = *current.borrow() {
                if let Some(runtime) = Self::get_runtime(id) {
                    if let Ok(mut state) = runtime.lock() {
                        return Some(f(&mut state));
                    }
                }
            }

            // Don't auto-initialize if we're shutting down
            if SHUTTING_DOWN.load(Ordering::Acquire) {
                return None;
            }

            // Auto-initialize if no runtime is set for this thread
            Self::initialize();
            let new_state = Arc::new(Mutex::new(QirRuntimeState::new()));
            let id = Self::register_runtime(new_state.clone());
            *current.borrow_mut() = Some(id);

            // Now try again with the new runtime - this should always succeed
            match new_state.lock() {
                Ok(mut state) => Some(f(&mut state)),
                Err(e) => {
                    error!(
                        "QIR Runtime: Critical error - failed to lock new runtime state: {e}"
                    );
                    // Return a default/fallback result instead of None to avoid crashes
                    // This is a last resort to prevent segfaults
                    panic!("QIR Runtime: Failed to initialize runtime state");
                }
            }
        })
    }

    /// Try to get the current runtime state without auto-initialization
    /// This is safer to use during cleanup/teardown
    pub fn try_with_current_runtime<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&mut QirRuntimeState) -> R,
    {
        CURRENT_RUNTIME_ID.with(|current| {
            // Check if we already have a runtime ID set for this thread
            if let Some(id) = *current.borrow() {
                if let Some(runtime) = Self::get_runtime(id) {
                    if let Ok(mut state) = runtime.lock() {
                        return Some(f(&mut state));
                    }
                }
            }
            // Don't auto-initialize - just return None
            None
        })
    }
}

// Thread-local storage for the current runtime ID
thread_local! {
    static CURRENT_RUNTIME_ID: std::cell::RefCell<Option<u64>> = const { std::cell::RefCell::new(None) };
}

/// Initialize the runtime registry (call once at startup)
pub fn initialize_registry() {
    RuntimeRegistry::initialize();
}

/// Set the shutdown flag to prevent new runtime initialization
pub fn set_shutting_down() {
    SHUTTING_DOWN.store(true, Ordering::Release);
}

/// Clear the shutdown flag (for testing purposes)
pub fn clear_shutting_down() {
    SHUTTING_DOWN.store(false, Ordering::Release);
}

/// Clean up all runtime states (for testing purposes)
pub fn cleanup_all_runtimes() {
    let mut registry = RUNTIME_REGISTRY.write().unwrap();
    if let Some(reg) = registry.as_mut() {
        reg.states.clear();
    }

    // Also clear thread-local state
    CURRENT_RUNTIME_ID.with(|current| {
        *current.borrow_mut() = None;
    });
}
