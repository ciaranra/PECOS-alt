use crate::runtime::core_runtime;
use pecos_engines::byte_message::ByteMessage;
use pecos_core::errors::PecosError;
use std::sync::{Arc, Mutex};

/// Sets up an interactive execution callback for the LLVM runtime
///
/// This enables on-demand quantum execution when measurement results are needed.
/// The callback will be invoked by __`quantum__rt__result_get_one` when it needs
/// to execute quantum operations to get measurement results.
pub fn setup_interactive_callback<F>(callback: F) 
where
    F: Fn(ByteMessage) -> Result<Vec<u32>, PecosError> + Send + Sync + 'static,
{
    core_runtime::set_interactive_callback(Box::new(callback));
}

/// Creates an interactive callback that executes quantum operations through a quantum engine
///
/// This is designed to work with the `EngineSystem` architecture where:
/// 1. Classical engine generates quantum commands
/// 2. Quantum engine executes them and returns measurements
/// 3. Classical engine processes the measurements
pub fn create_quantum_callback<QE>(quantum_engine: Arc<Mutex<QE>>) -> impl Fn(ByteMessage) -> Result<Vec<u32>, PecosError> + Send + Sync + 'static
where
    QE: pecos_engines::Engine<Input = ByteMessage, Output = ByteMessage> + Send + 'static,
{
    move |commands: ByteMessage| {
        // Execute quantum operations and get measurements
        let mut engine = quantum_engine.lock().map_err(|e| {
            PecosError::Processing(format!("Failed to lock quantum engine: {}", e))
        })?;
        
        let result = engine.process(commands)?;
        
        // Extract measurement outcomes from the result ByteMessage
        let measurements = result.outcomes()?;
        
        Ok(measurements)
    }
}