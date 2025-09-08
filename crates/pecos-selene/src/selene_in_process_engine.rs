//! In-process Selene execution engine
//!
//! This engine loads and executes Selene Interface Plugins in-process,
//! allowing the `PecosSeleneBridgeSimulator` to communicate directly via `EngineInterface`.

use crate::SeleneError;
use pecos_core::prelude::PecosError;
use pecos_engines::{
    ByteMessage, ByteMessageBuilder, ClassicalEngine, ControlEngine, Data, Engine, EngineStage,
    Shot,
};
use pecos_programs::SeleneInterfaceProgram;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::{any::Any, collections::BTreeMap};

// Import the bridge interface
use pecos_selene_bridge::{EngineInterface, initialize_engine_interface};

/// Configuration for in-process Selene execution
#[derive(Clone)]
pub struct SeleneInProcessConfig {
    pub num_qubits: usize,
    pub verbose: bool,
}

impl Default for SeleneInProcessConfig {
    fn default() -> Self {
        Self {
            num_qubits: 10,
            verbose: false,
        }
    }
}

/// In-process Selene engine that executes Interface Plugins directly
pub struct SeleneInProcessEngine {
    config: SeleneInProcessConfig,
    program: Option<SeleneInterfaceProgram>,
    operation_queue: Vec<ByteMessage>,
    measurement_results: BTreeMap<String, Data>,
    shot_count: u64,
    _message_builder: ByteMessageBuilder,
    // Dynamic library and entry point
    _plugin_lib: Option<libloading::Library>,
    entry_point: Option<libloading::Symbol<'static, unsafe extern "C" fn()>>,
}

impl SeleneInProcessEngine {
    pub fn new(num_qubits: usize) -> Result<Self, PecosError> {
        Ok(Self {
            config: SeleneInProcessConfig {
                num_qubits,
                verbose: false,
            },
            program: None,
            operation_queue: Vec::new(),
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            _message_builder: ByteMessageBuilder::new(),
            _plugin_lib: None,
            entry_point: None,
        })
    }

    #[must_use]
    pub fn with_program(mut self, program: SeleneInterfaceProgram) -> Self {
        self.program = Some(program);
        self
    }

