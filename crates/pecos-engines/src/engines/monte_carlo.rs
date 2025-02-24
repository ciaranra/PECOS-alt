use crate::channels::stdio::StdioChannel;
use crate::engines::HybridEngine;
use crate::engines::classical::setup_engine;
use crate::engines::quantum::new_quantum_engine_arbitrary_qgate;
use crate::errors::QueueError;
use log::{debug, info};
use parking_lot::Mutex;
use pecos_core::types::ShotResults;
use pecos_noise::NoiseModel;
use pecos_qsim::StateVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::PathBuf;
use std::sync::Arc;

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

        // Storage for results from all shots
        let shot_results = Arc::new(Mutex::new(Vec::with_capacity(num_shots)));

        // Calculate shots per worker
        let base_shots_per_worker = num_shots / self.num_workers;
        let extra_shots = num_shots % self.num_workers;

        // Create worker pool
        (0..self.num_workers)
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

                // Each worker gets its own engines
                let classical_engine = setup_engine(&self.program_path)?;
                let simulator = StateVec::new(2);
                let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);

                // Create hybrid engine for this worker
                let cmd_channel = StdioChannel::create_for_shot()?;
                let meas_channel = StdioChannel::create_for_shot()?;

                let mut engine =
                    HybridEngine::new(classical_engine, quantum_engine, cmd_channel, meas_channel);

                // Apply noise model if configured
                if let Some(noise_model) = &self.noise_model {
                    engine.set_noise_model(Some(noise_model.clone_box()));
                }

                // Process all shots assigned to this worker
                for shot_num in 0..worker_shots {
                    debug!(
                        "Worker {} starting shot {}/{}",
                        worker_idx,
                        shot_num + 1,
                        worker_shots
                    );
                    let result = engine.run_shot()?;
                    shot_results.lock().push(result);
                    debug!(
                        "Worker {} completed shot {}/{}",
                        worker_idx,
                        shot_num + 1,
                        worker_shots
                    );
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
