mod classical;
pub mod hybrid;
pub mod phir_engine;
pub mod quantum;

use crate::errors::QueueError;
pub use classical::ClassicalEngine;
pub use hybrid::HybridEngine;
pub use quantum::QuantumEngine;

/// Core engine trait for processing inputs to outputs
pub trait Engine: Send + Sync {
    type Input;
    type Output;

    /// Process a single input
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError>;

    /// Reset engine state for reuse
    ///
    /// This allows engines to be reused for multiple simulation runs
    /// by resetting any internal state to initial conditions.
    fn reset(&mut self) -> Result<(), QueueError>;
}

/// Represents the stage of processing in a control engine
///
/// Control engines orchestrate the execution flow by:
/// 1. Taking input and determining what needs to be sent to a controlled engine
/// 2. Processing results from the controlled engine
/// 3. Deciding whether to continue processing or complete
#[derive(Debug)]
pub enum EngineStage<I, O> {
    /// Indicates more processing is needed by sending input to controlled engine
    NeedsProcessing(I),
    /// Processing is complete with final output
    Complete(O),
}

/// A control engine that orchestrates execution flow with another engine
///
/// Control engines manage complex workflows by:
/// - Breaking down input into smaller pieces for processing
/// - Sending those pieces to another engine
/// - Handling results from that engine
/// - Determining when processing is complete
///
/// # Type Parameters
/// * `Input` - Type received as input
/// * `Output` - Type returned as final output
/// * `EngineInput` - Type sent to the controlled engine
/// * `EngineOutput` - Type received from the controlled engine
pub trait ControlEngine {
    type Input;
    type Output;
    type EngineInput;
    type EngineOutput;

    /// Start processing new input
    ///
    /// # Parameters
    /// * `input` - Initial input to process
    ///
    /// # Returns
    /// * `NeedsProcessing(input)` if more processing needed
    /// * `Complete(output)` if processing finished
    fn start(
        &mut self,
        input: Self::Input,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError>;

    /// Continue processing with result from controlled engine
    ///
    /// # Parameters
    /// * `result` - Result from previous engine processing
    ///
    /// # Returns
    /// * `NeedsProcessing(input)` if more processing needed
    /// * `Complete(output)` if processing finished
    fn continue_processing(
        &mut self,
        result: Self::EngineOutput,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError>;
}
