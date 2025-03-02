use crate::channels::stdio::StdioChannel;
use crate::engines::noise::NoiseModel;
use crate::engines::{ClassicalEngine, HybridEngine, QuantumEngine};
use crate::errors::QueueError;
use log::{debug, info};
use parking_lot::Mutex;
use pecos_core::types::ShotResults;
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
    pub fn builder() -> MonteCarloBuilder {
        MonteCarloBuilder::new()
    }

    /// Run a simulation with the configured engines.
    ///
    /// If no classical engine was previously configured, one will be created
    /// from the provided `program_path`.
    ///
    /// # Parameters
    /// - `program_path`: Path to the quantum program file (required if no classical engine is configured)
    /// - `num_shots`: Number of shots to run in the simulation
    /// - `num_workers`: Number of parallel workers to use
    ///
    /// # Returns
    /// - `Ok(ShotResults)`: Results from all simulation shots
    /// - `Err(QueueError)`: If an error occurs during simulation
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - Neither a classical engine nor a program path is provided
    /// - The program cannot be loaded or compiled
    /// - Engine initialization fails
    /// - Simulation execution fails
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

                // Set up channels
                let cmd_channel = StdioChannel::create_for_shot()?;
                let meas_channel = StdioChannel::create_for_shot()?;

                let quantum_engine = self.quantum_engine.clone_box();
                let noise_model = self.noise_model.clone_box();

                // Create hybrid engine for this worker
                let mut engine = HybridEngine::with_noise(
                    self.classical_engine.clone_box(),
                    quantum_engine,
                    noise_model,
                    cmd_channel,
                    meas_channel,
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
}

#[derive(Default)]
pub struct MonteCarloBuilder {
    /// Classical engine used for simulation (optional - can be provided at runtime)
    classical_engine: Option<Box<dyn ClassicalEngine>>,

    /// Noise model template to clone for workers (optional - no noise by default)
    noise_model: Option<Box<dyn NoiseModel>>,

    /// Quantum engine template to clone for workers (optional - default `StateVec` will be used)
    quantum_engine: Option<Box<dyn QuantumEngine>>,
}

impl MonteCarloBuilder {
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

    #[must_use]
    pub fn build(self) -> MonteCarloEngine {
        MonteCarloEngine {
            classical_engine: self.classical_engine.expect("ClassicalEngine is None"),
            noise_model: self.noise_model.expect("NoiseModel is None"),
            quantum_engine: self.quantum_engine.expect("QuantumEngine is None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;
    use crate::engines::classical::ClassicalEngine;
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
        type Input = CommandBatch;
        type Output = Vec<Message>;

        fn process(&mut self, _input: Self::Input) -> Result<Self::Output, QueueError> {
            // Always return a fixed measurement result
            Ok(vec![1])
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
        // Test that we can create a MonteCarloEngine with default settings
        let _engine = MonteCarloEngine::builder().build();

        // Test construction with a specific quantum engine
        let simulator = StateVec::new(2);
        let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);
        let _engine = MonteCarloEngine::builder()
            .with_quantum_engine(quantum_engine)
            .build();

        // Test construction with a specific noise model
        let noise_model = DepolarizingNoise::builder().with_probability(0.01).build();
        let _engine = MonteCarloEngine::builder()
            .with_noise_model(noise_model)
            .build();

        // Test that we can chain method calls
        let noise_model = DepolarizingNoise::builder().with_probability(0.01).build();
        let simulator = StateVec::new(2);
        let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);

        let _engine = MonteCarloEngine::builder()
            .with_quantum_engine(quantum_engine)
            .with_noise_model(noise_model);
    }

    #[test]
    fn test_run_with_program_path() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        // TODO: add builder setup

        // Test running with just a program path
        let result = MonteCarloEngine::builder().build().run(2, 1);

        assert!(result.is_ok(), "Basic run with program path should succeed");

        // Verify the result contains expected data
        let shot_results = result.unwrap();
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

        // Create depolarizing noise model
        let noise_model = DepolarizingNoise::builder().with_probability(0.05).build();

        // TODO: add classical engine builder setup

        // Test running with noise model
        let result = MonteCarloEngine::builder()
            .with_noise_model(noise_model)
            .build()
            .run(10, 1);

        assert!(result.is_ok(), "Run with noise model should succeed");

        // Create pass-through noise model
        let noise_model = Box::new(PassThroughNoise);

        // TODO: add classical engine builder setup

        // Test running with pass-through noise
        let result = MonteCarloEngine::builder()
            .with_noise_model(noise_model)
            .build()
            .run(2, 1);

        assert!(result.is_ok(), "Run with pass-through noise should succeed");
    }

