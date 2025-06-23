/*!
QIR-aware panic handler that provides better error diagnostics

This module implements a custom panic handler that can provide QIR execution
context when panics occur, making it easier to debug QIR-related issues.
*/

use crate::error_handling::{get_qir_diagnostic_report, with_qir_context};
use std::panic::{self, PanicInfo};
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize QIR-aware panic handler
pub fn init_qir_panic_handler() {
    INIT.call_once(|| {
        // Set our custom panic hook
        panic::set_hook(Box::new(qir_panic_hook));
    });
}

/// Custom panic hook that includes QIR execution context
fn qir_panic_hook(info: &PanicInfo) {
    // Get the default panic message
    let location = if let Some(location) = info.location() {
        format!(" at {}:{}:{}", location.file(), location.line(), location.column())
    } else {
        " at unknown location".to_string()
    };

    let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Box<dyn Any>".to_string()
    };

    // Check if this might be a QIR-related panic
    let is_qir_related = message.contains("index out of bounds") ||
                        message.contains("quantum") ||
                        message.contains("qubit") ||
                        location.contains("pecos") ||
                        location.contains("qir");

    eprintln!("thread '{}' panicked at {}: {}", 
              std::thread::current().name().unwrap_or("<unnamed>"),
              location,
              message);

    if is_qir_related {
        eprintln!("\n🔍 QIR-Related Panic Detected!");
        eprintln!("This panic may be related to QIR execution. Here's the diagnostic information:\n");
        
        // Get QIR diagnostic report
        let diagnostic = get_qir_diagnostic_report();
        eprintln!("{}", diagnostic);

        // Provide helpful suggestions
        eprintln!("\n💡 Debugging Suggestions:");
        
        if message.contains("index out of bounds") {
            eprintln!("   - Check qubit/result indices in your QIR code");
            eprintln!("   - Verify that all qubits are properly allocated before use");
            eprintln!("   - Look for hardcoded large indices that exceed allocated resources");
        }

        if message.contains("quantum") || message.contains("qubit") {
            eprintln!("   - Verify QIR format compatibility (HUGR vs Standard QIR)");
            eprintln!("   - Check for mixed calling conventions (i64 vs pointer types)");
            eprintln!("   - Ensure proper qubit allocation/deallocation");
        }

        eprintln!("   - Run with RUST_BACKTRACE=1 for detailed stack trace");
        eprintln!("   - Enable debug logging for more context");
        eprintln!("   - Use QIR validation tools to check format compatibility");
        
        // Get additional context from current QIR execution
        with_qir_context(|ctx| {
            if !ctx.errors.is_empty() {
                eprintln!("\n⚠️  Previous QIR Errors Detected:");
                for (i, error) in ctx.errors.iter().take(3).enumerate() {
                    eprintln!("   {}. {:?}: {}", i + 1, error.error_type, error.message);
                }
                if ctx.errors.len() > 3 {
                    eprintln!("   ... and {} more errors", ctx.errors.len() - 3);
                }
            }
        });
    }

    // Print backtrace if available
    if std::env::var("RUST_BACKTRACE").is_ok() {
        let backtrace = std::backtrace::Backtrace::capture();
        eprintln!("\nstack backtrace:\n{}", backtrace);
    } else {
        eprintln!("\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace");
    }
}

/// Wrapper function to execute QIR operations with enhanced error reporting
pub fn with_qir_error_context<F, R>(operation_name: &str, f: F) -> Result<R, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<R, Box<dyn std::error::Error>>,
{
    // Initialize panic handler if not already done
    init_qir_panic_handler();

    // Record the operation we're about to perform
    with_qir_context(|ctx| {
        ctx.last_operation = Some(operation_name.to_string());
    });

    // Execute the operation with panic catching
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => {
            eprintln!("🚨 QIR Operation '{}' failed with error: {}", operation_name, e);
            
            // Add error to QIR context
            with_qir_context(|ctx| {
                ctx.errors.push(crate::error_handling::QirError {
                    timestamp: std::time::SystemTime::now(),
                    error_type: crate::error_handling::QirErrorType::RuntimePanic,
                    message: format!("Operation '{}' failed: {}", operation_name, e),
                    context: ctx.function_name.clone(),
                });
            });
            
            Err(e)
        }
        Err(panic_payload) => {
            eprintln!("🚨 QIR Operation '{}' panicked!", operation_name);
            
            // Try to extract panic message
            let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let error_msg = format!("QIR operation '{}' panicked: {}", operation_name, panic_msg);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_msg)))
        }
    }
}

/// Macro to wrap QIR operations with error context
#[macro_export]
macro_rules! qir_operation {
    ($name:expr, $body:expr) => {
        $crate::panic_handler::with_qir_error_context($name, || $body)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panic_handler_initialization() {
        // This should not panic
        init_qir_panic_handler();
        
        // Call again to test it's idempotent
        init_qir_panic_handler();
    }

    #[test]
    fn test_error_context_success() {
        let result = with_qir_error_context("test_operation", || {
            Ok::<i32, Box<dyn std::error::Error>>(42)
        });
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_error_context_failure() {
        let result = with_qir_error_context("test_operation", || {
            Err::<i32, Box<dyn std::error::Error>>(
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"))
            )
        });
        
        assert!(result.is_err());
    }
}