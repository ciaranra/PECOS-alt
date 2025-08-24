//! Selene Native Runner Engine
//!
//! This engine uses Selene's native Python runner directly instead of
//! extracting and running the Interface Plugin separately. This preserves
//! the result recording infrastructure that Selene provides.

use pecos_core::prelude::PecosError;
use pecos_engines::{
    ByteMessage, ByteMessageBuilder, ClassicalEngine, ControlEngine, Engine, EngineStage, Shot,
    Data,
};
use pecos_programs::SeleneInterfaceProgram;
use std::{any::Any, collections::BTreeMap};
use crate::SeleneError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

/// Configuration for Selene Native Runner execution
#[derive(Clone)]
pub struct SeleneNativeRunnerConfig {
    pub num_qubits: usize,
    pub verbose: bool,
    pub simulator_name: String,
}

impl Default for SeleneNativeRunnerConfig {
    fn default() -> Self {
        Self {
            num_qubits: 10,
            verbose: false,
            simulator_name: "Quest".to_string(),
        }
    }
}

/// Engine that uses Selene's native runner infrastructure
pub struct SeleneNativeRunnerEngine {
    config: SeleneNativeRunnerConfig,
    program: Option<SeleneInterfaceProgram>,
    results: BTreeMap<String, Data>,
    shot_count: u64,
    hugr_package: Option<PyObject>,
    runner: Option<PyObject>,
}

impl SeleneNativeRunnerEngine {
    pub fn new(num_qubits: usize) -> Result<Self, PecosError> {
        Ok(Self {
            config: SeleneNativeRunnerConfig {
                num_qubits,
                verbose: false,
                simulator_name: "Quest".to_string(),
            },
            program: None,
            results: BTreeMap::new(),
            shot_count: 0,
            hugr_package: None,
            runner: None,
        })
    }
    
    pub fn with_program(mut self, program: SeleneInterfaceProgram) -> Self {
        self.program = Some(program);
        self
    }
    
    pub fn with_simulator(mut self, simulator_name: String) -> Self {
        self.config.simulator_name = simulator_name;
        self
    }
    
    /// Initialize Selene's native runner from the executable path
    fn initialize_native_runner(&mut self) -> Result<(), PecosError> {
        let program = self.program.as_ref()
            .ok_or_else(|| SeleneError::NoProgramSpecified)?;
        
        log::info!("Initializing Selene native runner");
        
        Python::with_gil(|py| -> Result<(), PecosError> {
            // We need to reconstruct the HUGR from the executable
            // For now, we'll try to read it from the artifacts directory
            let artifacts_path = std::path::Path::new(&program.artifacts_path);
            
            // Look for a HUGR file in the artifacts
            let hugr_file = artifacts_path.join("program.hugr");
            if !hugr_file.exists() {
                return Err(PecosError::Processing(
                    "HUGR file not found in artifacts directory. Native runner requires HUGR.".to_string()
                ));
            }
            
            // Read the HUGR bytes
            let hugr_bytes = std::fs::read(&hugr_file)
                .map_err(|e| PecosError::Processing(format!("Failed to read HUGR file: {}", e)))?;
            
            // Import hugr-py to deserialize
            let hugr_module = py.import("hugr")
                .map_err(|e| PecosError::Processing(format!("Failed to import hugr: {}", e)))?;
            
            let package_class = hugr_module.getattr("Package")
                .map_err(|e| PecosError::Processing(format!("Failed to get Package class: {}", e)))?;
            
            // Deserialize the HUGR
            let hugr_package = package_class.call_method1("from_json", (hugr_bytes,))
                .map_err(|e| PecosError::Processing(format!("Failed to deserialize HUGR: {}", e)))?;
            
            // Import selene_sim.build
            let selene_build = py.import("selene_sim.build")
                .map_err(|e| PecosError::Processing(format!("Failed to import selene_sim.build: {}", e)))?;
            
            let build_fn = selene_build.getattr("build")
                .map_err(|e| PecosError::Processing(format!("Failed to get build function: {}", e)))?;
            
            // Build the runner
            let runner = build_fn.call((hugr_package, "pecos_native_runner"), None)
                .map_err(|e| PecosError::Processing(format!("Failed to build Selene runner: {}", e)))?;
            
            self.hugr_package = Some(hugr_package.to_object(py));
            self.runner = Some(runner.to_object(py));
            
            log::info!("Selene native runner initialized successfully");
            Ok(())
        })
    }
    