    #[test]
    fn test_reuse_engine_with_different_programs() {
        // Create two different test programs
        let (_dir1, program_path1) = create_test_program();
        let (_dir2, program_path2) = create_test_program();

        // TODO: add classical and quantum engine builder setup

        // Create a configured engine
        let engine = MonteCarloEngine::builder()
            .with_noise_model(DepolarizingNoise::builder().with_probability(0.01).build())
            .build();

        // Run with first program
        let result1 = engine.run(2, 1);
        assert!(result1.is_ok(), "First run should succeed");

        // TODO: add classical and quantum engine builder setup

        // Run with second program
        let result2 = engine.run(2, 1);
        assert!(result2.is_ok(), "Second run should succeed");
    }

    #[test]
    fn test_run_with_different_parameters() {
        // Create a test program
        let (_dir, program_path) = create_test_program();

        // Create a configured engine
        let engine = MonteCarloEngine::builder().build();

        // TODO: add builder setup

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

        // Create a mock quantum engine
        let quantum_engine = Box::new(MockQuantumEngine) as Box<dyn QuantumEngine>;

        // TODO: add builder setup

        // Run with mock engine
        let result = MonteCarloEngine::builder()
            .with_quantum_engine(quantum_engine)
            .build()
            .run(5, 1);

        assert!(
            result.is_ok(),
            "Run with mock quantum engine should succeed"
        );
    }

    #[test]
    fn test_error_conditions() {
        // TODO: add builder setup

        // Test with no classical engine and no program path
        let engine = MonteCarloEngine::builder().build();
        let result = engine.run(1, 1);

        assert!(
            result.is_err(),
            "Should fail when no classical engine or program path is provided"
        );

        // Test with invalid program path
        let result = MonteCarloEngine::builder().build().run(
            // Some(Path::new("nonexistent_program.json")),
            1, 1,
        );

        assert!(result.is_err(), "Should fail with invalid program path");

        // TODO: add builder setup

        // Test with invalid shots/workers
        let (_dir, program_path) = create_test_program();
        let result = MonteCarloEngine::builder().build().run(0, 0);

        // This might not fail in the implementation, but if it does, check the error
        if result.is_err() {
            println!("Failed with zero shots/workers as expected");
        }
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

        fn handle_measurement(&mut self, measurement: Message) -> Result<(), QueueError> {
            // Extract result_id and outcome
            let result_id = (measurement >> 16) as usize;
            let outcome = measurement & 0xFFFF;

            // Store the measurement
            self.measurements
                .insert(format!("measurement_{result_id}"), outcome);

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
        type EngineInput = CommandBatch;
        type EngineOutput = Vec<Message>;

        fn start(
            &mut self,
            _input: (),
        ) -> Result<EngineStage<CommandBatch, ShotResult>, QueueError> {
            self.command_index = 0;
            self.measurements.clear();

            let commands = self.process_program()?;
            if commands.is_empty() {
                Ok(EngineStage::Complete(self.get_results()?))
            } else {
                Ok(EngineStage::NeedsProcessing(commands))
            }
        }

        fn continue_processing(
            &mut self,
            measurements: Vec<Message>,
        ) -> Result<EngineStage<CommandBatch, ShotResult>, QueueError> {
            // Handle measurements
            for measurement in measurements {
                self.handle_measurement(measurement)?;
            }

            // Get next batch of commands
            let commands = self.process_program()?;
            if commands.is_empty() {
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
