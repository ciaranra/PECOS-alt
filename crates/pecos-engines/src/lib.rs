pub mod byte_message;
pub mod classical;
pub mod engine;
pub mod engine_system;
pub mod hybrid;
pub mod monte_carlo;
pub mod noise;
pub mod prelude;
pub mod quantum;
pub mod quantum_system;
pub mod shot_results;
pub mod simulation_builder;

#[cfg(test)]
mod tests;

pub use byte_message::{ByteMessage, ByteMessageBuilder, Gate, GateType};
pub use engine::Engine;
pub use engine_system::{ClassicalEngine, ControlEngine, EngineStage, EngineSystem};
pub use hybrid::HybridEngine;
pub use monte_carlo::MonteCarloEngine;
pub use noise::{DepolarizingNoiseModel, NoiseModel, PassThroughNoiseModel};
pub use pecos_core::errors::PecosError;
pub use quantum::QuantumEngine;
pub use quantum_system::QuantumSystem;
pub use shot_results::data_vec::DataVecType;
pub use shot_results::{
    BitVecDisplayFormat, Data, DataVec, Shot, ShotMap, ShotMapDisplay, ShotMapDisplayExt,
    ShotMapDisplayOptions, ShotVec,
};
pub use simulation_builder::{SimulationBuilder, run_sim_safe};

/// Run a quantum simulation.
///
/// This function provides a flexible interface for running quantum simulations.
/// It takes a classical engine along with optional components for noise modeling
/// and quantum simulation.
///
/// # Parameters
/// * `classical_engine` - The classical engine that defines the program to run
/// * `shots` - Number of shots to run the simulation
/// * `seed` - Optional seed for reproducibility
/// * `workers` - Optional number of workers for parallelization (default: 1)
/// * `noise_model` - Optional noise model (default: `PassThroughNoiseModel` - no noise)
/// * `quantum_engine` - Optional quantum engine (default: `StateVecEngine`)
///
/// # Returns
/// The `ShotVec` structure containing measurement results for each shot
///
/// # Examples
///
/// ```
/// use pecos_engines::{run_sim, ClassicalEngine, ByteMessage, Engine};
/// use pecos_engines::shot_results::{Shot, ShotVec};
/// use pecos_core::errors::PecosError;
/// use std::any::Any;
///
/// // A minimal classical engine implementation for the example
/// #[derive(Clone)]
/// struct DummyEngine;
///
/// impl Engine for DummyEngine {
///     type Input = ();
///     type Output = Shot;
///
///     fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
///         Ok(Shot::default())
///     }
///
///     fn reset(&mut self) -> Result<(), PecosError> {
///         Ok(())
///     }
/// }
///
/// impl ClassicalEngine for DummyEngine {
///     fn num_qubits(&self) -> usize { 2 }
///
///     fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
///         // Return empty message to indicate no commands
///         Ok(ByteMessage::builder().build())
///     }
///
///     fn handle_measurements(&mut self, _message: ByteMessage) -> Result<(), PecosError> {
///         Ok(())
///     }
///
///     fn get_results(&self) -> Result<Shot, PecosError> {
///         Ok(Shot::default())
///     }
///
///     fn compile(&self) -> Result<(), PecosError> {
///         Ok(())
///     }
///
///     fn as_any(&self) -> &dyn Any { self }
///     fn as_any_mut(&mut self) -> &mut dyn Any { self }
/// }
///
/// let engine = Box::new(DummyEngine);
/// let results = run_sim(engine, 1000, Some(42), None, None, None).unwrap();
/// ```
///
/// # Errors
/// Returns an error if the hybrid engine creation or execution fails.
pub fn run_sim(
    classical_engine: Box<dyn ClassicalEngine>,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
) -> Result<ShotVec, PecosError> {
    // Runtime validation
    debug_assert!(shots > 0, "Number of shots must be positive");
    debug_assert!(
        workers.is_none_or(|w| w > 0),
        "Number of workers must be positive if specified"
    );

    // Get the number of qubits from the classical engine
    let num_qubits = classical_engine.num_qubits();
    log::debug!("run_sim: Classical engine reports {} qubits", num_qubits);

    // Use default noise model if none provided
    let noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoiseModel));

    // Create default quantum engine if none provided
    let quantum_engine = quantum_engine.unwrap_or_else(|| {
        log::debug!("run_sim: Creating StateVecEngine with {} qubits", num_qubits);
        Box::new(quantum::StateVecEngine::new(num_qubits))
    });

    // Run the simulation
    MonteCarloEngine::run_with_engines(
        classical_engine,
        noise_model,
        quantum_engine,
        shots,
        workers.unwrap_or(1),
        seed,
    )
}
