pub mod classical;
pub mod hybrid;
pub mod monte_carlo;
pub mod noise;
pub mod phir;
pub mod qir;
pub mod quantum;

use crate::errors::QueueError;
pub use classical::ClassicalEngine;
pub use hybrid::HybridEngine;
pub use monte_carlo::MonteCarloEngine;
pub use quantum::QuantumEngine;

/// Core engine trait for processing inputs to outputs
pub trait Engine: Send + Sync {
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

pub struct EngineSystem<C, E, Input, Output, EngineInput, EngineOutput>
where
    C: ControlEngine<Input = Input, Output = Output>,
    E: Engine<Input = EngineInput, Output = EngineOutput>,
{
    controller: C,
    engine: E,
}

impl<C, E, Input, Output, EngineInput, EngineOutput>
    EngineSystem<C, E, Input, Output, EngineInput, EngineOutput>
where
    C: ControlEngine<
            Input = Input,
            Output = Output,
            EngineInput = EngineInput,
            EngineOutput = EngineOutput,
        >,
    E: Engine<Input = EngineInput, Output = EngineOutput>,
{
    pub fn new(controller: C, engine: E) -> Self {
        Self { controller, engine }
    }

    /// Process an input through the engine system.
    ///
    /// This method orchestrates processing by initiating the control engine,
    /// performing processing work through the controlled engine, and
    /// iteratively handling the results until the computation is complete.
    ///
    /// # Parameters
    /// - `input`: The initial input to process.
    ///
    /// # Returns
    /// - `Ok(output)`: The final output after successful processing.
    ///
    /// # Errors
    /// This method returns a `QueueError` if:
    /// - An error occurs during the start phase in the `controller`.
    /// - An error occurs during processing in the controlled `engine`.
    /// - An error occurs during the continuation phase in the `controller`.
    pub fn process(&mut self, input: Input) -> Result<Output, QueueError> {
        let mut stage = self.controller.start(input)?;

        while let EngineStage::NeedsProcessing(batch) = stage {
            let processed = self.engine.process(batch)?;
            stage = self.controller.continue_processing(processed)?;
        }

        match stage {
            EngineStage::Complete(output) => Ok(output),
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }

    /// Reset the state of both the controller and the engine for reuse.
    ///
    /// This method resets the `controller` and `engine` to their initial states,
    /// allowing the system to be reused for new processing tasks or simulations.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error during the reset of the controller.
    /// - There is an error during the reset of the engine.
    pub fn reset(&mut self) -> Result<(), QueueError> {
        println!("DDDDD");
        self.controller.reset()?;
        self.engine.reset()
    }
}

/// An `EngineSystem` itself is an `Engine`.
impl<C, E, Input, Output, EngineInput, EngineOutput> Engine
    for EngineSystem<C, E, Input, Output, EngineInput, EngineOutput>
where
    C: ControlEngine<
            Input = Input,
            Output = Output,
            EngineInput = EngineInput,
            EngineOutput = EngineOutput,
        >,
    E: Engine<Input = EngineInput, Output = EngineOutput>,
{
    type Input = Input;
    type Output = Output;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        EngineSystem::process(self, input)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        EngineSystem::reset(self)
    }
}