    /// Execute a single shot using Selene's native runner
    fn execute_shot(&mut self) -> Result<BTreeMap<String, Data>, PecosError> {
        let runner = self.runner.as_ref()
            .ok_or_else(|| PecosError::Processing("Runner not initialized".to_string()))?;
        
        Python::with_gil(|py| -> Result<BTreeMap<String, Data>, PecosError> {
            // Import the simulator
            let selene_sim = py.import("selene_sim")
                .map_err(|e| PecosError::Processing(format!("Failed to import selene_sim: {}", e)))?;
            
            let simulator_class = selene_sim.getattr(&self.config.simulator_name)
                .map_err(|e| PecosError::Processing(format!("Failed to get {} simulator: {}", self.config.simulator_name, e)))?;
            
            let simulator = simulator_class.call0()
                .map_err(|e| PecosError::Processing(format!("Failed to create {} instance: {}", self.config.simulator_name, e)))?;
            
            // Call runner.run() with the simulator
            let run_result = runner.call_method1(py, "run", (simulator,))
                .and_then(|result| result.call_method1(py, "run", ()))
                .map_err(|e| PecosError::Processing(format!("Failed to run simulation: {}", e)))?;
            
            // Add n_qubits parameter
            let kwargs = PyDict::new(py);
            kwargs.set_item("n_qubits", self.config.num_qubits)
                .map_err(|e| PecosError::Processing(format!("Failed to set n_qubits: {}", e)))?;
            
            let run_result = runner.call_method(py, "run", (simulator,), Some(&kwargs))
                .map_err(|e| PecosError::Processing(format!("Failed to run simulation: {}", e)))?;
            
            // Convert the result to dict
            let result_dict = if let Ok(dict) = run_result.call_method0(py, "dict") {
                dict
            } else {
                // Try to convert to dict directly
                run_result.downcast::<PyDict>(py)
                    .map_err(|e| PecosError::Processing(format!("Result is not a dict: {}", e)))?
                    .as_ref()
            };
            
            // Convert Python dict to Rust BTreeMap
            let mut results = BTreeMap::new();
            for (key, value) in result_dict.iter() {
                let key_str = key.extract::<String>()
                    .map_err(|e| PecosError::Processing(format!("Failed to extract key: {}", e)))?;
                
                if let Ok(val) = value.extract::<u32>() {
                    results.insert(key_str, Data::U32(val));
                } else if let Ok(val) = value.extract::<i32>() {
                    results.insert(key_str, Data::U32(val as u32));
                } else if let Ok(val) = value.extract::<bool>() {
                    results.insert(key_str, Data::U32(if val { 1 } else { 0 }));
                }
            }
            
            log::info!("Shot executed successfully, got {} results", results.len());
            Ok(results)
        })
    }
}

// Implement ClassicalEngine
impl ClassicalEngine for SeleneNativeRunnerEngine {
    fn num_qubits(&self) -> usize {
        self.config.num_qubits
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        // Native runner doesn't use ByteMessage commands
        Ok(ByteMessage::create_empty())
    }
    
    fn handle_measurements(&mut self, _message: ByteMessage) -> Result<(), PecosError> {
        // Results are handled directly by the native runner
        Ok(())
    }
    
    fn get_results(&self) -> Result<Shot, PecosError> {
        let mut shot = Shot::default();
        shot.data = self.results.clone();
        Ok(shot)
    }
    
    fn compile(&self) -> Result<(), PecosError> {
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        self.results.clear();
        Ok(())
    }
}

// Implement ControlEngine
impl ControlEngine for SeleneNativeRunnerEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;
    
    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        log::info!("SeleneNativeRunnerEngine: start() called");
        
        // Initialize the native runner if not done already
        if self.runner.is_none() {
            self.initialize_native_runner()?;
        }
        
        // Execute the shot
        let results = self.execute_shot()?;
        self.results = results;
        
        // Return complete result immediately
        Ok(EngineStage::Complete(self.get_results()?))
    }
    
    fn continue_processing(&mut self, _measurements: ByteMessage)
        -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // Native runner completes in one step
        Ok(EngineStage::Complete(self.get_results()?))
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}

// Implement Engine
impl Engine for SeleneNativeRunnerEngine {
    type Input = ();
    type Output = Shot;
    
    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        if self.runner.is_none() {
            self.initialize_native_runner()?;
        }
        
        let results = self.execute_shot()?;
        self.results = results;
        
        self.get_results()
    }
    
    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}