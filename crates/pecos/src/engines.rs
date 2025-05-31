use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::noise::NoiseModel;
use pecos_engines::quantum::{QuantumEngine, StateVecEngine};
use pecos_engines::{
    ClassicalEngine, MonteCarloEngine, PassThroughNoiseModel, core::shot_results::ShotResults,
};
use std::path::Path;
// Import the QirEngine from pecos-qir
use pecos_qir::QirEngine;

// We no longer need the SimulationInput enum as we'll work directly with the classical engine

/// Sets up a basic QASM engine.
///
/// This function creates a QASM engine from the provided path.
///
/// # Parameters
///
/// - `program_path`: A reference to the path of the QASM program file
/// - `seed`: Optional seed value for deterministic execution
///
/// # Returns
///
/// Returns a `Box<dyn ClassicalEngine>` containing the QASM engine
///
/// # Errors
///
/// This function may return the following errors:
/// - `PecosError::IO`: If the QASM file cannot be read
/// - `PecosError::Processing`: If the QASM engine creation fails or if parsing fails
pub fn setup_qasm_engine(
    program_path: &Path,
    seed: Option<u64>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    debug!("Setting up QASM engine for: {}", program_path.display());

    // Note: The seed parameter is unused as QASMEngine doesn't handle randomness.
    // Randomness is managed by the QuantumEngine in MonteCarloEngine.
    // The seed parameter is kept for API consistency with other engines.
    let _ = seed;

    // Use the QASMEngine from the pecos-qasm crate
    let engine = pecos_qasm::QASMEngine::from_file(program_path).map_err(|e| {
        PecosError::Processing(format!(
            "QASM engine setup failed: Could not create engine: {e}"
        ))
    })?;

    Ok(Box::new(engine))
}

/// Sets up a basic QIR engine.
///
/// This function creates a QIR engine from the provided path.
///
/// # Parameters
///
/// - `program_path`: A reference to the path of the QIR program file
/// - `shots`: Optional number of shots to assign to the engine
///
/// # Returns
///
/// Returns a `Box<dyn ClassicalEngine>` containing the QIR engine
///
/// # Errors
///
/// This function may return the following errors:
/// - `PecosError::Compilation`: If the QIR file cannot be compiled
/// - `PecosError::Processing`: If the QIR engine fails to process commands
pub fn setup_qir_engine(
    program_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    debug!("Setting up QIR engine for: {}", program_path.display());

    // Create a QirEngine from the path
    let mut engine = QirEngine::new(program_path.to_path_buf());

    // Set the number of shots assigned to this engine if specified
    if let Some(num_shots) = shots {
        engine.set_assigned_shots(num_shots)?;
    }

    // Pre-compile the QIR library for efficient cloning
    engine.pre_compile()?;

    Ok(Box::new(engine))
}

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
/// The `ShotResults` structure containing measurement results for each shot
///
/// # Examples
///
/// ```
/// use pecos::prelude::*;
/// use std::str::FromStr;
///
/// // Bell state in OpenQASM
/// let program_str = r#"
/// OPENQASM 2.0;
/// include "qelib1.inc";
/// qreg q[2];
/// creg c[2];
/// h q[0];
/// cx q[0], q[1];
/// measure q -> c;
/// "#;
///
/// // Option 1: Parse QASM string to engine directly
/// let engine1 = QASMEngine::from_str(program_str).unwrap();
/// let results1 = run_sim(Box::new(engine1), 1000, Some(42), None, None, None).unwrap();
///
/// // Option 2: Use the QASMProgram type with into_engine_box for maximum convenience
/// let program = QASMProgram::from_str(program_str).unwrap();
/// let results2 = run_sim(program.into_engine_box(), 1000, Some(42), None, None, None).unwrap();
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
) -> Result<ShotResults, PecosError> {
    // Get the number of qubits from the classical engine
    let num_qubits = classical_engine.num_qubits();

    // Use default noise model if none provided
    let noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoiseModel));

    // Create default quantum engine if none provided
    let quantum_engine =
        quantum_engine.unwrap_or_else(|| Box::new(StateVecEngine::new(num_qubits)));

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
