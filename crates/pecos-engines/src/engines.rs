pub mod classical;
pub mod hybrid;
pub mod monte_carlo;
pub mod noise;
pub mod phir;
pub mod qir;
pub mod quantum;
pub mod quantum_system;

use crate::errors::QueueError;
pub use classical::ClassicalEngine;
use dyn_clone::DynClone;
pub use hybrid::HybridEngine;
pub use monte_carlo::MonteCarloEngine;
pub use quantum::QuantumEngine;

/// Core engine trait for processing inputs to outputs
pub trait Engine: DynClone + Send + Sync {
    type Input;
    type Output;

    /// Process a single input
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error during processing.
    /// - The input cannot be processed due to a serialization or execution issue.
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError>;

    /// Reset engine state for reuse
    ///
    /// This allows engines to be reused for multiple simulation runs
    /// by resetting any internal state to initial conditions.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error during resetting the engine state.
    fn reset(&mut self) -> Result<(), QueueError>;
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
pub trait ControlEngine: Send + Sync {
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
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error during the start of processing.
    /// - The input cannot be serialized or deserialized.
    /// - An operation fails during initialization.
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
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - The result cannot be deserialized or processed.
    /// - There is an error during the continuation of processing.
    /// - Any operation fails while handling the result.
    fn continue_processing(
        &mut self,
        result: Self::EngineOutput,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError>;

    /// Reset engine state for reuse
    ///
    /// This allows engines to be reused for multiple simulation runs
    /// by resetting any internal state to initial conditions.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error during resetting the engine state.
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

/// A system that combines a controller and a controlled engine
///
/// This trait represents a complete engine system that consists of:
/// 1. A controller component that manages the execution flow
/// 2. A controlled engine component that performs the actual processing
pub trait EngineSystem: Send + Sync + Clone {
    /// The type of the controller component
    type Controller: ControlEngine<
            Input = Self::Input,
            Output = Self::Output,
            EngineInput = Self::EngineInput,
            EngineOutput = Self::EngineOutput,
        >;

    /// The type of the controlled engine component
    type ControlledEngine: Engine<Input = Self::EngineInput, Output = Self::EngineOutput>;

    /// The input type for the system
    type Input;

    /// The output type from the system
    type Output;

    /// The input type for the controlled engine
    type EngineInput;

    /// The output type from the controlled engine
    type EngineOutput;

    /// Get a reference to the controller component
    fn controller(&self) -> &Self::Controller;

    /// Get a mutable reference to the controller component
    fn controller_mut(&mut self) -> &mut Self::Controller;

    /// Get a reference to the controlled engine component
    fn engine(&self) -> &Self::ControlledEngine;

    /// Get a mutable reference to the controlled engine component
    fn engine_mut(&mut self) -> &mut Self::ControlledEngine;
}

/// Default implementation of Engine for any `EngineSystem`
impl<T> Engine for T
where
    T: EngineSystem + 'static,
{
    type Input = T::Input;
    type Output = T::Output;

    /// Process the input through the engine system.
    ///
    /// This method orchestrates the processing of input through the controller and engine components.
    /// It takes an input, starts the processing, and continues processing until completion.
    ///
    /// # Parameters
    /// * `input` - The input to process
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        let mut stage = self.controller_mut().start(input)?;

        loop {
            match stage {
                EngineStage::NeedsProcessing(engine_input) => {
                    let engine_output = self.engine_mut().process(engine_input)?;
                    stage = self.controller_mut().continue_processing(engine_output)?;
                }
                EngineStage::Complete(output) => return Ok(output),
            }
        }
    }

    /// Reset the state of both the controller and the engine for reuse.
    ///
    /// This method resets the controller and engine to their initial states,
    /// allowing the system to be reused for new processing tasks or simulations.
    fn reset(&mut self) -> Result<(), QueueError> {
        self.controller_mut().reset()?;
        self.engine_mut().reset()
    }
}