    /// Load the Interface Plugin and prepare for execution
    fn load_interface_plugin(&mut self) -> Result<(), PecosError> {
        let program = self
            .program
            .as_ref()
            .ok_or(SeleneError::NoProgramSpecified)?;

        log::info!("Loading Interface Plugin ({} bytes)", program.plugin.len());

        // Write the plugin to a temporary file
        let temp_dir = tempfile::tempdir()
            .map_err(|e| PecosError::Processing(format!("Failed to create temp dir: {e}")))?;

        let plugin_obj_path = temp_dir.path().join("plugin.o");
        let plugin_so_path = temp_dir.path().join("plugin.so");

        // Write the .o file
        std::fs::write(&plugin_obj_path, &program.plugin)
            .map_err(|e| PecosError::Processing(format!("Failed to write plugin: {e}")))?;

        // Find the Selene runtime library
        let selene_lib_path = std::env::var("SELENE_LIB_PATH").unwrap_or_else(|_| {
            // Try to find it in the Python package
            let python_site = std::process::Command::new("python")
                .args([
                    "-c",
                    "import selene_sim; import os; print(os.path.dirname(selene_sim.__file__))",
                ])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if python_site.is_empty() {
                String::from("/usr/local/lib")
            } else {
                format!("{python_site}/_dist/lib")
            }
        });

        // Convert .o to .so using gcc, linking with Selene runtime
        let output = Command::new("gcc")
            .args(["-shared", "-o"])
            .arg(&plugin_so_path)
            .arg(&plugin_obj_path)
            .arg(format!("-L{selene_lib_path}"))
            .arg("-lselene")
            .arg("-Wl,-rpath")
            .arg(&selene_lib_path)
            .output()
            .map_err(|e| PecosError::Processing(format!("Failed to run gcc: {e}")))?;

        if !output.status.success() {
            return Err(PecosError::Processing(format!(
                "Failed to create shared library: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        log::info!("Created shared library at {plugin_so_path:?}");

        // Initialize the engine interface so the bridge can communicate with us
        initialize_engine_interface(Arc::new(Mutex::new(self.clone())));

        // Load the shared library
        unsafe {
            let lib = libloading::Library::new(&plugin_so_path)
                .map_err(|e| PecosError::Processing(format!("Failed to load plugin: {e}")))?;

            // We need to leak the library to get a 'static lifetime for the symbol
            let lib_boxed = Box::new(lib);
            let lib_static = Box::leak(lib_boxed);

            // Find the qmain entry point
            let entry: libloading::Symbol<unsafe extern "C" fn()> = lib_static
                .get(b"qmain")
                .map_err(|e| PecosError::Processing(format!("Failed to find qmain: {e}")))?;

            log::info!("Found qmain entry point");

            // Store the entry point with static lifetime
            let entry_static: libloading::Symbol<'static, unsafe extern "C" fn()> =
                std::mem::transmute(entry);

            self.entry_point = Some(entry_static);
        }

        // Keep the temp directory alive
        std::mem::forget(temp_dir);

        Ok(())
    }

    /// Generate test operations to verify the flow
    fn generate_test_operations(&mut self) {
        // Create a simple quantum circuit
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // H gate on qubit 0
        builder.add_h(&[0]);

        // Measurement on qubit 0
        builder.add_measurements(&[0]);

        let message = builder.build();
        self.operation_queue.push(message);

        log::info!("Generated test quantum operations");
    }

    /// Execute the Interface Plugin
    fn execute_plugin(&mut self) -> Result<(), PecosError> {
        log::info!("Executing Interface Plugin for shot {}", self.shot_count);

        if let Some(entry) = &self.entry_point {
            log::info!("Calling qmain entry point");
            unsafe {
                // Call the plugin's qmain function
                // This should trigger calls through the bridge simulator
                entry();
            }
            log::info!("qmain execution completed");
        } else {
            log::warn!("No entry point loaded, generating test operations");
            // Fallback to test operations
            self.generate_test_operations();
        }

        self.shot_count += 1;
        Ok(())
    }
}

// Implement ClassicalEngine
impl ClassicalEngine for SeleneInProcessEngine {
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
        if let Some(message) = self.operation_queue.pop() {
            log::debug!("Returning queued operations");
            Ok(message)
        } else {
            Ok(ByteMessage::create_empty())
        }
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        let outcomes = message
            .outcomes()
            .map_err(|e| PecosError::Processing(format!("Failed to extract outcomes: {e}")))?;

        for (i, value) in outcomes.iter().enumerate() {
            let result_key = format!("measurement_{i}");
            self.measurement_results
                .insert(result_key, Data::U32(*value));
        }

        log::debug!("Stored {} measurement results", outcomes.len());
        Ok(())
    }

    fn get_results(&self) -> Result<Shot, PecosError> {
        let mut shot = Shot::default();
        shot.data = self.measurement_results.clone();
        Ok(shot)
    }

    fn compile(&self) -> Result<(), PecosError> {
        Ok(())
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.operation_queue.clear();
        self.measurement_results.clear();
        Ok(())
    }
}

// Implement ControlEngine
impl ControlEngine for SeleneInProcessEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        log::info!("SeleneInProcessEngine: start() called");

        // Load the Interface Plugin
        self.load_interface_plugin()?;

        // Execute it
        self.execute_plugin()?;

        // Get initial commands
        let commands = self.generate_commands()?;

        if commands.is_empty()? {
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        self.handle_measurements(measurements)?;

        let commands = self.generate_commands()?;

        if commands.is_empty()? {
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}

// Implement Engine
impl Engine for SeleneInProcessEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        self.load_interface_plugin()?;
        self.execute_plugin()?;

        // Process all operations
        while !self.operation_queue.is_empty() {
            let _ops = self.generate_commands()?;
            // In a real system, these would be sent to the quantum engine
        }

        self.get_results()
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}

// Implement Clone for thread safety
impl Clone for SeleneInProcessEngine {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            program: self.program.clone(),
            operation_queue: Vec::new(),
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            _message_builder: ByteMessageBuilder::new(),
            _plugin_lib: None,
            entry_point: None,
        }
    }
}

// Implement EngineInterface for bridge communication
impl EngineInterface for SeleneInProcessEngine {
    fn send_operation(&mut self, message: ByteMessage) -> Result<(), anyhow::Error> {
        log::debug!("Bridge simulator sending operation to engine");
        self.operation_queue.push(message);
        Ok(())
    }

    fn receive_measurements(&mut self) -> Result<ByteMessage, anyhow::Error> {
        // Return measurement results
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_outcomes();

        // Add some test results
        builder.add_outcomes(&[0]);

        Ok(builder.build())
    }

    fn get_named_results(&mut self) -> Result<BTreeMap<String, bool>, anyhow::Error> {
        // Results are now handled by the LLVM runtime registry
        Ok(BTreeMap::new())
    }
}
