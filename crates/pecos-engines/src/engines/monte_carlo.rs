use crate::engines::noise::{DepolarizingNoise, NoiseModel, PassThroughNoise};
use crate::engines::quantum::new_quantum_engine_arbitrary_qgate;
use crate::engines::{ClassicalEngine, HybridEngine, QuantumEngine};
use crate::errors::QueueError;
use log::{debug, info};
use parking_lot::Mutex;
use pecos_core::types::ShotResults;
use pecos_qsim::StateVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;

/// A high-level engine that orchestrates Monte Carlo simulations of quantum programs.
///
/// This engine manages the parallel execution of multiple shots of a quantum program,
/// coordinating the classical and quantum components through a hybrid engine setup.
/// It handles program loading, noise model application, and result aggregation.
pub struct MonteCarloEngine {
    /// Classical engine used for simulation (optional - can be provided at runtime)
    classical_engine: Box<dyn ClassicalEngine>,

    /// Noise model template to clone for workers (optional - no noise by default)
    noise_model: Box<dyn NoiseModel>,

    /// Quantum engine template to clone for workers (optional - default `StateVec` will be used)
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
    pub fn run(&self, num_shots: usize, num_workers: usize) -> Result<ShotResults, QueueError> {
        info!(
            "Starting Monte Carlo simulation with {} shots across {} workers",
            num_shots, num_workers
        );

        // Validate and adjust worker count
        let effective_workers = if num_workers == 0 {
            1 // Minimum of 1 worker
        } else if num_workers > num_shots {
            num_shots // Don't use more workers than shots
        } else {
            num_workers
        };

        // Storage for results from all shots
        let shot_results = Arc::new(Mutex::new(Vec::with_capacity(num_shots)));

        // Calculate shots per worker
        let base_shots_per_worker = num_shots / effective_workers;
        let extra_shots = num_shots % effective_workers;

        // Create worker pool
        (0..effective_workers)
            .into_par_iter()
            .try_for_each::<_, Result<(), QueueError>>(|worker_idx| {
                // Calculate shots for this worker
                let worker_shots = if worker_idx < extra_shots {
                    base_shots_per_worker + 1
                } else {
                    base_shots_per_worker
                };

                debug!(
                    "Worker {} initializing for {} shots",
                    worker_idx, worker_shots
                );

                // Create hybrid engine for this worker
                let mut engine = HybridEngine::with_noise(
                    self.classical_engine.clone_box(),
                    self.quantum_engine.clone_box(),
                    self.noise_model.clone_box(),
                );

                // Process all shots assigned to this worker
                for _shot_num in 0..worker_shots {
                    let result = engine.run_shot()?;
                    engine.reset()?;
                    shot_results.lock().push(result);
                }

                debug!("Worker {} completed all shots", worker_idx);
                Ok(())
            })?;

        // Process all results
        let results = Arc::try_unwrap(shot_results)
            .map_err(|_| QueueError::LockError("Failed to unwrap results Arc".into()))?
            .into_inner();

        Ok(ShotResults::from_measurements(&results))
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
        // TODO: Return an error...
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
    use crate::engines::classical::{ClassicalEngine, setup_engine};
    use crate::engines::noise::{DepolarizingNoise, PassThroughNoise};
    use crate::engines::phir::PHIREngine;
    use crate::engines::quantum::{CliffordEngine, new_quantum_engine_arbitrary_qgate};
    use crate::engines::{ControlEngine, Engine, EngineStage};
    use pecos_core::types::{CommandBatch, GateType, QuantumCommand, ShotResult};
    use pecos_qsim::{StateVec, StdSparseStab};
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // Helper to create a mock quantum engine for testing
    struct MockQuantumEngine;

    impl Engine for MockQuantumEngine {
        type Input = ByteMessage;
        type Output = ByteMessage;

        fn process(&mut self, _input: Self::Input) -> Result<Self::Output, QueueError> {
            // Always return a fixed measurement result
            let measurement = (0u32 << 16) | 1u32; // result_id=0, outcome=1
            ByteMessage::create_measurements(&[measurement])
        }

        fn reset(&mut self) -> Result<(), QueueError> {
            Ok(())
        }
    }

    impl QuantumEngine for MockQuantumEngine {
        fn clone_box(&self) -> Box<dyn QuantumEngine> {
            Box::new(MockQuantumEngine)
        }
    }

    // Helper to create a simple test PHIR program file
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
            .with_classical_engine(classical_engine.clone_box())
            .with_quantum_engine(quantum_engine.clone_box())
            .build();

        // Test construction with a specific noise model
        let noise_model = DepolarizingNoise::builder().with_probability(0.01).build();
        let _engine = MonteCarloEngine::builder()
            .with_classical_engine(classical_engine.clone_box())
            .with_noise_model(noise_model)
            .with_quantum_engine(quantum_engine.clone_box())
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
        let stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(CliffordEngine::new(stabilizer));

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

        let stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(CliffordEngine::new(stabilizer));

        // Test running with noise model
        let result = MonteCarloEngine::builder()
            .with_classical_engine(classical_engine.clone_box())
            .with_noise_model(noise_model)
            .with_quantum_engine(quantum_engine.clone_box())
            .build()
            .run(10, 1);

        assert!(result.is_ok(), "Run with noise model should succeed");

        // Create pass-through noise model
        let noise_model = Box::new(PassThroughNoise);

        // Test running with pass-through noise
        let result = MonteCarloEngine::builder()
            .with_classical_engine(classical_engine)
            .with_noise_model(noise_model)
            .with_quantum_engine(quantum_engine)
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

        let stabilizer = StdSparseStab::new(2);
        let quantum_engine = Box::new(CliffordEngine::new(stabilizer));

        let engine = MonteCarloEngine::builder()
            .with_classical_engine(classical_engine)
            .with_noise_model(noise_model)
            .with_quantum_engine(quantum_engine)
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
            .with_classical_engine(classical_engine)
            .with_quantum_engine(quantum_engine)
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
    #[should_panic(expected = "attempt to divide by zero")]
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
        let simulator = StateVec::new(2);
        let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);

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
    // This implementation needs to be updated to work with ByteMessage
    #[derive(Debug)]
    struct ExternalClassicalEngine {
        commands: Vec<QuantumCommand>,
        measurements: HashMap<String, u32>,
        command_index: usize,
        current_shot: usize,
    }

