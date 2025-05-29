use crate::QASMEngine;
use pecos_core::errors::PecosError;
use pecos_engines::noise::NoiseModel;
use pecos_engines::quantum::{QuantumEngine, StateVecEngine};
use pecos_engines::{ClassicalEngine, MonteCarloEngine, PassThroughNoiseModel};
use std::collections::HashMap;
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
pub fn run_qasm_sim(
    qasm: &str,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
) -> Result<HashMap<String, Vec<u32>>, PecosError> {
    let classical_engine = QASMEngine::from_str(qasm)?;
    let num_qubits = classical_engine.num_qubits();

    // Use default noise model if none provided
    let noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoiseModel));

    // Create default quantum engine if none provided
    let quantum_engine =
        quantum_engine.unwrap_or_else(|| Box::new(StateVecEngine::new(num_qubits)));

    let results = MonteCarloEngine::run_with_engines(
        Box::new(classical_engine),
        noise_model,
        quantum_engine,
        shots,
        workers.unwrap_or(1),
        seed,
    )?
    .register_shots;

    Ok(results)
}
