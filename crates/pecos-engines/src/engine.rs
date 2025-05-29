use dyn_clone::DynClone;
use pecos_core::errors::PecosError;

/// Core engine trait for processing inputs to outputs
pub trait Engine: DynClone + Send + Sync {
    type Input;
    type Output;

    /// Process a single input
    ///
    /// # Errors
    /// This function may return an error if:
    /// - There is an error during processing.
    /// - The input cannot be processed due to a serialization or execution issue.
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, PecosError>;

    /// Reset engine state for reuse
    ///
    /// This allows engines to be reused for multiple simulation runs
    /// by resetting any internal state to initial conditions.
    ///
    /// # Errors
    /// This function may return an error if:
    /// - There is an error during resetting the engine state.
    fn reset(&mut self) -> Result<(), PecosError>;
}
