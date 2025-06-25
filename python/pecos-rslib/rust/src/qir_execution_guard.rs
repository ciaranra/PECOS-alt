// Copyright 2025 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! Execution guard for QIR to prevent cleanup issues and enable future context isolation

use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use pyo3::prelude::*;

/// Global state for managing QIR execution lifecycle
static EXECUTION_STATE: OnceLock<Arc<ExecutionState>> = OnceLock::new();

/// State tracking for QIR executions
struct ExecutionState {
    /// Number of active executions
    active_executions: AtomicUsize,
    
    /// Flag indicating if Python is shutting down
    shutting_down: AtomicBool,
    
    /// Mutex for coordinating cleanup (used sparingly)
    cleanup_lock: Mutex<()>,
}

impl ExecutionState {
    fn new() -> Self {
        Self {
            active_executions: AtomicUsize::new(0),
            shutting_down: AtomicBool::new(false),
            cleanup_lock: Mutex::new(()),
        }
    }
    
    fn get() -> &'static Arc<ExecutionState> {
        EXECUTION_STATE.get_or_init(|| Arc::new(Self::new()))
    }
}

/// RAII guard for QIR execution that prevents cleanup race conditions
pub struct QirExecutionGuard {
    /// Whether this guard is active
    active: bool,
}

impl QirExecutionGuard {
    /// Create a new execution guard
    pub fn new() -> Result<Self, &'static str> {
        let state = ExecutionState::get();
        
        // Check if we're shutting down
        if state.shutting_down.load(Ordering::Acquire) {
            return Err("Cannot start QIR execution during shutdown");
        }
        
        // Increment active execution count
        state.active_executions.fetch_add(1, Ordering::AcqRel);
        
        Ok(Self { active: true })
    }
    
    /// Check if any executions are active
    pub fn has_active_executions() -> bool {
        let state = ExecutionState::get();
        state.active_executions.load(Ordering::Acquire) > 0
    }
    
    /// Mark that Python is shutting down
    pub fn mark_shutting_down() {
        let state = ExecutionState::get();
        state.shutting_down.store(true, Ordering::Release);
    }
    
    /// Wait for all executions to complete with timeout
    pub fn wait_for_completion() {
        let state = ExecutionState::get();
        
        // Add timeout to prevent infinite hanging during pytest cleanup
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(10); // 10 second timeout
        
        // Busy wait with exponential backoff
        let mut sleep_ms = 1;
        while state.active_executions.load(Ordering::Acquire) > 0 {
            // Check for timeout
            if start_time.elapsed() > timeout {
                eprintln!("Warning: QirExecutionGuard timeout waiting for {} active executions to complete", 
                         state.active_executions.load(Ordering::Acquire));
                // Force reset the counter to prevent infinite hanging
                state.active_executions.store(0, Ordering::Release);
                break;
            }
            
            std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
            sleep_ms = (sleep_ms * 2).min(100);
        }
    }
}

impl Drop for QirExecutionGuard {
    fn drop(&mut self) {
        if self.active {
            let state = ExecutionState::get();
            let prev_count = state.active_executions.fetch_sub(1, Ordering::AcqRel);
            
            // Defensive check - prevent underflow
            if prev_count == 0 {
                eprintln!("Warning: QirExecutionGuard underflow detected - execution counter was already 0");
                // Reset to 0 to be safe
                state.active_executions.store(0, Ordering::Release);
            }
            
            self.active = false;
        }
    }
}

/// Python module cleanup handler to prevent abort during shutdown
pub fn register_cleanup_handler() {
    // Disabled: atexit handlers can cause hangs during pytest
    // The timeout in wait_for_completion should prevent infinite hangs
    // and QIR execution guard will clean up on drop
}

/// Mark QIR as shutting down (called from Python atexit)
#[pyo3::pyfunction]
pub fn _mark_qir_shutting_down() {
    QirExecutionGuard::mark_shutting_down();
}

/// Wait for QIR executions to complete (called from Python atexit)  
#[pyo3::pyfunction]
pub fn _wait_for_qir_completion() {
    QirExecutionGuard::wait_for_completion();
}

/// Future: Context handle for isolated QIR execution
/// This will replace global state in the runtime
#[derive(Clone)]
pub struct QirContext {
    /// Unique context ID
    id: usize,
    
    /// Context-specific state will go here
    _phantom: std::marker::PhantomData<()>,
}

impl QirContext {
    /// Create a new isolated context
    pub fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Get the context ID
    pub fn id(&self) -> usize {
        self.id
    }
}

/// Future: Execute with an isolated context
pub fn with_qir_context<F, R>(f: F) -> R
where
    F: FnOnce(&QirContext) -> R,
{
    let context = QirContext::new();
    f(&context)
}