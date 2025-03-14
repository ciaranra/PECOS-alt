use crate::engines::HybridEngine;
use crate::engines::noise::{DepolarizingNoise, NoiseModel, PassThroughNoise};
use crate::engines::quantum::new_quantum_engine_arbitrary_qgate;
use crate::engines::{ClassicalEngine, QuantumEngine};
use crate::errors::QueueError;
use crate::shot_results::ShotResults;
use dyn_clone;
use log::{debug, info};
use parking_lot::Mutex;
use pecos_qsim::StateVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;

/// A high-level engine that orchestrates Monte Carlo simulations of quantum programs.
///
/// This engine manages the parallel execution of multiple shots of a quantum program,
/// coordinating the classical and quantum components through a hybrid engine setup.
/// It handles program loading, noise model application, and result aggregation.
pub struct MonteCarloEngine {
    /// Classical engine used for simulation
    classical_engine: Box<dyn ClassicalEngine>,

    /// Noise model template to clone for workers
    noise_model: Box<dyn NoiseModel>,

    /// Quantum engine template to clone for workers
    quantum_engine: Box<dyn QuantumEngine>,
}

impl MonteCarloEngine {
    /// Create a new Monte Carlo engine with default settings.
    #[must_use]
    pub fn builder() -> MonteCarloEngineBuilder {
        MonteCarloEngineBuilder::new()
    }

    /// Run a simulation with the configured engines.
    ///
    ///
    /// # Parameters
    /// - `num_shots`: Number of shots to run in the simulation
    /// - `num_workers`: Number of parallel workers to use
    ///
    /// # Returns
    /// - `Ok(ShotResults)`: Results from all simulation shots
    /// - `Err(QueueError)`: If an error occurs during simulation
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - Engine initialization fails
    ///
    /// # Panics
    ///
    /// This function will panic if `num_shots` is zero.
    pub fn run(&self, num_shots: usize, num_workers: usize) -> Result<ShotResults, QueueError> {
        info!(
            "Running Monte Carlo simulation with {} shots and {} workers",
            num_shots, num_workers
        );

        assert_ne!(num_shots, 0, "Number of shots must be greater than 0");

        // Determine how many shots to run per worker
        let shots_per_worker = num_shots.div_ceil(num_workers);
        debug!(
            "Running {} shots per worker ({} workers)",
            shots_per_worker, num_workers
        );

        // Create a vector to store results from each worker
        let results = Arc::new(Mutex::new(Vec::with_capacity(num_shots)));

        // Create a vector of worker indices
        let worker_indices: Vec<usize> = (0..num_workers).collect();

        // Run workers in parallel
        worker_indices
            .into_par_iter()
            .try_for_each(|worker_idx| -> Result<(), QueueError> {
                // Calculate the range of shots for this worker
                let start_shot = worker_idx * shots_per_worker;
                let end_shot = std::cmp::min(start_shot + shots_per_worker, num_shots);
                let worker_shots = end_shot - start_shot;

                if worker_shots > 0 {
                    debug!(
                        "Worker {} running shots {}-{}",
                        worker_idx,
                        start_shot,
                        end_shot - 1
                    );

                    // Create a hybrid engine for this worker
                    let mut hybrid_engine = HybridEngine::with_noise(
                        dyn_clone::clone_box(&*self.classical_engine),
                        dyn_clone::clone_box(&*self.quantum_engine),
                        dyn_clone::clone_box(&*self.noise_model),
                    );

                    // Process all shots assigned to this worker
                    for _shot_num in start_shot..end_shot {
                        let result = hybrid_engine.run_shot()?;
                        hybrid_engine.reset()?;
                        results.lock().push(result);
                    }

                    debug!("Worker {} completed all shots", worker_idx);
                }
                Ok(())
            })?;

        // Process all results
        let results_vec = Arc::try_unwrap(results)
            .map_err(|_| QueueError::LockError("Failed to unwrap results Arc".into()))?
            .into_inner();

        // TODO: Consider refactoring to collect ByteMessage instances directly and use
        // ShotResults::from_byte_messages for more efficient and context-aware processing.
        // This would require storing a mapping from result_id to register name.
        Ok(ShotResults::from_measurements(&results_vec))
    }

