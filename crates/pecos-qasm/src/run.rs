use crate::QASMEngine;
use pecos_core::errors::PecosError;
use pecos_engines::noise::NoiseModel;
use pecos_engines::quantum::{QuantumEngine, StateVecEngine};
use pecos_engines::{ClassicalEngine, MonteCarloEngine, PassThroughNoiseModel, ShotResults};
use std::str::FromStr;

/// Run a QASM simulation with detailed control over noise model and quantum engine
///
/// This function takes a QASM string and runs a simulation with the specified settings.
/// For more type safety, consider using [`QASMProgram`] instead of raw QASM strings.
///
/// # Parameters
/// * `qasm` - QASM code as a string
/// * `shots` - Number of shots to run
/// * `seed` - Optional seed for reproducibility
/// * `workers` - Optional number of workers for parallelization (default: 1)
/// * `noise_model` - Optional custom noise model to use (default: `PassThroughNoiseModel`)
/// * `quantum_engine` - Optional custom quantum engine to use (default: `StateVecEngine`)
///
/// # Errors
///
/// Returns an error if QASM parsing or simulation fails.
pub fn run_qasm_sim(
    qasm: &str,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
) -> Result<ShotResults, PecosError> {
    let classical_engine = QASMEngine::from_str(qasm)?;
    let num_qubits = classical_engine.num_qubits();

    // Use default noise model if none provided
    let noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoiseModel));

    // Create default quantum engine if none provided
    let quantum_engine =
        quantum_engine.unwrap_or_else(|| Box::new(StateVecEngine::new(num_qubits)));

    MonteCarloEngine::run_with_engines(
        Box::new(classical_engine),
        noise_model,
        quantum_engine,
        shots,
        workers.unwrap_or(1),
        seed,
    )
}
