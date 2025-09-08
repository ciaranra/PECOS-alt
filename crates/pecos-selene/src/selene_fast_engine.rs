//! Fast Selene Engine using SeleneInstance directly
//!
//! This is the fastest and most natural way to use Selene - directly using
//! SeleneInstance from the selene-sim crate with our ByteMessage runtime.

use crate::error::SeleneError;
use pecos_core::prelude::PecosError;
use pecos_engines::{ByteMessage, ByteMessageBuilder, ClassicalEngine, Engine, Shot};
use pecos_programs::SeleneInterfaceProgram;
use std::io::Write;
use std::{any::Any, collections::BTreeMap, path::PathBuf};
use tempfile::NamedTempFile;

/// Configuration for fast Selene engine
#[derive(Clone)]
pub struct SeleneFastConfig {
    /// Number of qubits
    pub num_qubits: usize,
    /// Path to ByteMessage runtime plugin
    pub runtime_plugin_path: PathBuf,
    /// Verbose logging
    pub verbose: bool,
}

/// The fastest Selene engine - uses SeleneInstance directly in the same process
pub struct SeleneFastEngine {
    /// Configuration
    config: Option<SeleneFastConfig>,
    /// The loaded program (Selene Interface plugin)
    program: Option<SeleneInterfaceProgram>,
    /// Current shot number
    shot_count: u64,
    /// Selene instance (loaded on first execution) - use String placeholder for now
    selene_instance: Option<String>, // Placeholder until we integrate real SeleneInstance
    /// Results collected from execution
    execution_results: Vec<BTreeMap<String, i64>>,
    /// Current result index
    result_index: usize,
}

impl SeleneFastEngine {
    pub fn new() -> Self {
        Self {
            config: None,
            program: None,
            shot_count: 0,
            selene_instance: None,
            execution_results: Vec::new(),
            result_index: 0,
        }
    }

    pub fn with_config(mut self, config: SeleneFastConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Load a SeleneInterfaceProgram
    pub fn load_program(&mut self, program: SeleneInterfaceProgram) -> Result<(), PecosError> {
        log::info!("Loading SeleneInterfaceProgram for fast execution");
        self.program = Some(program);
        Ok(())
    }

    /// Initialize Selene instance using our configuration
    fn initialize_selene(&mut self) -> Result<(), PecosError> {
        let config = self.config.as_ref().ok_or_else(|| {
            SeleneError::CompilationError("No configuration provided".to_string())
        })?;

        let program = self
            .program
            .as_ref()
            .ok_or_else(|| SeleneError::CompilationError("No program loaded".to_string()))?;

        log::info!("Initializing Selene with {} qubits", config.num_qubits);

        // Create a temporary file for the Interface Plugin
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to create temp file: {}", e)))?;

        temp_file.write_all(&program.plugin).map_err(|e| {
            SeleneError::RuntimeError(format!("Failed to write Interface Plugin: {}", e))
        })?;

        let plugin_path = temp_file.into_temp_path();

        // Create Selene configuration
        // This is where we'd use selene-sim's Configuration struct
        // For now, this is a placeholder showing the approach

        log::info!("Would create Selene configuration with:");
        log::info!("  Interface Plugin: {:?}", plugin_path);
        log::info!("  Runtime Plugin: {:?}", config.runtime_plugin_path);
        log::info!("  Qubits: {}", config.num_qubits);

        // TODO: Actually create and store SeleneInstance
        // This would involve:
        // 1. Creating selene_sim::selene_instance::configuration::Configuration
        // 2. Setting up simulator, error_model, runtime plugins
        // 3. Creating selene_sim::selene_instance::SeleneInstance::new(config)
        // 4. Storing it in self.selene_instance

        // For now, simulate execution results
        self.execution_results = vec![
            BTreeMap::from([("qubit1".to_string(), 0), ("qubit2".to_string(), 0)]),
            BTreeMap::from([("qubit1".to_string(), 1), ("qubit2".to_string(), 1)]),
        ];

        Ok(())
    }

    /// Execute using the loaded Selene instance
    fn execute_selene(&mut self) -> Result<(), PecosError> {
        if self.selene_instance.is_none() {
            self.initialize_selene()?;
        }

        log::debug!("Executing Selene instance");

        // TODO: Actually execute the Selene instance
        // This would involve calling methods on the SeleneInstance to:
        // 1. Start a shot
        // 2. Execute the Interface Plugin (which calls our runtime)
        // 3. Collect results
        // 4. End the shot

        Ok(())
    }

    /// Convert Selene results to ByteMessage
    fn results_to_byte_message(&mut self) -> Result<ByteMessage, PecosError> {
        if self.result_index >= self.execution_results.len() {
            return Ok(ByteMessage::create_empty());
        }

        let result = &self.execution_results[self.result_index];
        self.result_index += 1;

        // Convert results to ByteMessage operations
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Add measurement results
        for (key, &value) in result {
            log::debug!("Result: {} = {}", key, value);
            // In a real implementation, we'd add appropriate operations
            // For now, just create an empty operation set
        }

        Ok(builder.build())
    }
}

impl Default for SeleneFastEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SeleneFastEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        <Self as ClassicalEngine>::reset(self)?;
        // Process all commands until done
        loop {
            let commands = self.generate_commands()?;
            if commands.is_empty()? {
                break;
            }
            // Simulate handling measurements
            let empty_measurements = ByteMessage::create_empty();
            self.handle_measurements(empty_measurements)?;
        }
        self.get_results()
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}

impl ClassicalEngine for SeleneFastEngine {
    fn num_qubits(&self) -> usize {
        self.config.as_ref().map(|c| c.num_qubits).unwrap_or(1)
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        if self.shot_count == 0 {
            // Execute Selene on first call
            self.execute_selene()?;
        }

        self.shot_count += 1;

        // Return operations from Selene execution
        self.results_to_byte_message()
    }

    fn handle_measurements(&mut self, _message: ByteMessage) -> Result<(), PecosError> {
        // Handle measurement results
        Ok(())
    }

    fn get_results(&self) -> Result<Shot, PecosError> {
        Ok(Shot::default())
    }

    fn compile(&self) -> Result<(), PecosError> {
        Ok(())
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.shot_count = 0;
        self.result_index = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Clone for SeleneFastEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            program: self.program.clone(),
            shot_count: 0,         // Reset for cloned instance
            selene_instance: None, // Each clone gets fresh instance
            execution_results: Vec::new(),
            result_index: 0,
        }
    }
}

// No need for clone_trait_object! for concrete types
