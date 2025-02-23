use crate::channels::stdio::StdioChannel;
use crate::engines::HybridEngine;
use crate::engines::classical::{ProgramType, detect_program_type, setup_engine};
use crate::engines::quantum::new_quantum_engine_arbitrary_qgate;
use crate::errors::QueueError;
use log::{debug, info};
use parking_lot::{Mutex, RwLock};
use pecos_core::types::{GateType, ShotResult, ShotResults};
use pecos_noise::NoiseModel;
use pecos_qsim::StateVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::prelude::*;
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

        // Create storage for results
        let shot_results = Arc::new(Mutex::new(Vec::with_capacity(num_shots)));

        // Compile QIR program if needed
        if let ProgramType::QIR = detect_program_type(&self.program_path)? {
            let engine = setup_engine(&self.program_path)?;
            engine.compile()?;
        }

        // Run parallel shots
        (0..num_shots)
            .into_par_iter()
            .with_max_len(1) // Process 1 item per thread to avoid contention
            .try_for_each::<_, Result<(), QueueError>>(|shot_idx| {
                debug!("Starting shot {}", shot_idx);

                // Create fresh engines and channels for this shot
                let classical_engine = setup_engine(&self.program_path)?;
                let simulator = StateVec::new(2);
                let quantum_engine = new_quantum_engine_arbitrary_qgate(simulator);
                let channel = StdioChannel::create_for_shot()?;

                // Create hybrid engine for this shot
                let mut engine =
                    HybridEngine::new(classical_engine, quantum_engine, channel.clone(), channel);

                // Apply noise model if configured
                if let Some(noise_model) = &self.noise_model {
                    engine.set_noise_model(Some(noise_model.clone_box()));
                }

                // Run single shot and collect results
                let result = engine.run_shot()?;
                shot_results.lock().push(result);

                debug!("Completed shot {}", shot_idx);
                Ok(())
            })?;

        // Process and return results
        let results = Arc::try_unwrap(shot_results)
            .expect("Arc should be uniquely owned")
            .into_inner();
        Ok(ShotResults::from_measurements(&results))
    }

    // /// Runs a parallel execution of quantum circuits for a specified number of shots.
    // ///
    // /// # Parameters
    // ///
    // /// - `shots`: The total number of shots to execute in parallel.
    // /// - `workers`: The number of workers to use for parallel execution.
    // ///
    // /// # Returns
    // ///
    // /// Returns a `ShotResults` object containing the processed results for all shots,
    // /// or a `QueueError` if an error occurs during execution.
    // ///
    // /// # Errors
    // ///
    // /// This function may return the following errors:
    // /// - `QueueError::OperationError` if an operation is not supported.
    // /// - `QueueError::ExecutionError` if the quantum engine execution fails.
    // /// - `QueueError::LockError` if there is a failure in acquiring or unwrapping a lock.
    // pub fn run_parallel(&self, shots: usize, workers: usize) -> Result<ShotResults, QueueError> {
    //     // TODO: classical engine should be able to send multiple rounds of commands off
    //
    //     info!(
    //         "Starting parallel execution with {} shots and {} workers",
    //         shots, workers
    //     );
    //
    //     let shot_results = Arc::new(Mutex::new(Vec::with_capacity(shots)));
    //
    //     // Get commands just once from classical engine
    //     // TODO: It should not be just once... and it should be inside the parallel loop...
    //     let base_commands = {
    //         let mut classical = self.classical.write();
    //         let cmds = classical.process_program()?;
    //         debug!("Generated base commands: {:?}", cmds);
    //         cmds
    //     };
    //
    //     // Get noise model reference outside the loop
    //     let noise_model = self.noise_model.read();
    //
    //     (0..shots)
    //         .into_par_iter()
    //         .try_for_each::<_, Result<(), QueueError>>(|shot_idx| {
    //             debug!("Starting shot {}", shot_idx);
    //             let mut shot_result = ShotResult::default();
    //
    //             // Clone the base commands for this shot
    //             let mut commands = base_commands.clone();
    //
    //             // Apply noise model independently for this shot
    //             if let Some(noise_model) = &*noise_model {
    //                 commands = noise_model.apply_noise(commands);
    //                 debug!(
    //                     "Applied noise model for shot {}, commands: {:?}",
    //                     shot_idx, commands
    //                 );
    //             }
    //
    //             // Process commands through quantum engine
    //             {
    //                 let mut quantum = self.quantum.write();
    //                 // Reset quantum state before processing this shot
    //                 quantum.reset()?;
    //
    //                 for cmd in &commands {
    //                     if let Some(measurement) = quantum.process(cmd.clone())? {
    //                         let GateType::Measure { result_id: res_id } = cmd.gate else {
    //                             continue;
    //                         };
    //                         shot_result
    //                             .measurements
    //                             .insert(format!("measurement_{res_id}"), measurement);
    //                     }
    //                 }
    //             }
    //
    //             shot_results.lock().push(shot_result);
    //             debug!("Completed shot {}", shot_idx);
    //             Ok(())
    //         })?;
    //
    //     let mutex = Arc::try_unwrap(shot_results)
    //         .map_err(|_| QueueError::LockError("Could not unwrap results".into()))?;
    //
    //     let raw_results = mutex.into_inner();
    //
    //     // Convert to our new ShotResults type
    //     let results = ShotResults::from_measurements(&raw_results);
    //
    //     // Print results
    //     // results.print();
    //
    //     Ok(results)
    // }
}
