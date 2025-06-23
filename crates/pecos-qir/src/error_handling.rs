/*!
Enhanced Error Handling for QIR Execution

This module provides comprehensive error detection and reporting for QIR execution,
helping diagnose issues that might otherwise cause segfaults or aborts.
*/

use pecos_core::errors::PecosError;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Context information for QIR execution debugging
#[derive(Debug, Clone)]
pub struct QirExecutionContext {
    /// Currently executing QIR function name
    pub function_name: Option<String>,
    /// Number of qubits allocated
    pub qubit_count: usize,
    /// Number of results allocated  
    pub result_count: usize,
    /// Last executed operation
    pub last_operation: Option<String>,
    /// Qubit state tracking
    pub qubit_states: HashMap<usize, QubitState>,
    /// Error history
    pub errors: Vec<QirError>,
}

#[derive(Debug, Clone)]
pub struct QubitState {
    pub allocated: bool,
    pub measured: bool,
    pub last_gate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QirError {
    pub timestamp: std::time::SystemTime,
    pub error_type: QirErrorType,
    pub message: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone)]
pub enum QirErrorType {
    InvalidQubitIndex,
    InvalidResultIndex,
    QubitNotAllocated,
    ResultNotAllocated,
    MemoryCorruption,
    FormatIncompatibility,
    RuntimePanic,
}

/// Global execution context for QIR debugging
static QIR_CONTEXT: Mutex<Option<QirExecutionContext>> = Mutex::new(None);

impl QirExecutionContext {
    pub fn new() -> Self {
        Self {
            function_name: None,
            qubit_count: 0,
            result_count: 0,
            last_operation: None,
            qubit_states: HashMap::new(),
            errors: Vec::new(),
        }
    }

    pub fn set_function_name(&mut self, name: String) {
        self.function_name = Some(name);
    }

    pub fn allocate_qubit(&mut self, index: usize) -> Result<(), QirError> {
        if self.qubit_states.contains_key(&index) {
            let error = QirError {
                timestamp: std::time::SystemTime::now(),
                error_type: QirErrorType::MemoryCorruption,
                message: format!("Attempting to allocate already allocated qubit {}", index),
                context: self.function_name.clone(),
            };
            self.errors.push(error.clone());
            return Err(error);
        }

        self.qubit_states.insert(index, QubitState {
            allocated: true,
            measured: false,
            last_gate: None,
        });
        self.qubit_count = self.qubit_states.len();
        Ok(())
    }

    pub fn validate_qubit(&mut self, index: usize, operation: &str) -> Result<(), QirError> {
        if !self.qubit_states.contains_key(&index) {
            let error = QirError {
                timestamp: std::time::SystemTime::now(),
                error_type: QirErrorType::QubitNotAllocated,
                message: format!("Operation '{}' on unallocated qubit {}", operation, index),
                context: self.function_name.clone(),
            };
            self.errors.push(error.clone());
            return Err(error);
        }

        if index >= self.qubit_count {
            let error = QirError {
                timestamp: std::time::SystemTime::now(),
                error_type: QirErrorType::InvalidQubitIndex,
                message: format!(
                    "Qubit index {} out of bounds (max: {})", 
                    index, 
                    self.qubit_count.saturating_sub(1)
                ),
                context: self.function_name.clone(),
            };
            self.errors.push(error.clone());
            return Err(error);
        }

        // Update qubit state
        if let Some(state) = self.qubit_states.get_mut(&index) {
            state.last_gate = Some(operation.to_string());
        }

        self.last_operation = Some(format!("{}(qubit_{})", operation, index));
        Ok(())
    }

    pub fn validate_result(&mut self, index: usize) -> Result<(), QirError> {
        if index >= self.result_count {
            let error = QirError {
                timestamp: std::time::SystemTime::now(),
                error_type: QirErrorType::InvalidResultIndex,
                message: format!("Result index {} out of bounds (max: {})", index, self.result_count),
                context: self.function_name.clone(),
            };
            self.errors.push(error.clone());
            return Err(error);
        }
        Ok(())
    }

    pub fn record_measurement(&mut self, qubit_index: usize, result_index: usize) -> Result<(), QirError> {
        self.validate_qubit(qubit_index, "measure")?;
        
        if let Some(state) = self.qubit_states.get_mut(&qubit_index) {
            state.measured = true;
        }

        if result_index >= self.result_count {
            self.result_count = result_index + 1;
        }

        self.last_operation = Some(format!("measure(qubit_{}, result_{})", qubit_index, result_index));
        Ok(())
    }

    pub fn get_diagnostic_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== QIR Execution Diagnostic Report ===\n");
        
        if let Some(ref func_name) = self.function_name {
            report.push_str(&format!("Function: {}\n", func_name));
        }
        
        report.push_str(&format!("Qubits allocated: {}\n", self.qubit_count));
        report.push_str(&format!("Results allocated: {}\n", self.result_count));
        
        if let Some(ref last_op) = self.last_operation {
            report.push_str(&format!("Last operation: {}\n", last_op));
        }

