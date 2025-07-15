use crate::config::{LlvmSimConfig, QuantumEngineType};
use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::{
    ClassicalEngine, MonteCarloEngine,
    shot_results::{Data, Shot},
};
use pecos_llvm_runtime::{LlvmEngine, LlvmEngineConfig};
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;
use tempfile::NamedTempFile;

/// A built LLVM simulation ready to run.
///
/// This struct holds a compiled LLVM engine and configuration for running
/// quantum circuit simulations with various options for noise and parallelization.
pub struct LlvmSimulation {
    /// The LLVM execution engine
    engine: LlvmEngine,
    /// Simulation configuration
    config: LlvmSimConfig,
    /// Temporary file for LLVM IR (if created from string)
    /// Kept alive to prevent deletion while engine is in use
    _temp_file: Option<NamedTempFile>,
    /// Statistics
    total_shots: usize,
    total_runs: usize,
}

impl LlvmSimulation {
    /// Create a new simulation from LLVM IR string and configuration.
    pub(crate) fn new(llvm_ir: String, config: LlvmSimConfig) -> Result<Self, PecosError> {
        // Create temporary file for LLVM IR
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| PecosError::with_context(e, "Failed to create temp file for LLVM IR"))?;

        std::io::Write::write_all(&mut temp_file, llvm_ir.as_bytes())
            .map_err(|e| PecosError::with_context(e, "Failed to write LLVM IR to temp file"))?;

        // Ensure the file is flushed to disk
        temp_file
            .flush()
            .map_err(|e| PecosError::with_context(e, "Failed to flush LLVM IR to temp file"))?;

        let llvm_path = temp_file.path().to_path_buf();

        // Create LLVM engine with configuration
        let engine_config = LlvmEngineConfig {
            assigned_shots: 0, // Will be set per run
            verbose: config.verbose,
            max_qubits: config.max_qubits,
        };

        let engine = LlvmEngine::with_config(llvm_path, engine_config);

        Ok(Self {
            engine,
            config,
            _temp_file: Some(temp_file),
            total_shots: 0,
            total_runs: 0,
        })
    }

    /// Run the simulation for the specified number of shots.
    ///
    /// Returns a columnar format with register names as keys and vectors of values.
    pub fn run(&mut self, shots: usize) -> Result<HashMap<String, Vec<i64>>, PecosError> {
        if shots == 0 {
            return Ok(HashMap::new());
        }

        let start = Instant::now();
        debug!("Running LLVM simulation with {shots} shots");

        // Get number of qubits from the engine
        let num_qubits = self.engine.num_qubits();
        if num_qubits == 0 {
            return Err(PecosError::Input(
                "Cannot run simulation: LLVM program has no qubits allocated".to_string(),
            ));
        }

        // Create noise model
        let noise_model = self.config.noise_model.clone().create_noise_model();

        // Run using MonteCarloEngine for parallelization and noise
        let shot_vec = if let Some(max_qubits) = self.config.max_qubits {
            // Use max_qubits when specified to handle dynamic allocation in loops
            MonteCarloEngine::run_with_noise_model_and_max_qubits(
                Box::new(self.engine.clone()),
                noise_model,
                max_qubits,
                shots,
                self.config.workers,
                self.config.seed,
            )?
        } else {
            // Fallback to the original method for backward compatibility
            MonteCarloEngine::run_with_noise_model(
                Box::new(self.engine.clone()),
                noise_model,
                shots,
                self.config.workers,
                self.config.seed,
            )?
        };

        // Convert to columnar format
        let columnar = self.shots_to_columnar(shot_vec.shots);

        let elapsed = start.elapsed();
        self.total_shots += shots;
        self.total_runs += 1;

        debug!(
            "Completed {} shots in {:.3}s (total: {} shots, {} runs)",
            shots,
            elapsed.as_secs_f64(),
            self.total_shots,
            self.total_runs
        );

        Ok(columnar)
    }

    /// Run with custom quantum engine type.
    ///
    /// This allows using a specific quantum engine type instead of the configured one.
    pub fn run_with_quantum_engine(
        &mut self,
        shots: usize,
        quantum_engine_type: QuantumEngineType,
    ) -> Result<HashMap<String, Vec<i64>>, PecosError> {
        // Temporarily override the quantum engine type
        let original_type = self.config.quantum_engine;
        self.config.quantum_engine = quantum_engine_type;

        let result = self.run(shots);

        // Restore original type
        self.config.quantum_engine = original_type;

        result
    }

    /// Convert shot results to columnar format.
    fn shots_to_columnar(&self, shots: Vec<Shot>) -> HashMap<String, Vec<i64>> {
        let mut columnar = HashMap::new();

        if shots.is_empty() {
            return columnar;
        }

        // Get all register names from first shot
        let register_names: Vec<String> = if let Some(first_shot) = shots.first() {
            first_shot.data.keys().cloned().collect()
        } else {
            return columnar;
        };

        // Initialize columns
        for name in &register_names {
            columnar.insert(name.clone(), Vec::with_capacity(shots.len()));
        }

        // Fill columns
        for shot in &shots {
            for name in &register_names {
                if let Some(data) = shot.data.get(name) {
                    let value = match data {
                        Data::U32(v) => i64::from(*v),
                        Data::I64(v) => *v,
                        Data::F64(v) => *v as i64,
                        Data::Bool(v) => i64::from(*v),
                        _ => 0,
                    };
                    columnar.get_mut(name).unwrap().push(value);
                } else {
                    columnar.get_mut(name).unwrap().push(0);
                }
            }
        }

        // If no named registers, create a default "_result" register
        if columnar.is_empty() {
            let values: Vec<i64> = shots.iter().map(|_| 0).collect();
            columnar.insert("_result".to_string(), values);
        }

        columnar
    }

    /// Get statistics about the simulation.
    ///
    /// Returns (`total_shots`, `total_runs`).
    #[must_use]
    pub fn stats(&self) -> (usize, usize) {
        (self.total_shots, self.total_runs)
    }

    /// Get the underlying LLVM engine.
    #[must_use]
    pub fn engine(&self) -> &LlvmEngine {
        &self.engine
    }

    /// Get the number of qubits in the circuit.
    #[must_use]
    pub fn num_qubits(&self) -> usize {
        self.engine.num_qubits()
    }
}
