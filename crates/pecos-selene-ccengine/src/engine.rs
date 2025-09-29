//! Selene Classical Control Engine
//!
//! A classical control engine that uses Selene runtime plugins for control flow
//! while generating PECOS ByteMessages for quantum operations.

use crate::bridge::SeleneRuntimeBridge;
use crate::runtime_plugin::RuntimePlugin;
use log::{debug, info};
use pecos_core::errors::PecosError;
use pecos_engines::byte_message::ByteMessage;
use pecos_engines::engine_system::{ClassicalEngine, ControlEngine, EngineStage};
use pecos_engines::shot_results::{Shot, Data};
use pecos_engines::Engine;
use pecos_qis_runtime::QisEngine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration for the Selene Classical Control Engine
#[derive(Debug, Clone)]
pub struct SeleneEngineConfig {
    /// Path to the runtime plugin (.so file)
    pub runtime_plugin_path: PathBuf,

    /// Number of qubits
    pub n_qubits: u64,

    /// Verbose output
    pub verbose: bool,
}

impl Default for SeleneEngineConfig {
    fn default() -> Self {
        Self {
            runtime_plugin_path: PathBuf::from("libselene_simple_runtime.so"),
            n_qubits: 20,
            verbose: false,
        }
    }
}

/// Classical control engine using Selene runtime plugins
pub struct SeleneClassicalControlEngine {
    /// Configuration
    config: SeleneEngineConfig,

    /// Runtime bridge
    bridge: Option<SeleneRuntimeBridge>,

    /// QIS Engine for running LLVM programs
    qis_engine: Option<QisEngine>,

    /// Shot counter
    shot_count: usize,

    /// Accumulated measurement results
    measurement_results: HashMap<usize, bool>,
}

impl SeleneClassicalControlEngine {
    /// Create a new Selene classical control engine
    pub fn new(config: SeleneEngineConfig) -> Result<Self, PecosError> {
        Ok(Self {
            config,
            bridge: None,
            qis_engine: None,
            shot_count: 0,
            measurement_results: HashMap::new(),
        })
    }

    /// Load a QIS program from LLVM IR file
    pub fn load_llvm_ir(&mut self, llvm_file: impl AsRef<Path>) -> Result<(), PecosError> {
        let engine = QisEngine::new(llvm_file.as_ref().to_path_buf());
        self.qis_engine = Some(engine);
        Ok(())
    }

    /// Initialize the runtime bridge
    fn init_bridge(&mut self) -> Result<(), PecosError> {
        if self.bridge.is_none() {
            info!("Loading runtime plugin from {:?}", self.config.runtime_plugin_path);

            let plugin = RuntimePlugin::load(&self.config.runtime_plugin_path)
                .map_err(|e| PecosError::Processing(format!("Failed to load runtime plugin: {}", e)))?;

            let bridge = SeleneRuntimeBridge::new(plugin, self.config.n_qubits)
                .map_err(|e| PecosError::Processing(format!("Failed to create runtime bridge: {}", e)))?;

            self.bridge = Some(bridge);
        }
        Ok(())
    }


    /// Run the QIS program to queue operations in the runtime
    fn run_qis_program(&mut self) -> Result<(), PecosError> {
        if let Some(ref mut qis_engine) = self.qis_engine {
            // For now, just generate commands from the QIS engine
            // TODO: We need to modify pecos-qis-runtime to forward to selene_runtime_* functions
            let _commands = qis_engine.generate_commands()?;
            Ok(())
        } else {
            Err(PecosError::Processing("No QIS engine loaded".to_string()))
        }
    }

    /// Get the next batch of operations from the runtime
    fn get_next_operations(&mut self) -> Result<Option<ByteMessage>, PecosError> {
        if let Some(ref mut bridge) = self.bridge {
            let has_ops = bridge.get_next_operations()
                .map_err(|e| PecosError::Processing(format!("Failed to get operations: {}", e)))?;

            if has_ops {
                let message = bridge.get_byte_message();
                if message.is_empty()? {
                    return Ok(None);
                } else {
                    debug!("Retrieved ByteMessage with operations");
                    return Ok(Some(message));
                }
            }
        }
        Ok(None)
    }