        if !self.errors.is_empty() {
            report.push_str("\n=== Errors Detected ===\n");
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("{}. {:?}: {}\n", i + 1, error.error_type, error.message));
                if let Some(ref ctx) = error.context {
                    report.push_str(&format!("   Context: {}\n", ctx));
                }
            }
        }

        if !self.qubit_states.is_empty() {
            report.push_str("\n=== Qubit States ===\n");
            for (index, state) in &self.qubit_states {
                report.push_str(&format!("Qubit {}: allocated={}, measured={}", 
                    index, state.allocated, state.measured));
                if let Some(ref gate) = state.last_gate {
                    report.push_str(&format!(", last_gate={}", gate));
                }
                report.push('\n');
            }
        }

        report
    }
}

/// Initialize QIR execution context
pub fn init_qir_context(function_name: Option<String>) {
    let mut context = QirExecutionContext::new();
    if let Some(name) = function_name {
        context.set_function_name(name);
    }
    
    if let Ok(mut guard) = QIR_CONTEXT.lock() {
        *guard = Some(context);
    }
}

/// Get current QIR execution context
pub fn with_qir_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut QirExecutionContext) -> R,
{
    if let Ok(mut guard) = QIR_CONTEXT.lock() {
        if let Some(ref mut context) = *guard {
            return Some(f(context));
        }
    }
    None
}

/// Record a QIR operation for debugging
pub fn record_qir_operation(operation: &str, qubit_indices: &[usize]) {
    with_qir_context(|ctx| {
        for &index in qubit_indices {
            if let Err(error) = ctx.validate_qubit(index, operation) {
                eprintln!("QIR Error: {}", error.message);
                // In debug mode, we could panic here to catch issues early
                #[cfg(debug_assertions)]
                panic!("QIR validation failed: {}", error.message);
            }
        }
    });
}

/// Record a QIR measurement for debugging
pub fn record_qir_measurement(qubit_index: usize, result_index: usize) {
    with_qir_context(|ctx| {
        if let Err(error) = ctx.record_measurement(qubit_index, result_index) {
            eprintln!("QIR Error: {}", error.message);
            #[cfg(debug_assertions)]
            panic!("QIR measurement validation failed: {}", error.message);
        }
    });
}

/// Get diagnostic report for current QIR execution
pub fn get_qir_diagnostic_report() -> String {
    with_qir_context(|ctx| ctx.get_diagnostic_report())
        .unwrap_or_else(|| "No QIR context available".to_string())
}

/// Clear QIR execution context
pub fn clear_qir_context() {
    if let Ok(mut guard) = QIR_CONTEXT.lock() {
        *guard = None;
    }
}

/// Validate QIR format and detect potential runtime issues
pub fn validate_qir_for_runtime_issues(qir_content: &str) -> Result<Vec<String>, PecosError> {
    let mut warnings = Vec::new();
    
    // Check for common patterns that cause runtime issues
    
    // 1. Check for hardcoded large qubit indices
    for line in qir_content.lines() {
        if line.contains("inttoptr") && (line.contains("to %Qubit*") || line.contains("to i8*")) {
            // Simple string parsing instead of regex for better error handling
            if let Some(start) = line.find("inttoptr (i64 ") {
                let after_start = &line[start + 14..];
                if let Some(end) = after_start.find(' ') {
                    if let Ok(index) = after_start[..end].parse::<i64>() {
                        if index > 100 {
                            warnings.push(format!(
                                "Suspiciously large qubit index {} detected - may cause out-of-bounds errors", 
                                index
                            ));
                        }
                    }
                }
            }
        }
    }

    // 2. Check for inconsistent qubit/result allocation patterns
    let qubit_allocs = qir_content.matches("__quantum__rt__qubit_allocate").count();
    let qubit_uses = qir_content.matches("__quantum__qis__").count();
    
    if qubit_uses > 0 && qubit_allocs == 0 {
        warnings.push(
            "QIR uses quantum operations but has no explicit qubit allocations - may rely on static indices".to_string()
        );
    }

    // 3. Check for mixed calling conventions
    let has_i64_calls = qir_content.contains("__quantum__qis__h__body(i64");
    let has_ptr_calls = qir_content.contains("__quantum__qis__h__body(i8*") || 
                       qir_content.contains("__quantum__qis__h__body(%Qubit*");
    
    if has_i64_calls && has_ptr_calls {
        warnings.push(
            "QIR contains mixed calling conventions (both i64 and pointer) - may cause runtime confusion".to_string()
        );
    }

    // 4. Check for entry point issues
    let void_entry = qir_content.contains("define void @") && qir_content.contains("EntryPoint");
    let typed_entry = qir_content.contains("define i1 @") && qir_content.contains("EntryPoint");
    
    if typed_entry && void_entry {
        warnings.push(
            "QIR contains multiple entry point types - may cause execution ambiguity".to_string()
        );
    }

    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qubit_validation() {
        let mut ctx = QirExecutionContext::new();
        ctx.set_function_name("test_function".to_string());
        
        // Should fail - qubit not allocated
        assert!(ctx.validate_qubit(0, "H").is_err());
        
        // Allocate qubit
        assert!(ctx.allocate_qubit(0).is_ok());
        
        // Should succeed now
        assert!(ctx.validate_qubit(0, "H").is_ok());
        
        // Should fail - out of bounds
        assert!(ctx.validate_qubit(5, "H").is_err());
    }

    #[test]
    fn test_measurement_tracking() {
        let mut ctx = QirExecutionContext::new();
        ctx.allocate_qubit(0).unwrap();
        
        assert!(ctx.record_measurement(0, 0).is_ok());
        assert!(ctx.qubit_states[&0].measured);
    }
}