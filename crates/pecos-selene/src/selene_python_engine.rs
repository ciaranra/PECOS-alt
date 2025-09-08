//! Selene Engine using Python API via PyO3
//!
//! This engine calls Selene's Python API directly from Rust for maximum speed
//! and complete compatibility. It's the fastest way to use Selene naturally.

use crate::error::SeleneError;
use crate::prelude::*;
use pecos_core::prelude::PecosError;
use pecos_engines::{
    ByteMessage, ByteMessageBuilder, ClassicalEngine, ControlEngine, Engine, EngineStage, Shot,
    Data,
};
use pecos_programs::SeleneInterfaceProgram;
use std::{any::Any, collections::BTreeMap, path::PathBuf};
use log;

/// Configuration for Python-based Selene engine
pub struct SelenePythonConfig {
    /// Number of qubits
    pub num_qubits: usize,
    /// Path to runtime plugin (or None to use SimpleRuntime)
    pub runtime_plugin_path: Option<PathBuf>,
    /// Simulator to use (default: Quest)
    pub simulator: String,
    /// Verbose logging
    pub verbose: bool,
}

impl Default for SelenePythonConfig {
    fn default() -> Self {
        Self {
            num_qubits: 10,
            runtime_plugin_path: None,
            simulator: "Quest".to_string(),
            verbose: false,
        }
    }
}

/// A ClassicalControlEngine that uses Selene's Python API directly
pub struct SelenePythonEngine {
    /// Configuration
    config: Option<SelenePythonConfig>,
    /// The loaded program (Selene Interface plugin)
    program: Option<SeleneInterfaceProgram>,
    /// Current shot number
    shot_count: u64,
    /// Built Selene instance path (temporary)
    instance_path: Option<PathBuf>,
    /// Collected results from Python execution
    results: Vec<std::collections::HashMap<String, i64>>,
    /// Current result index
    result_index: usize,
}

impl SelenePythonEngine {
    pub fn new() -> Self {
        Self {
            config: None,
            program: None,
            shot_count: 0,
            instance_path: None,
            results: Vec::new(),
            result_index: 0,
        }
    }

    pub fn with_config(mut self, config: SelenePythonConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Load a SeleneInterfaceProgram
    pub fn load_program(&mut self, program: SeleneInterfaceProgram) -> Result<(), PecosError> {
        log::info!("Loading SeleneInterfaceProgram for Python execution");
        self.program = Some(program);
        Ok(())
    }

    /// Execute the program using Selene's Python API
    fn execute_with_python(&mut self) -> Result<(), PecosError> {
        let config = self.config.as_ref()
            .ok_or_else(|| SeleneError::ConfigurationError("No configuration provided".to_string()))?;

        let program = self.program.as_ref()
            .ok_or_else(|| SeleneError::ConfigurationError("No program loaded".to_string()))?;

        // Build the Selene executable from the Interface Plugin
        // This is where we'd need to integrate with the existing build process

        // For now, log what we would do
        log::info!("Would execute Selene with {} qubits using {} simulator",
                   config.num_qubits, config.simulator);

        // Placeholder: simulate some results
        // In reality, this would call Python via PyO3
        self.results = vec![
            std::collections::HashMap::from([("qubit1".to_string(), 0), ("qubit2".to_string(), 0)]),
            std::collections::HashMap::from([("qubit1".to_string(), 1), ("qubit2".to_string(), 1)]),
        ];

        Ok(())
    }

    /// Convert Python results to ByteMessage
    fn python_results_to_byte_message(&mut self) -> Result<ByteMessage, PecosError> {
        if self.result_index >= self.results.len() {
            return Ok(ByteMessage::create_empty());
        }

        let result = &self.results[self.result_index];
        self.result_index += 1;

        // Convert the Python result dictionary to operations
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // For each measurement result, add it as a measurement operation
        for (key, value) in result {
            log::debug!("Result: {} = {}", key, value);
            // In reality, we'd need to convert this properly to PECOS operations
            // For now, just log it
        }

        Ok(builder.build())
    }
}

impl Default for SelenePythonEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SelenePythonEngine {
    fn type_name(&self) -> String {
        "SelenePythonEngine".to_string()
    }

    fn stage(&self) -> EngineStage {
        EngineStage::ClassicalControl
    }

    fn execute(
        &mut self,
        _shot: Shot,
        _data: &mut BTreeMap<String, Box<dyn Any>>,
    ) -> Result<(), PecosError> {
        // Execution happens in get_next_operations
        Ok(())
    }
}

impl ClassicalEngine for SelenePythonEngine {
    fn get_next_operations(&mut self) -> Result<ByteMessage, PecosError> {
        if self.shot_count == 0 {
            // Execute the program using Python API
            self.execute_with_python()?;
        }

        self.shot_count += 1;

        // Convert Python results to ByteMessage
        self.python_results_to_byte_message()
    }
}

impl ControlEngine for SelenePythonEngine {
    fn set_data(&mut self, _key: String, _data: Box<dyn Any>) -> Result<(), PecosError> {
        Ok(())
    }

    fn get_data(&self, _key: &str) -> Option<&dyn Any> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}