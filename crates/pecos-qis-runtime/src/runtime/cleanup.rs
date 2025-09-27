//! Runtime cleanup utilities for preventing state contamination between tests

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to track if we're in the middle of cleanup
static CLEANUP_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Ensure runtime is fully cleaned up
pub fn force_runtime_cleanup() {
    // Prevent recursive cleanup
    if CLEANUP_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return;
    }

    // Clear all callbacks
    crate::runtime::core_runtime::clear_interactive_callback();

    // Clean up all runtime states
    super::registry::cleanup_all_runtimes();

    // Mark cleanup as complete
    CLEANUP_IN_PROGRESS.store(false, Ordering::SeqCst);
}

/// Thread-local cleanup for test isolation
pub fn cleanup_thread_local_state() {
    // Clear thread-local runtime ID
    super::registry::RuntimeRegistry::clear_current_runtime();
}
