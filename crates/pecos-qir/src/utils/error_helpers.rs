/// Error handling utilities for LLVM operations
use log::warn;
use pecos_core::errors::PecosError;
use std::fmt::Display;

/// Log an error with context and return a `PecosError`
///
/// # Errors
///
/// This function always returns an error of type `PecosError::Processing` with the provided context
pub fn log_error<E: Display>(component: &str, context: &str, error: E) -> PecosError {
    let message = format!("{component}: {context}: {error}");
    warn!("{}", message);
    PecosError::Processing(format!("{component} operation failed - {context}: {error}"))
}

/// Convert a Result with a custom error message
pub trait ProcessingError<T> {
    /// Map an error to a `PecosError` with additional context
    ///
    /// # Errors
    ///
    /// Returns `PecosError::Processing` with the provided context and original error message
    fn map_processing_err(self, context: &str) -> Result<T, PecosError>;
}

impl<T, E: Display> ProcessingError<T> for Result<T, E> {
    fn map_processing_err(self, context: &str) -> Result<T, PecosError> {
        self.map_err(|e| PecosError::Processing(format!("{context}: {e}")))
    }
}

/// Retry an operation with exponential backoff
///
/// # Errors
///
/// Returns the last error encountered if all retry attempts fail
pub fn retry_with_backoff<T, F>(
    mut operation: F,
    max_attempts: usize,
    initial_delay_ms: u64,
) -> Result<T, PecosError>
where
    F: FnMut() -> Result<T, PecosError>,
{
    use std::thread;
    use std::time::Duration;

    let mut delay = initial_delay_ms;

    for attempt in 1..=max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_attempts => {
                warn!(
                    "Attempt {} failed: {}. Retrying in {}ms...",
                    attempt, e, delay
                );
                thread::sleep(Duration::from_millis(delay));
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}