    /// Run a simulation using the provided engines directly.
    ///
    /// # Parameters
    /// - `classical_engine`: The classical engine to use for the simulation.
    /// - `noise_model`: The noise model to apply during the simulation.
    /// - `quantum_engine`: The quantum engine to use for the simulation.
    /// - `num_shots`: The number of shots to execute in the simulation.
    /// - `num_workers`: The number of parallel workers to use.
    ///
    /// # Returns
    /// - `Ok(ShotResults)`: The results from the simulation.
    /// - `Err(QueueError)`: If an error occurs during the configuration or simulation.
    ///
    /// # Errors
    /// This function will return a `QueueError` if:
    /// - The engines fail to build properly.
    /// - There is an error during the execution of the simulation.
    pub fn run_with_engines(
        classical_engine: Box<dyn ClassicalEngine>,
        noise_model: Box<dyn NoiseModel>,
        quantum_engine: Box<dyn QuantumEngine>,
        num_shots: usize,
        num_workers: usize,
    ) -> Result<ShotResults, QueueError> {
        MonteCarloEngine::builder()
            .with_classical_engine(classical_engine)
            .with_noise_model(noise_model)
            .with_quantum_engine(quantum_engine)
            .build()
            .run(num_shots, num_workers)
    }
    // TODO: Format ShotResults into JSON

    /// Run a Monte Carlo simulation using only a classical engine.
    ///
    /// This method automatically configures a depolarizing noise model and a quantum engine
    /// based on the number of qubits in the provided classical engine.
    ///
    /// # Parameters
    /// - `classical_engine`: The classical engine used for simulation.
    /// - `p`: Probability for depolarizing noise (0.0 - 1.0).
    /// - `num_shots`: Number of shots to execute.
    /// - `num_workers`: Number of parallel workers to use.
    ///
    /// # Returns
    /// - `Ok(ShotResults)`: Results of the simulation shots.
    /// - `Err(QueueError)`: If an error occurs during the configuration or execution.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - The number of qubits is invalid.
    /// - Noise or quantum engine initialization fails.
    /// - Simulation execution fails.
    pub fn run_with_classical_engine(
        classical_engine: Box<dyn ClassicalEngine>,
        p: f64,
        num_shots: usize,
        num_workers: usize,
    ) -> Result<ShotResults, QueueError> {
        let num_qubits = classical_engine.num_qubits();
        let noise_model = DepolarizingNoise::builder().with_probability(p).build();
        let quantum_engine = new_quantum_engine_arbitrary_qgate(StateVec::new(num_qubits));

        MonteCarloEngine::builder()
            .with_classical_engine(classical_engine)
            .with_noise_model(noise_model)
            .with_quantum_engine(quantum_engine)
            .build()
            .run(num_shots, num_workers)
    }

    /// Run a Monte Carlo simulation using configuration.
    ///
    /// # Parameters
    /// - `config`: Configuration for the simulation.
    /// - `num_shots`: Number of shots to execute.
    /// - `num_workers`: Number of parallel workers to use.
    ///
    /// # Returns
    /// - `Ok(ShotResults)`: The results of the simulation shots.
    /// - `Err(QueueError)`: If an error occurs during the setup or execution.
    ///
    /// # Errors
    /// This function will return a `QueueError` if:
    /// - The configuration string is invalid or cannot be parsed.
    /// - Simulation execution fails.
    #[allow(unused_variables)]
    pub fn run_with_config(
        config: &str,
        num_shots: usize,
        num_workers: usize,
    ) -> Result<ShotResults, QueueError> {
        todo!()
    }
}

impl Clone for MonteCarloEngine {
    fn clone(&self) -> Self {
        MonteCarloEngine {
            classical_engine: dyn_clone::clone_box(&*self.classical_engine),
            quantum_engine: dyn_clone::clone_box(&*self.quantum_engine),
            noise_model: dyn_clone::clone_box(&*self.noise_model),
        }
    }
}

