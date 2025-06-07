use crate::simulation::{NoiseModelType, QuantumEngineType, qasm_sim};
use pecos_core::errors::PecosError;
use pecos_engines::noise::NoiseModel;
use pecos_engines::quantum::QuantumEngine;
use pecos_engines::shot_results::ShotVec;

/// Run a QASM simulation with detailed control over noise model and quantum engine
///
/// **Note**: This function is maintained for backward compatibility. For new code,
/// consider using the more ergonomic [`qasm_sim`] builder API instead:
///
/// ```
/// use pecos_qasm::prelude::*;
/// let qasm = "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; creg c[1]; h q[0]; measure q[0] -> c[0];";
/// let results = qasm_sim(qasm).seed(42).run(100)?;
/// assert_eq!(results.len(), 100);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
/// A [`ShotVec`] containing the simulation results. This can be converted to
/// [`ShotMap`] for columnar access via `try_as_shot_map()`
///
/// # Errors
///
/// Returns an error if QASM parsing or simulation fails.
///
/// # Example
///
/// ```
/// use pecos_qasm::run_qasm_sim;
/// use pecos_engines::ShotMapDisplayExt;
///
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
/// let shot_vec = run_qasm_sim(qasm, 100, Some(42), None, None, None)?;
///
/// // Convert to ShotMap for display and analysis
/// let shot_map = shot_vec.try_as_shot_map()?;
/// assert_eq!(shot_map.num_shots(), 100);
///
/// // Access specific data
/// let measurements = shot_map.try_bits_as_decimal("c")?;
/// assert_eq!(measurements.len(), 100);
///
/// // Check Bell state results (should be 0 or 3)
/// for m in &measurements {
///     let val: u64 = m.parse()?;
///     assert!(val == 0 || val == 3);
/// }
///
/// assert_eq!(shot_vec.len(), 100);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_qasm_sim(
    qasm: &str,
    shots: usize,
    seed: Option<u64>,
    workers: Option<usize>,
    noise_model: Option<Box<dyn NoiseModel>>,
    quantum_engine: Option<Box<dyn QuantumEngine>>,
) -> Result<ShotVec, PecosError> {
    // Build the simulation using the new API
    let mut builder = qasm_sim(qasm);

    // Set seed if provided
    if let Some(s) = seed {
        builder = builder.seed(s);
    }

    // Set workers
    builder = builder.workers(workers.unwrap_or(1));

    // For noise model and quantum engine, we need to use the boxed types directly
    // since the new API uses enums. For now, we'll use defaults from the new API
    // when custom models are provided, as converting from trait objects to enums
    // is not straightforward.

    // Note: This maintains backward compatibility but doesn't fully utilize
    // the custom noise/engine parameters. Users should migrate to the new API
    // for full control.
    if noise_model.is_some() || quantum_engine.is_some() {
        // Fall back to original implementation for custom models
        use crate::QASMEngine;
        use pecos_engines::quantum::StateVecEngine;
        use pecos_engines::{ClassicalEngine, MonteCarloEngine, PassThroughNoiseModel};
        use std::str::FromStr;

        let engine = QASMEngine::from_str(qasm)?;
        let num_qubits = engine.num_qubits();

        let noise_model = noise_model.unwrap_or_else(|| Box::new(PassThroughNoiseModel));
        let quantum_engine =
            quantum_engine.unwrap_or_else(|| Box::new(StateVecEngine::new(num_qubits)));

        MonteCarloEngine::run_with_engines(
            Box::new(engine),
            noise_model,
            quantum_engine,
            shots,
            workers.unwrap_or(1),
            seed,
        )
    } else {
        // Use the new API when no custom models are provided
        builder.run(shots)
    }
}

/// Run a QASM simulation with a clean, modern API
///
/// This function provides a simple interface for running QASM simulations,
/// similar to the Python `run_qasm` function. It uses the `qasm_sim` builder
/// internally.
///
/// # Parameters
/// * `qasm` - QASM code as a string
/// * `shots` - Number of shots to run
/// * `noise_model` - Optional noise model configuration
/// * `engine` - Optional quantum engine type (defaults to `SparseStabilizer`)
/// * `workers` - Optional number of workers for parallelization (default: 1)
/// * `seed` - Optional seed for reproducibility
///
/// # Returns
///
/// A [`ShotVec`] containing the simulation results.
///
/// # Example
///
/// ```
/// # use pecos_qasm::prelude::*;
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
/// // Simple usage - ideal simulation (no noise)
/// let results = run_qasm(qasm, 100, PassThroughNoise, None, None, None)?;
/// assert_eq!(results.len(), 100);
///
/// // With depolarizing noise
/// let results = run_qasm(
///     qasm,
///     1000,
///     DepolarizingNoise { p: 0.01 },
///     Some(QuantumEngineType::StateVector),
///     Some(4),  // workers
///     Some(42), // seed
/// )?;
/// assert_eq!(results.len(), 1000);
///
/// // With custom noise parameters
/// let custom_noise = DepolarizingCustomNoise {
///     p_prep: 0.001,
///     p_meas: 0.01,
///     p1: 0.005,
///     p2: 0.02,
/// };
/// let results = run_qasm(qasm, 100, custom_noise, None, None, Some(42))?;
///
/// // Check results are Bell states
/// let shot_map = results.try_as_shot_map()?;
/// let values = shot_map.try_bits_as_u64("c")?;
/// for val in &values[..10] {  // Check first 10
///     assert!(*val == 0 || *val == 3 || *val == 1 || *val == 2); // With noise, all outcomes possible
/// }
/// # Ok(())
/// # }
/// ```
pub fn run_qasm<N>(
    qasm: &str,
    shots: usize,
    noise_model: N,
    engine: Option<QuantumEngineType>,
    workers: Option<usize>,
    seed: Option<u64>,
) -> Result<ShotVec, PecosError>
where
    N: Into<NoiseModelType>,
{
    let mut builder = qasm_sim(qasm).noise(noise_model);

    if let Some(e) = engine {
        builder = builder.quantum_engine(e);
    }

    if let Some(w) = workers {
        builder = builder.workers(w);
    }

    if let Some(s) = seed {
        builder = builder.seed(s);
    }

    builder.run(shots)
}
