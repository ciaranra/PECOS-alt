//! Thread-local runtime context for QIR execution
//! 
//! This module provides isolated runtime contexts for QIR executions,
//! replacing global state with thread-local storage to enable parallel execution.

use parking_lot::Mutex;
use pecos_engines::ByteMessageBuilder;
use std::collections::HashMap;
use std::sync::Arc;

/// Runtime state for a single QIR execution context
#[derive(Debug)]
pub struct RuntimeContext {
    /// Message builder for recording quantum operations
    pub message_builder: ByteMessageBuilder,
    /// Measurement results by result ID
    pub measurement_results: HashMap<usize, bool>,
    /// Classical register values
    pub classical_registers: HashMap<String, i64>,
    /// Next available qubit ID
    pub next_qubit_id: usize,
    /// Next available result ID  
    pub next_result_id: usize,
    /// Unique context identifier
    pub context_id: usize,
}

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new(context_id: usize) -> Self {
        let mut message_builder = ByteMessageBuilder::new();
        let _ = message_builder.for_quantum_operations();
        
        Self {
            message_builder,
            measurement_results: HashMap::new(),
            classical_registers: HashMap::new(),
            next_qubit_id: 0,
            next_result_id: 0,
            context_id,
        }
    }

    /// Allocate a new qubit ID
    pub fn allocate_qubit(&mut self) -> usize {
        let id = self.next_qubit_id;
        self.next_qubit_id += 1;
        id
    }

    /// Allocate a new result ID
    pub fn allocate_result(&mut self) -> usize {
        let id = self.next_result_id;
        self.next_result_id += 1;
        id
    }

    /// Record a measurement result
    pub fn record_measurement(&mut self, result_id: usize, value: bool) {
        self.measurement_results.insert(result_id, value);
    }

    /// Get a measurement result
    pub fn get_measurement(&self, result_id: usize) -> Option<bool> {
        self.measurement_results.get(&result_id).copied()
    }

    /// Set a classical register value
    pub fn set_register(&mut self, name: String, value: i64) {
        self.classical_registers.insert(name, value);
    }

    /// Get a classical register value
    pub fn get_register(&self, name: &str) -> Option<i64> {
        self.classical_registers.get(name).copied()
    }

    /// Reset the context for reuse
    pub fn reset(&mut self) {
        self.message_builder = ByteMessageBuilder::new();
        let _ = self.message_builder.for_quantum_operations();
        self.measurement_results.clear();
        self.classical_registers.clear();
        self.next_qubit_id = 0;
        self.next_result_id = 0;
    }
}

// Thread-local storage for the current runtime context
thread_local! {
    static CURRENT_CONTEXT: std::cell::RefCell<Option<Arc<Mutex<RuntimeContext>>>> = 
        std::cell::RefCell::new(None);
}

/// Global context counter for unique IDs
static CONTEXT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

/// RAII guard for managing runtime context lifecycle
pub struct ContextGuard {
    context: Arc<Mutex<RuntimeContext>>,
}

impl ContextGuard {
    /// Create a new context guard with an isolated runtime context
    pub fn new() -> Self {
        let context_id = CONTEXT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let context = Arc::new(Mutex::new(RuntimeContext::new(context_id)));
        
        // Set as current context for this thread
        CURRENT_CONTEXT.with(|c| {
            *c.borrow_mut() = Some(context.clone());
        });

        Self { context }
    }

    /// Get access to the context
    pub fn context(&self) -> &Arc<Mutex<RuntimeContext>> {
        &self.context
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        // Clear the thread-local context when guard is dropped
        CURRENT_CONTEXT.with(|c| {
            *c.borrow_mut() = None;
        });
    }
}

/// Execute a closure with the current runtime context
pub fn with_current_context<F, R>(f: F) -> Result<R, &'static str>
where
    F: FnOnce(&mut RuntimeContext) -> R,
{
    CURRENT_CONTEXT.with(|c| {
        let context_opt = c.borrow().clone();
        match context_opt {
            Some(context) => {
                let mut ctx = context.lock();
                Ok(f(&mut ctx))
            }
            None => Err("No active runtime context"),
        }
    })
}

/// Get the current context ID for debugging
pub fn current_context_id() -> Option<usize> {
    CURRENT_CONTEXT.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.lock().context_id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_context_isolation() {
        let handle1 = thread::spawn(|| {
            let _guard = ContextGuard::new();
            let id1 = current_context_id().unwrap();
            
            with_current_context(|ctx| {
                ctx.allocate_qubit();
                ctx.allocate_result();
            }).unwrap();
            
            id1
        });

        let handle2 = thread::spawn(|| {
            let _guard = ContextGuard::new();
            let id2 = current_context_id().unwrap();
            
            with_current_context(|ctx| {
                ctx.allocate_qubit();
                ctx.allocate_qubit();
            }).unwrap();
            
            id2
        });

        let id1 = handle1.join().unwrap();
        let id2 = handle2.join().unwrap();
        
        // Each thread should have a different context ID
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_context_operations() {
        let _guard = ContextGuard::new();
        
        let qubit_id = with_current_context(|ctx| ctx.allocate_qubit()).unwrap();
        assert_eq!(qubit_id, 0);
        
        let result_id = with_current_context(|ctx| ctx.allocate_result()).unwrap();
        assert_eq!(result_id, 0);
        
        with_current_context(|ctx| {
            ctx.record_measurement(result_id, true);
            ctx.set_register("test".to_string(), 42);
        }).unwrap();
        
        let measurement = with_current_context(|ctx| ctx.get_measurement(result_id)).unwrap();
        assert_eq!(measurement, Some(true));
        
        let register_val = with_current_context(|ctx| ctx.get_register("test")).unwrap();
        assert_eq!(register_val, Some(42));
    }
}