    impl ExternalClassicalEngine {
        fn new() -> Self {
            // Create a simple Bell state preparation circuit
            let commands = vec![
                QuantumCommand {
                    gate: GateType::H,
                    qubits: vec![0],
                },
                QuantumCommand {
                    gate: GateType::CX,
                    qubits: vec![0, 1],
                },
                QuantumCommand {
                    gate: GateType::Measure { result_id: 0 },
                    qubits: vec![0],
                },
                QuantumCommand {
                    gate: GateType::Measure { result_id: 1 },
                    qubits: vec![1],
                },
            ];

            Self {
                commands,
                measurements: HashMap::new(),
                command_index: 0,
                current_shot: 0,
            }
        }
    }

    impl Clone for ExternalClassicalEngine {
        fn clone(&self) -> Self {
            Self {
                commands: self.commands.clone(),
                measurements: HashMap::new(),
                command_index: 0,
                current_shot: 0,
            }
        }
    }

    impl ClassicalEngine for ExternalClassicalEngine {
        fn num_qubits(&self) -> usize {
            // If we have no commands, return 0
            if self.commands.is_empty() {
                return 0;
            }

            // Find the highest qubit index used in any command
            let mut max_qubit_index = 0;
            for cmd in &self.commands {
                for &qubit in &cmd.qubits {
                    if qubit > max_qubit_index {
                        max_qubit_index = qubit;
                    }
                }
            }

            // The number of qubits is max_qubit_index + 1 (since indices start at 0)
            max_qubit_index + 1
        }

        // The old process_program method still works for backward compatibility
        fn process_program(&mut self) -> Result<CommandBatch, QueueError> {
            // If we've processed all commands, return empty batch
            if self.command_index >= self.commands.len() {
                return Ok(CommandBatch::new());
            }

            // Create a batch with the next command
            let mut batch = CommandBatch::new();
            batch.add_command(self.commands[self.command_index].clone());
            self.command_index += 1;

            Ok(batch)
        }

        // New method for ByteMessage architecture
        fn generate_commands(&mut self) -> Result<ByteMessage, QueueError> {
            let batch = self.process_program()?;
            if batch.is_empty() {
                ByteMessage::create_flush(true)
            } else {
                ByteMessage::create_quantum_operations(&batch)
            }
        }

        // The old handle_measurement method for backward compatibility
        fn handle_measurement(&mut self, measurement: u32) -> Result<(), QueueError> {
            // Extract result_id and outcome
            let result_id = (measurement >> 16) as usize;
            let outcome = measurement & 0xFFFF;

            // Store the measurement
            self.measurements
                .insert(format!("measurement_{result_id}"), outcome);

            Ok(())
        }

        // New method for ByteMessage architecture
        fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), QueueError> {
            let measurements = message.parse_measurements()?;
            for measurement in measurements {
                self.handle_measurement(measurement)?;
            }
            Ok(())
        }

        fn get_results(&self) -> Result<ShotResult, QueueError> {
            // Process all measurements into a "result" string
            let mut result_string = String::new();

            // Sort keys to ensure consistent ordering
            let mut keys: Vec<_> = self.measurements.keys().collect();
            keys.sort();

            for key in keys {
                if let Some(&value) = self.measurements.get(key) {
                    result_string.push_str(&value.to_string());
                }
            }

            // Create a ShotResult with both individual measurements and the combined result
            let mut result_measurements = self.measurements.clone();
            if !result_string.is_empty() {
                if let Ok(value) = result_string.parse::<u32>() {
                    result_measurements.insert("result".to_string(), value);
                }
            }

            Ok(ShotResult {
                measurements: result_measurements,
            })
        }

        fn compile(&self) -> Result<(), Box<dyn Error>> {
            // No compilation needed for this mock engine
            Ok(())
        }

        fn reset(&mut self) -> Result<(), QueueError> {
            self.command_index = 0;
            self.measurements.clear();
            self.current_shot += 1;
            Ok(())
        }

        fn clone_box(&self) -> Box<dyn ClassicalEngine> {
            Box::new(self.clone())
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
            self.measurements.clear();

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