#[derive(Default)]
pub struct MonteCarloEngineBuilder {
    /// Classical engine used for simulation (optional - can be provided at runtime)
    classical_engine: Option<Box<dyn ClassicalEngine>>,

    /// Noise model template to clone for workers (optional - no noise by default)
    noise_model: Option<Box<dyn NoiseModel>>,

    /// Quantum engine template to clone for workers (optional - default `StateVec` will be used)
    quantum_engine: Option<Box<dyn QuantumEngine>>,
}

impl MonteCarloEngineBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the classical engine to use for simulation.
    #[must_use]
    pub fn with_classical_engine(mut self, engine: Box<dyn ClassicalEngine>) -> Self {
        self.classical_engine = Some(engine);
        self
    }

    /// Set the noise model to use as template for workers.
    #[must_use]
    pub fn with_noise_model(mut self, model: Box<dyn NoiseModel>) -> Self {
        self.noise_model = Some(model);
        self
    }

    /// Set the quantum engine to use as template for workers.
    #[must_use]
    pub fn with_quantum_engine(mut self, engine: Box<dyn QuantumEngine>) -> Self {
        self.quantum_engine = Some(engine);
        self
    }

    /// Builds and returns a configured `MonteCarloEngine`.
    ///
    /// # Panics
    /// Panics if `classical_engine`, `noise_model`, or `quantum_engine` are not set.
    #[must_use]
    pub fn build(self) -> MonteCarloEngine {
        MonteCarloEngine {
            classical_engine: self.classical_engine.expect("ClassicalEngine is None"),
            noise_model: self
                .noise_model
                .unwrap_or_else(|| Box::new(PassThroughNoise)),
            quantum_engine: self.quantum_engine.expect("QuantumEngine is None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::ByteMessage;
    use crate::channels::byte::gate_type::{GateTypeId, QuantumGate};
    use crate::engines::ControlEngine;
    use crate::engines::Engine;
    use crate::engines::EngineStage;
    use crate::engines::classical::setup_engine;
    use crate::engines::phir::PHIREngine;
    use crate::engines::quantum::StateVecEngine;
    use crate::shot_results::ShotResult;
    use pecos_qsim::StdSparseStab;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[derive(Debug, Clone)]
    struct MockQuantumEngine;

    impl Engine for MockQuantumEngine {
        type Input = ByteMessage;
        type Output = ByteMessage;

        fn process(&mut self, _input: Self::Input) -> Result<Self::Output, QueueError> {
            // Always return a fixed measurement result
            // result_id=0, outcome=1
            Ok(ByteMessage::record_measurement_results(&[(0, 1)]))
        }

        fn reset(&mut self) -> Result<(), QueueError> {
            Ok(())
        }
    }

    impl QuantumEngine for MockQuantumEngine {
        fn set_seed(&mut self, _seed: u64) -> Result<(), QueueError> {
            // Mock implementation - doesn't actually use the seed
            Ok(())
        }
    }

    /// Creates a temporary test program file for testing
    ///
    /// This helper function creates a temporary directory and writes a simple
    /// PHIR program to a file within it. The program includes basic quantum
    /// operations like Hadamard and CNOT gates.
    ///
    /// # Returns
    /// A tuple containing the temporary directory (to keep it alive) and the path to the program file
    fn create_test_program() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let program_path = dir.path().join("test_program.json");

        let program_content = r#"{
  "format": "PHIR/JSON",
  "version": "0.1.0",
  "metadata": {"description": "Bell state preparation"},
  "ops": [
    {
      "data": "qvar_define",
      "data_type": "qubits",
      "variable": "q",
      "size": 2
    },
    {
      "data": "cvar_define",
      "data_type": "i64",
      "variable": "m",
      "size": 2
    },
    {
      "data": "cvar_define",
      "data_type": "i64",
      "variable": "result",
      "size": 2
    },
    {"qop": "H", "args": [["q", 0]]},
    {"qop": "CX", "args": [["q", 0], ["q", 1]]},
    {"qop": "Measure", "args": [["q", 0]], "returns": [["m", 0]]},
    {"qop": "Measure", "args": [["q", 1]], "returns": [["m", 1]]},
    {"cop": "Result", "args": [["m", 0]], "returns": [["result", 0]]},
    {"cop": "Result", "args": [["m", 1]], "returns": [["result", 1]]}
  ]
}"#;

        let mut file = File::create(&program_path).unwrap();
        file.write_all(program_content.as_bytes()).unwrap();

        (dir, program_path)
    }

    #[test]
    fn test_basic_construction() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        let classical_engine = setup_engine(&program_path).expect("Could not setup engine");

        // Test construction with a specific quantum engine
        let simulator = StateVec::new(2);
        let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);
        let _engine = MonteCarloEngine::builder()
            .with_classical_engine(dyn_clone::clone_box(&*classical_engine))
            .with_quantum_engine(dyn_clone::clone_box(&*quantum_engine))
            .build();

        // Test construction with a specific noise model
        let noise_model = DepolarizingNoise::builder().with_probability(0.01).build();
        let _engine = MonteCarloEngine::builder()
            .with_classical_engine(dyn_clone::clone_box(&*classical_engine))
            .with_noise_model(noise_model)
            .with_quantum_engine(dyn_clone::clone_box(&*quantum_engine))
            .build();
    }

    #[test]
    fn test_run_with_program_path() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        let classical_engine = setup_engine(&program_path).expect("Could not setup engine");

        // Test running with just a program path
        let results = MonteCarloEngine::run_with_classical_engine(classical_engine, 0.0, 2, 1);

        assert!(
            results.is_ok(),
            "Basic run with program path should succeed"
        );

        // Verify the result contains expected data
        let shot_results = results.unwrap();
        assert_eq!(shot_results.shots.len(), 2, "Should have 2 shots");
    }

    #[test]
    fn test_run_with_custom_engines() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        // Create the PHIR engine directly
        let classical_engine = PHIREngine::new(&program_path).unwrap();

        // Create a quantum engine with a specific simulator
        let _stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(StateVecEngine::new(2));

        // Test running with custom engines
        let result = MonteCarloEngine::builder()
            .with_classical_engine(Box::new(classical_engine))
            .with_quantum_engine(quantum_engine)
            .build()
            .run(2, 1);

        assert!(result.is_ok(), "Run with custom engines should succeed");
    }

    #[test]
    fn test_run_with_noise_model() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        let classical_engine = setup_engine(&program_path).expect("Could not setup engine");

        // Create depolarizing noise model
        let noise_model = DepolarizingNoise::builder().with_probability(0.05).build();

        let _stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(StateVecEngine::new(2));

        // Test running with noise model
        let result = MonteCarloEngine::builder()
            .with_classical_engine(dyn_clone::clone_box(&*classical_engine))
            .with_noise_model(noise_model)
            .with_quantum_engine(dyn_clone::clone_box(&*quantum_engine))
            .build()
            .run(10, 1);

        assert!(result.is_ok(), "Run with noise model should succeed");

        // Create pass-through noise model
        let noise_model = Box::new(PassThroughNoise);

        // Test running with pass-through noise
        let result = MonteCarloEngine::builder()
            .with_classical_engine(dyn_clone::clone_box(&*classical_engine))
            .with_noise_model(noise_model)
            .with_quantum_engine(dyn_clone::clone_box(&*quantum_engine))
            .build()
            .run(2, 1);

        assert!(result.is_ok(), "Run with pass-through noise should succeed");
    }

    #[test]
    fn test_run_with_different_parameters() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        // Create a configured engines
        let classical_engine = setup_engine(&program_path).expect("Could not setup engine");

        // Create depolarizing noise model
        let noise_model = DepolarizingNoise::builder().with_probability(0.05).build();

        let _stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(StateVecEngine::new(2));

        let engine = MonteCarloEngine::builder()
            .with_classical_engine(dyn_clone::clone_box(&*classical_engine))
            .with_noise_model(noise_model)
            .with_quantum_engine(dyn_clone::clone_box(&*quantum_engine))
            .build();

        // Run with different shots and workers
        let result1 = engine.run(2, 1);
        assert!(result1.is_ok(), "Run with 2 shots, 1 worker should succeed");

        let result2 = engine.run(4, 2);
        assert!(
            result2.is_ok(),
            "Run with 4 shots, 2 workers should succeed"
        );

        // Verify shot counts
        let shots1 = result1.unwrap().shots.len();
        let shots2 = result2.unwrap().shots.len();

        assert_eq!(shots1, 2, "First run should have 2 shots");
        assert_eq!(shots2, 4, "Second run should have 4 shots");
    }

    #[test]
    fn test_mock_quantum_engine() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        // Create a configured engines
        let classical_engine = setup_engine(&program_path).expect("Could not setup engine");

        // Create a mock quantum engine
        let quantum_engine = Box::new(MockQuantumEngine) as Box<dyn QuantumEngine>;

        // Run with mock engine
        let result = MonteCarloEngine::builder()
            .with_classical_engine(dyn_clone::clone_box(&*classical_engine))
            .with_quantum_engine(dyn_clone::clone_box(&*quantum_engine))
            .build()
            .run(5, 1);

        assert!(
            result.is_ok(),
            "Run with mock quantum engine should succeed"
        );
    }

    #[test]
    #[should_panic(expected = "ClassicalEngine is None")]
    fn test_monte_carlo_engine_build_panics() {
        let _engine = MonteCarloEngine::builder().build();
    }

    #[test]
    #[should_panic(expected = "Number of shots must be greater than 0")]
    fn test_zero_shots_panics() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        let classical_engine = setup_engine(&program_path).expect("Could not setup engine");

        // Test running with just a program path
        let _results = MonteCarloEngine::run_with_classical_engine(classical_engine, 0.0, 0, 1);
    }

    #[test]
    fn test_with_external_classical_engine() {
        // Create a mock external classical engine
        let external_engine = Box::new(ExternalClassicalEngine::new());

        // Create a quantum engine
        let _stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(StateVecEngine::new(2));

        // Create a MonteCarloEngine with the external engine
        let engine = MonteCarloEngine::builder()
            .with_classical_engine(external_engine)
            .with_quantum_engine(quantum_engine)
            .build();

        // Run the simulation
        let result = engine.run(10, 2);

        assert!(
            result.is_ok(),
            "Simulation with external engine should succeed"
        );

        // Verify we have the expected number of shots
        let shot_results = result.unwrap();
        assert_eq!(shot_results.shots.len(), 10, "Should have 10 shots");

        // Each shot should have results
        for shot in &shot_results.shots {
            assert!(
                shot.contains_key("result"),
                "Each shot should have a result"
            );
        }
    }

    // Mock implementation of an external classical engine
    #[derive(Debug, Clone)]
    struct ExternalClassicalEngine {
        // Store the circuit definition as quantum gates
        gates: Vec<QuantumGate>,
        command_index: usize,
        current_shot: usize,
        measurements: HashMap<String, u32>,
    }

    impl ExternalClassicalEngine {
        fn new() -> Self {
            // Create a simple Bell state preparation circuit
            let gates = vec![
                QuantumGate::h(0),
                QuantumGate::cx(0, 1),
                QuantumGate::measure(0, 0),
                QuantumGate::measure(1, 1),
            ];

            Self {
                gates,
                command_index: 0,
                current_shot: 0,
                measurements: HashMap::new(),
            }
        }

        // Helper method to build a ByteMessage for a specific gate
        fn build_message_for_gate(gate: &QuantumGate) -> ByteMessage {
            ByteMessage::create_with_quantum_gate(gate)
        }

        // Helper method to get the maximum qubit index
        fn get_max_qubit_index(&self) -> usize {
            let mut max_qubit = 0;
            for gate in &self.gates {
                for &qubit in &gate.qubits {
                    if qubit > max_qubit {
                        max_qubit = qubit;
                    }
                }
            }
            max_qubit
        }
    }

    impl ClassicalEngine for ExternalClassicalEngine {
        fn num_qubits(&self) -> usize {
            // If we have no commands, return 0
            if self.gates.is_empty() {
                return 0;
            }

            // Find the highest qubit index used in any command
            self.get_max_qubit_index() + 1
        }

        fn generate_commands(&mut self) -> Result<ByteMessage, QueueError> {
            // If we've processed all commands, return empty batch (flush message)
            if self.command_index >= self.gates.len() {
                return Ok(ByteMessage::create_flush());
            }

            // Get the next command
            let gate = &self.gates[self.command_index];
            self.command_index += 1;

            // Build the message based on the gate
            let message = Self::build_message_for_gate(gate);
            Ok(message)
        }

        fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), QueueError> {
            let measurements = message.parse_measurements()?;

            // Store the measurements
            for (result_id, outcome) in measurements {
                self.measurements
                    .insert(format!("measurement_{result_id}"), outcome);
            }

            Ok(())
        }

        fn get_results(&self) -> Result<ShotResult, QueueError> {
            // TODO: If the measurements are already available in a ByteMessage format,
            // consider using ShotResult::from_byte_message for more efficient processing.
            // This would require maintaining a mapping from result_id to register name.

            // Create a string representation of the combined result
            let mut result_string = String::new();

            // Sort gates by result_id for consistent ordering
            let mut measurement_gates: Vec<_> = self
                .gates
                .iter()
                .filter(|gate| gate.gate_type == GateTypeId::Measure)
                .collect();

            measurement_gates.sort_by_key(|gate| gate.result_id);

            // Add each measurement to the result string
            for gate in measurement_gates {
                if let Some(result_id) = gate.result_id {
                    let key = format!("measurement_{result_id}");
                    if let Some(outcome) = self.measurements.get(&key) {
                        result_string.push_str(&outcome.to_string());
                    }
                }
            }

            // Create a ShotResult with both individual measurements and the combined result
            let mut result_measurements = HashMap::new();
            if !result_string.is_empty() {
                if let Ok(value) = result_string.parse::<u32>() {
                    result_measurements.insert("result".to_string(), value);
                }
            }

            // Add individual measurements
            for (key, outcome) in &self.measurements {
                result_measurements.insert(key.clone(), *outcome);
            }

            Ok(ShotResult {
                measurements: result_measurements,
            })
        }

        fn compile(&self) -> Result<(), Box<dyn std::error::Error>> {
            // No compilation needed for this mock
            Ok(())
        }

        fn reset(&mut self) -> Result<(), QueueError> {
            self.command_index = 0;
            self.measurements.clear();
            self.current_shot += 1;
            Ok(())
        }
    }

    impl ControlEngine for ExternalClassicalEngine {
        type Input = ();
        type Output = ShotResult;
        type EngineInput = ByteMessage;
        type EngineOutput = ByteMessage;

        fn start(
            &mut self,
            _input: (),
        ) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
            self.command_index = 0;

            let commands = self.generate_commands()?;
            if commands.is_empty().unwrap_or(false) {
                Ok(EngineStage::Complete(self.get_results()?))
            } else {
                Ok(EngineStage::NeedsProcessing(commands))
            }
        }

        fn continue_processing(
            &mut self,
            measurements: ByteMessage,
        ) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
            // Handle measurements
            self.handle_measurements(measurements)?;

            // Get next batch of commands
            let commands = self.generate_commands()?;
            if commands.is_empty().unwrap_or(false) {
                Ok(EngineStage::Complete(self.get_results()?))
            } else {
                Ok(EngineStage::NeedsProcessing(commands))
            }
        }

        fn reset(&mut self) -> Result<(), QueueError> {
            ClassicalEngine::reset(self)
        }
    }
}
