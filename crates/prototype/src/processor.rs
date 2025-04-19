mod composite;
mod program;
mod simulator;

pub use composite::CompositeProcessor;
pub use program::ProgramProcessor;
pub use simulator::SimulatorProcessor;

use crate::message::MessageBatch;

/// Core processor trait for handling quantum operations
pub trait Processor {
    /// Process a message batch and return a new batch
    fn process(&mut self, batch: MessageBatch) -> MessageBatch;
}
