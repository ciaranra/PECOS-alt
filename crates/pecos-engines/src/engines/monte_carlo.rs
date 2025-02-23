use crate::channels::stdio::StdioChannel;
use crate::engines::HybridEngine;
use crate::engines::classical::{ProgramType, detect_program_type, setup_engine};
use crate::engines::quantum::new_quantum_engine_arbitrary_qgate;
use crate::errors::QueueError;
use log::info;
use pecos_core::types::ShotResults;
use pecos_noise::NoiseModel;
use pecos_qsim::StateVec;
use std::path::PathBuf;

// TODO: Program should be taken ownership and copied per parallel instance
// TODO: Engines should all be spun up independently per thread and reset/reuse
//       assuming threads are used to run multiple shots

/// A high-level engine that orchestrates Monte Carlo simulations of quantum programs.
///
/// This engine manages the parallel execution of multiple shots of a quantum program,
/// coordinating the classical and quantum components through a hybrid engine setup.
/// It handles program loading, noise model application, and result aggregation.
pub struct MonteCarloEngine {
    num_workers: usize,
    program_path: PathBuf,
    noise_model: Option<Box<dyn NoiseModel>>,
}

impl MonteCarloEngine {
    // TODO: Optionally pass the in-memory programs
    // TODO: Move num_shots and workers declaration together.
    /// Creates a new Monte Carlo simulation engine.
    ///
    /// # Parameters
    /// - `program_path`: Path to the quantum program file (QIR or PHIR format)
    /// - `num_workers`: Number of parallel workers to use for simulation
    ///
    /// # Returns
    /// A new `MonteCarloEngine` instance configured for parallel simulation.
    #[must_use]
    pub fn new(program_path: PathBuf, num_workers: usize) -> Self {
        Self {
            num_workers,
            program_path,
            noise_model: None,
        }
    }

    /// Sets the noise model to be applied during simulation.
    ///
    /// # Parameters
    /// - `noise_model`: Optional noise model to apply to quantum operations
    pub fn set_noise_model(&mut self, noise_model: Option<Box<dyn NoiseModel>>) {
        self.noise_model = noise_model;
    }

    /// Runs the quantum program for the specified number of shots.
    ///
    /// This method:
    /// 1. Sets up the classical and quantum engines
    /// 2. Handles program compilation if needed (for QIR)
    /// 3. Executes the simulation across multiple workers
    /// 4. Collects and aggregates results
    ///
    /// # Parameters
    /// - `num_shots`: Total number of times to run the quantum circuit
    ///
    /// # Returns
    /// - `Ok(ShotResults)`: Results from all simulation shots
    /// - `Err(QueueError)`: If an error occurs during simulation
    ///
    /// # Errors
    /// Returns a `QueueError` if:
    /// - The program cannot be loaded or compiled
    /// - Engine initialization fails
    /// - Simulation execution fails
    pub fn run_program(&self, num_shots: usize) -> Result<ShotResults, QueueError> {
        info!(
            "Starting Monte Carlo simulation with {} shots across {} workers",
            num_shots, self.num_workers
        );

        // Create base engines
        let classical_engine = setup_engine(&self.program_path)?;

        // For QIR, ensure it's compiled first
        if let ProgramType::QIR = detect_program_type(&self.program_path)? {
            classical_engine.compile()?;
        }

        let simulator = StateVec::new(2); // TODO: Get qubit count from program analysis
        let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);
        let cmd_channel = StdioChannel::from_stdio()?;

        // Setup hybrid engine
        let engine = HybridEngine::new(
            classical_engine,
            quantum_engine,
            cmd_channel.clone(),
            cmd_channel,
        );

        // Set noise model if configured
        if let Some(noise_model) = &self.noise_model {
            engine.set_noise_model(Some(noise_model.clone_box()));
        }

        // Run simulation using the existing parallel implementation
        engine.run_parallel(num_shots, self.num_workers)
    }
}
