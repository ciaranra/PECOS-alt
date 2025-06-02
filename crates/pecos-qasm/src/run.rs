use crate::{QASMEngine, QASMResults};
use pecos_core::errors::PecosError;
use pecos_engines::noise::NoiseModel;
use pecos_engines::quantum::{QuantumEngine, StateVecEngine};
use pecos_engines::{ClassicalEngine, MonteCarloEngine, PassThroughNoiseModel};
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
/// # Returns
///
/// A [`QASMResults`] containing the simulation results with convenient formatting methods
/// for binary and decimal output.
///
/// # Errors
///
/// Returns an error if QASM parsing or simulation fails.
///
/// # Example
///
/// ```no_run
/// # use pecos_qasm::run_qasm_sim;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let qasm = r#"
///     OPENQASM 2.0;
///     include "qelib1.inc";
///     qreg q[2];
///     creg c[2];
///     h q[0];
///     cx q[0], q[1];
///     measure q -> c;
/// "#;
///
/// let results = run_qasm_sim(qasm, 100, Some(42), None, None, None)?;
///
/// // Direct access to formatting methods
/// println!("{}", results.to_compact_json());
/// println!("Outcome counts: {:?}", results.outcome_counts());
///
/// // Access the underlying ShotVec if needed
/// println!("Number of shots: {}", results.len());
/// # Ok(())
/// # }
/// ```
pub fn run_qasm_sim(
    qasm: &str,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
) -> Result<QASMResults, PecosError> {
    // Parse QASM to get register information
    let engine = QASMEngine::from_str(qasm)?;
    let num_qubits = engine.num_qubits();

    // Use default noise model if none provided
    let noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoiseModel));

    // Create default quantum engine if none provided
    let quantum_engine =
        quantum_engine.unwrap_or_else(|| Box::new(StateVecEngine::new(num_qubits)));

    // Run simulation
    let shot_results = MonteCarloEngine::run_with_engines(
        Box::new(engine),
        noise_model,
        quantum_engine,
        shots,
        workers.unwrap_or(1),
        seed,
    )?;

    // Return results wrapped in QASMResults
    Ok(QASMResults::new(shot_results))
}