    /// Process measurement results from the quantum engine
    fn process_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        // Extract measurement outcomes from the message
        let outcomes = message.outcomes()?;

        if let Some(ref mut bridge) = self.bridge {
            bridge.process_measurement_results(outcomes.clone());

            // Also accumulate in our own results
            for (idx, &outcome) in outcomes.iter().enumerate() {
                self.measurement_results.insert(idx, outcome != 0);
            }
        }

        Ok(())
    }
}

impl ClassicalEngine for SeleneClassicalControlEngine {
    fn num_qubits(&self) -> usize {
        self.config.n_qubits as usize
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        // Initialize bridge if needed
        self.init_bridge()?;

        // Start a new shot
        if let Some(ref bridge) = self.bridge {
            bridge.shot_start(self.shot_count as u64, self.shot_count as u64)?;
        }

        // Run the QIS program (queues operations in runtime)
        self.run_qis_program()?;

        // Get operations from the runtime
        match self.get_next_operations()? {
            Some(message) => Ok(message),
            None => Ok(ByteMessage::builder().build()),
        }
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        self.process_measurements(message)
    }

    fn get_results(&self) -> Result<Shot, PecosError> {
        // Convert measurement results to Shot format
        let mut shot = Shot::default();

        // For now, just return the raw measurement results
        // In a full implementation, we'd map these to register names
        for (idx, &value) in self.measurement_results.iter() {
            shot.data.insert(
                format!("m{}", idx),
                if value { Data::U8(1) } else { Data::U8(0) },
            );
        }

        Ok(shot)
    }

    fn set_seed(&mut self, _seed: u64) -> Result<(), PecosError> {
        // TODO: Pass seed to runtime plugin
        Ok(())
    }

    fn compile(&self) -> Result<(), PecosError> {
        // TODO: Compile the QIS program if needed
        Ok(())
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.shot_count = 0;
        self.measurement_results.clear();

        if let Some(ref mut bridge) = self.bridge {
            bridge.reset();
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Clone for SeleneClassicalControlEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            bridge: None, // Can't clone the bridge easily, will need to reinitialize
            qis_engine: self.qis_engine.clone(),
            shot_count: self.shot_count,
            measurement_results: self.measurement_results.clone(),
        }
    }
}

impl ControlEngine for SeleneClassicalControlEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // Initialize and start execution
        self.init_bridge()?;

        // Start shot in runtime
        if let Some(ref bridge) = self.bridge {
            bridge.shot_start(self.shot_count as u64, self.shot_count as u64)
                .map_err(|e| PecosError::Processing(format!("Failed to start shot: {}", e)))?;
        }

        // Run the QIS program
        self.run_qis_program()?;

        // Get first batch of operations
        match self.get_next_operations()? {
            Some(message) => Ok(EngineStage::NeedsProcessing(message)),
            None => Ok(EngineStage::Complete(self.get_results()?)),
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        // Process measurement results
        self.process_measurements(measurements)?;

        // Get next batch of operations
        match self.get_next_operations()? {
            Some(message) => Ok(EngineStage::NeedsProcessing(message)),
            None => {
                // End the shot
                if let Some(ref bridge) = self.bridge {
                    bridge.shot_end(self.shot_count as u64, self.shot_count as u64)
                        .map_err(|e| PecosError::Processing(format!("Failed to end shot: {}", e)))?;
                }
                Ok(EngineStage::Complete(self.get_results()?))
            }
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.shot_count = 0;
        self.measurement_results.clear();
        if let Some(ref mut bridge) = self.bridge {
            bridge.reset();
        }
        Ok(())
    }
}

impl Engine for SeleneClassicalControlEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, PecosError> {
        // Use the EngineStage pattern
        let mut stage = self.start(input)?;

        while let EngineStage::NeedsProcessing(_commands) = stage {
            // In a real scenario, commands would be sent to quantum engine
            // For testing, just return empty measurements
            let measurements = ByteMessage::builder().build();
            stage = self.continue_processing(measurements)?;
        }

        match stage {
            EngineStage::Complete(output) => Ok(output),
            EngineStage::NeedsProcessing(_) => unreachable!(),
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.shot_count = 0;
        self.measurement_results.clear();

        if let Some(ref mut bridge) = self.bridge {
            bridge.reset();
        }
        Ok(())
    }
}