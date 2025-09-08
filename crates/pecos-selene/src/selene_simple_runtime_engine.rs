//! SeleneSimpleRuntimeEngine - A ClassicalControlEngine that uses actual Selene Simple Runtime
//!
//! This engine loads the actual Selene Simple Runtime plugin and uses it exactly like SeleneEngine does.
//! Instead of reimplementing the plugin system, we leverage Selene's existing RuntimeInterface
//! and convert the operations to PECOS ByteMessages.

use libloading::Library;
use pecos_core::prelude::PecosError;
use pecos_engines::{
    ByteMessage, ByteMessageBuilder, ClassicalEngine, ControlEngine, Data, Engine, EngineStage,
    Shot,
};
use pecos_programs::SeleneInterfaceProgram;
use std::{any::Any, collections::BTreeMap, io::Write, path::PathBuf};

// Selene runtime integration
use crate::SeleneError;
use selene_core::runtime::{
    Operation, RuntimeInterface, RuntimeInterfaceFactory, plugin::RuntimePluginInterface,
};
use selene_core::time::Instant as SeleneInstant;

/// A ClassicalControlEngine that uses Selene's Simple Runtime plugin

pub struct SeleneSimpleRuntimeEngine {
    /// Path to the Selene Simple Runtime plugin library (libselene_simple_runtime.so)
    runtime_library_path: PathBuf,

    /// Loaded Runtime Plugin Interface (factory for creating runtime instances)
    plugin_interface: Option<std::sync::Arc<RuntimePluginInterface>>,

    /// Active runtime instance (created from plugin_interface)
    runtime: Option<Box<dyn RuntimeInterface>>,

    /// The loaded program (Selene Interface plugin)
    program: Option<SeleneInterfaceProgram>,

    /// Loaded Interface Plugin library
    interface_library: Option<Library>,

    /// Number of qubits
    num_qubits: usize,

    /// Current measurement results
    measurement_results: BTreeMap<String, Data>,

    /// Shot counter for unique seeding
    shot_count: u64,

    /// Reusable message builder for generating commands
    message_builder: ByteMessageBuilder,
}

impl SeleneSimpleRuntimeEngine {
    /// Create a new engine with the Selene Simple Runtime plugin
    pub fn new(runtime_library_path: PathBuf, num_qubits: usize) -> Result<Self, PecosError> {
        println!("[DEBUG] *** SELENE ENGINE CONSTRUCTOR - LOADING FFI STUBS ***");
        std::io::stdout().flush().unwrap();
        // Load FFI stub library automatically at construction time
        match Self::load_ffi_stubs() {
            Ok(()) => {
                println!("[DEBUG] *** FFI STUBS LOADED SUCCESSFULLY ***");
                std::io::stdout().flush().unwrap();
            }
            Err(e) => {
                println!("[DEBUG] *** FFI STUBS LOADING FAILED: {} ***", e);
                println!("[DEBUG] *** CONTINUING WITHOUT STUBS (WILL LIKELY HANG) ***");
                std::io::stdout().flush().unwrap();
            }
        }
        Ok(Self {
            runtime_library_path,
            plugin_interface: None,
            runtime: None,
            program: None,
            interface_library: None,
            num_qubits,
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            message_builder: ByteMessageBuilder::new(),
        })
    }

    /// Set the program to execute
    pub fn with_program(mut self, program: SeleneInterfaceProgram) -> Self {
        self.program = Some(program);
        self
    }

    /// Load the Selene Simple Runtime plugin and create runtime instance (following SeleneEngine pattern)
    fn ensure_runtime_loaded(&mut self) -> Result<(), PecosError> {
        println!("[DEBUG] ensure_runtime_loaded started");

        if self.plugin_interface.is_none() {
            println!("[DEBUG] Loading plugin interface");
            self.load_plugin_interface()?;
            println!("[DEBUG] Plugin interface loaded");
        }

        if self.runtime.is_none() {
            println!("[DEBUG] Creating runtime instance");
            self.create_runtime_instance()?;
            println!("[DEBUG] Runtime instance created");
        }

        if self.interface_library.is_none() && self.program.is_some() {
            println!("[DEBUG] Loading interface plugin");
            self.load_interface_plugin()?;
            println!("[DEBUG] Interface plugin loaded");
        }

        println!("[DEBUG] ensure_runtime_loaded completed successfully");
        Ok(())
    }

    /// Load the Selene Simple Runtime plugin as a RuntimeInterfaceFactory
    fn load_plugin_interface(&mut self) -> Result<(), PecosError> {
        log::info!(
            "Loading Selene Simple Runtime plugin from {:?}",
            self.runtime_library_path
        );

        let plugin_interface = RuntimePluginInterface::new_from_file(&self.runtime_library_path)
            .map_err(|e| {
                SeleneError::RuntimeError(format!("Failed to load runtime plugin: {}", e))
            })?;

        self.plugin_interface = Some(plugin_interface);
        log::info!("Successfully loaded Selene Simple Runtime plugin");
        Ok(())
    }

    /// Create a runtime instance from the loaded plugin (following SeleneEngine pattern)
    fn create_runtime_instance(&mut self) -> Result<(), PecosError> {
        let plugin_interface = self.plugin_interface.as_ref().ok_or_else(|| {
            SeleneError::CompilationError("No plugin interface available".to_string())
        })?;

        // Create runtime instance with current time and no arguments (like SeleneEngine)
        let start_time = SeleneInstant::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        );
        let args: Vec<String> = vec![];
        let runtime = plugin_interface
            .clone()
            .init(self.num_qubits as u64, start_time, &args)
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to init runtime: {}", e)))?;

        self.runtime = Some(runtime);
        log::info!("Created Selene runtime instance from plugin");
        Ok(())
    }

    /// Load the Interface Plugin from SeleneInterfaceProgram bytes
    fn load_interface_plugin(&mut self) -> Result<(), PecosError> {
        let program = self.program.as_ref().ok_or_else(|| {
            SeleneError::CompilationError("No Interface Plugin program available".to_string())
        })?;

        println!(
            "[SeleneSimpleRuntimeEngine] Loading Interface Plugin ({} bytes)",
            program.plugin.len()
        );
        log::info!("Loading Interface Plugin ({} bytes)", program.plugin.len());

        // Write plugin bytes to a temporary file and load as shared library
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to create temp file: {}", e)))?;

        temp_file.write_all(&program.plugin).map_err(|e| {
            SeleneError::RuntimeError(format!("Failed to write plugin bytes: {}", e))
        })?;

        let temp_path = temp_file.path();

        // Load the shared library
        let library = unsafe {
            Library::new(temp_path).map_err(|e| {
                SeleneError::RuntimeError(format!("Failed to load Interface Plugin library: {}", e))
            })?
        };

        self.interface_library = Some(library);
        log::info!("Successfully loaded Interface Plugin library");
        Ok(())
    }

    /// Execute the Interface Plugin's qmain function to generate operations
    fn execute_interface_plugin(&mut self) -> Result<(), PecosError> {
        println!("[DEBUG] *** EXECUTE_INTERFACE_PLUGIN CALLED ***");
        std::io::stdout().flush().unwrap();

        // FFI stub library is loaded at engine construction time

        let library = self.interface_library.as_ref().ok_or_else(|| {
            SeleneError::RuntimeError("No Interface Plugin library loaded".to_string())
        })?;

        println!("[DEBUG] Interface Plugin library found, proceeding with qmain execution");

        // Get the qmain function from the loaded library
        let qmain_symbol: libloading::Symbol<unsafe extern "C" fn(u64) -> u64> = unsafe {
            library.get(b"qmain").map_err(|e| {
                SeleneError::RuntimeError(format!("Failed to find qmain function: {}", e))
            })?
        };

        println!("[DEBUG] About to call qmain(0) with FFI stubs loaded");

        // Call qmain with parameter 0 (following Selene convention)
        // The qmain function will call our stub functions
        let result = unsafe { qmain_symbol(0) };

        println!("[DEBUG] Interface Plugin qmain returned: {}", result);

        Ok(())

        // The original code that hangs:
        /*
        let library = self.interface_library.as_ref()
            .ok_or_else(|| SeleneError::RuntimeError("No Interface Plugin library loaded".to_string()))?;

        println!("[DEBUG] Interface Plugin library found, proceeding with qmain execution");
        log::debug!("Executing Interface Plugin qmain function");

        // Preload our C stub library to provide FFI functions
        let stub_lib_path = std::env::current_dir()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to get current dir: {}", e)))?
            .join("libselene_stubs.so");

        if stub_lib_path.exists() {
            println!("[DEBUG] Loading stub library: {:?}", stub_lib_path);

            // Use dlopen with RTLD_GLOBAL to ensure symbols are globally available
            use std::ffi::CString;
            let path_cstr = CString::new(stub_lib_path.to_string_lossy().as_ref())
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to create CString: {}", e)))?;

            unsafe {
                let handle = libc::dlopen(path_cstr.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
                if handle.is_null() {
                    let error = std::ffi::CStr::from_ptr(libc::dlerror()).to_string_lossy();
                    return Err(SeleneError::RuntimeError(format!("Failed to dlopen stub library: {}", error)).into());
                }
                println!("[DEBUG] Stub library loaded with RTLD_GLOBAL");

                // Verify that selene_qalloc is available
                let symbol_name = CString::new("selene_qalloc").unwrap();
                let symbol = libc::dlsym(handle, symbol_name.as_ptr());
                if symbol.is_null() {
                    println!("[DEBUG] WARNING: selene_qalloc symbol not found after loading");
                } else {
                    println!("[DEBUG] SUCCESS: selene_qalloc symbol found at {:?}", symbol);
                }

                // Don't close the handle - keep it alive
            }
        } else {
            println!("[DEBUG] Stub library not found at {:?}", stub_lib_path);
        }

        // Get the qmain function from the loaded library
        let qmain_symbol: Symbol<unsafe extern "C" fn(u64) -> u64> = unsafe {
            library.get(b"qmain")
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to find qmain function: {}", e)))?
        };

        // Call qmain with parameter 0 (following Selene convention)
        // The qmain function will call our stub functions
        let result = unsafe { qmain_symbol(0) };

        log::debug!("Interface Plugin qmain returned: {}", result);

        Ok(())
        */
    }

    /// Load FFI stub library with RTLD_GLOBAL to make symbols available to Interface Plugins
    fn load_ffi_stubs() -> Result<(), PecosError> {
        // Try multiple locations for the stub library
        let stub_lib_paths = vec![
            // In target directory (from build script)
            std::env::current_dir()
                .unwrap_or_default()
                .join("target")
                .join("debug")
                .join("libselene_correct_stubs.so"),
            // Current working directory
            std::env::current_dir()
                .unwrap_or_default()
                .join("libselene_correct_stubs.so"),
            // Next to this crate
            std::env::current_dir()
                .unwrap_or_default()
                .join("crates")
                .join("pecos-selene")
                .join("libselene_correct_stubs.so"),
        ];

        for stub_path in &stub_lib_paths {
            if stub_path.exists() {
                println!("[DEBUG] Loading FFI stub library from: {:?}", stub_path);

                unsafe {
                    let path_cstr = std::ffi::CString::new(stub_path.to_string_lossy().as_ref())
                        .map_err(|e| {
                            SeleneError::RuntimeError(format!("Failed to create CString: {}", e))
                        })?;

                    let handle =
                        libc::dlopen(path_cstr.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
                    if handle.is_null() {
                        let error = std::ffi::CStr::from_ptr(libc::dlerror()).to_string_lossy();
                        println!("[DEBUG] Failed to load {:?}: {}", stub_path, error);
                        continue; // Try next path
                    }

                    println!("[DEBUG] Successfully loaded FFI stub library with RTLD_GLOBAL");
                    return Ok(());
                }
            }
        }

        Err(SeleneError::RuntimeError(
            "Could not find or load libselene_correct_stubs.so. Please run 'cargo build' to generate it.".to_string()
        ).into())
    }

    /// Get operations from the Selene runtime and convert to ByteMessage (following SeleneEngine pattern)
    fn get_operations_from_selene_runtime(&mut self) -> Result<ByteMessage, PecosError> {
        println!("[SeleneSimpleRuntimeEngine] get_operations_from_selene_runtime called");
        println!("[DEBUG] About to call ensure_runtime_loaded");
        self.ensure_runtime_loaded()?;
        println!("[DEBUG] ensure_runtime_loaded completed");

        // Execute Interface Plugin to generate operations (if we have a program and this is called for the first time in a shot)
        println!(
            "[SeleneSimpleRuntimeEngine] program.is_some()={}, shot_count={}",
            self.program.is_some(),
            self.shot_count
        );
        eprintln!("[BEFORE-IF] Before checking if program exists");
        if self.program.is_some() {
            eprintln!("[INSIDE-IF] Inside if block - about to print Executing message");
            println!(
                "[SeleneSimpleRuntimeEngine] Executing Interface Plugin for shot {}",
                self.shot_count
            );
            eprintln!("[BEFORE-CALL] About to call execute_interface_plugin");
            self.execute_interface_plugin()?;
            eprintln!("[AFTER-CALL] execute_interface_plugin completed successfully");
        } else {
            eprintln!("[NO-PROGRAM] No program to execute");
        }

        // Get runtime instance
        let runtime = self.runtime.as_mut().ok_or_else(|| {
            SeleneError::RuntimeError("No runtime instance available".to_string())
        })?;

        // Get operations from the actual Selene runtime (just like SeleneEngine does)
        match runtime
            .get_next_operations()
            .map_err(|e| SeleneError::RuntimeError(format!("Failed to get operations: {}", e)))?
        {
            Some(batch) => {
                let operations = batch.iter_ops().cloned().collect::<Vec<_>>();
                log::debug!(
                    "Retrieved {} operations from Selene runtime",
                    operations.len()
                );

                // Convert Selene operations to PECOS ByteMessage
                self.convert_selene_operations_to_pecos(&operations)
            }
            None => {
                log::debug!("No more operations from Selene runtime");
                Ok(ByteMessage::create_empty())
            }
        }
    }

    /// Convert Selene Operations to PECOS ByteMessage (adapted from SeleneEngine)
    fn convert_selene_operations_to_pecos(
        &mut self,
        operations: &[Operation],
    ) -> Result<ByteMessage, PecosError> {
        if operations.is_empty() {
            return Ok(ByteMessage::create_empty());
        }

        // Reset and initialize message builder for quantum operations
        self.message_builder.reset();
        let _ = self.message_builder.for_quantum_operations();

        // Convert each Selene operation to PECOS operations
        for operation in operations {
            match operation {
                Operation::RXYGate {
                    qubit_id,
                    theta,
                    phi,
                } => {
                    log::debug!(
                        "Converting RXY gate: qubit={}, theta={}, phi={}",
                        qubit_id,
                        theta,
                        phi
                    );
                    // Use R1XY for RXY gates (closest PECOS equivalent)
                    self.message_builder
                        .add_r1xy(*theta, *phi, &[*qubit_id as usize]);
                }
                Operation::RZGate { qubit_id, theta } => {
                    log::debug!("Converting RZ gate: qubit={}, theta={}", qubit_id, theta);
                    self.message_builder.add_rz(*theta, &[*qubit_id as usize]);
                }
                Operation::RZZGate {
                    qubit_id_1,
                    qubit_id_2,
                    theta,
                } => {
                    log::debug!(
                        "Converting RZZ gate: qubit1={}, qubit2={}, theta={}",
                        qubit_id_1,
                        qubit_id_2,
                        theta
                    );
                    self.message_builder.add_rzz(
                        *theta,
                        &[*qubit_id_1 as usize],
                        &[*qubit_id_2 as usize],
                    );
                }
                Operation::Measure {
                    qubit_id,
                    result_id,
                } => {
                    log::debug!(
                        "Converting Measure: qubit={}, result_id={}",
                        qubit_id,
                        result_id
                    );
                    self.message_builder.add_measurements(&[*qubit_id as usize]);
                }
                Operation::Reset { qubit_id } => {
                    log::debug!("Converting Reset: qubit={}", qubit_id);
                    // Reset is implemented as preparation (closest equivalent)
                    self.message_builder.add_prep(&[*qubit_id as usize]);
                }
                Operation::Custom { .. } => {
                    log::warn!("Custom operation encountered - not currently supported, skipping");
                    // Skip custom operations for now
                }
                Operation::MeasureLeaked {
                    qubit_id,
                    result_id,
                } => {
                    log::debug!(
                        "Converting MeasureLeaked: qubit={}, result_id={}",
                        qubit_id,
                        result_id
                    );
                    // MeasureLeaked in PECOS - measures if qubit is in leaked state
                    self.message_builder
                        .add_measure_leakages(&[*qubit_id as usize]);
                }
            }
        }

        // Build and return the ByteMessage
        let message = self.message_builder.build();
        log::debug!(
            "Converted {} Selene operations to PECOS ByteMessage",
            operations.len()
        );
        Ok(message)
    }
}

// Implement Engine trait
impl Engine for SeleneSimpleRuntimeEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        self.ensure_runtime_loaded()?;

        // Get runtime and execute all operations until complete
        loop {
            let commands = self.get_operations_from_selene_runtime()?;
            if commands.is_empty()? {
                break;
            }
            // In a real system, these would be sent to a quantum engine
            // For now, we simulate empty measurements
            let measurements = ByteMessage::builder().for_outcomes().build();
            self.handle_measurements(measurements)?;
        }

        self.get_results()
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ControlEngine>::reset(self)
    }
}

// Implement ClassicalEngine trait
impl ClassicalEngine for SeleneSimpleRuntimeEngine {
    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        eprintln!("[GENERATE_COMMANDS] Called - about to call get_operations_from_selene_runtime");
        let result = self.get_operations_from_selene_runtime();
        eprintln!(
            "[GENERATE_COMMANDS] get_operations_from_selene_runtime returned: {:?}",
            result.as_ref().map(|_| "Ok").unwrap_or("Err")
        );
        result
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        // Extract measurements and store them
        let outcomes = message
            .outcomes()
            .map_err(|e| PecosError::Processing(format!("Failed to extract outcomes: {}", e)))?;

        log::debug!("Processing {} measurement outcomes", outcomes.len());

        for (i, value) in outcomes.iter().enumerate() {
            let result_key = format!("measurement_{}", i + 1);
            self.measurement_results
                .insert(result_key, Data::U32(*value));
        }

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
        log::debug!("Resetting SeleneSimpleRuntimeEngine for next shot");

        // Reset shot-specific state
        self.measurement_results.clear();
        self.shot_count += 1;

        // Call runtime shot_end and shot_start if runtime exists
        if let Some(runtime) = self.runtime.as_mut() {
            runtime
                .shot_end()
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to end shot: {}", e)))?;
            runtime
                .shot_start(self.shot_count, 42 + self.shot_count)
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to start shot: {}", e)))?;
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Implement ControlEngine trait
impl ControlEngine for SeleneSimpleRuntimeEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        self.ensure_runtime_loaded()?;

        // Start shot on runtime
        if let Some(runtime) = self.runtime.as_mut() {
            runtime
                .shot_start(self.shot_count, 42 + self.shot_count)
                .map_err(|e| SeleneError::RuntimeError(format!("Failed to start shot: {}", e)))?;
        }

        // Generate initial commands
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

// Implement Clone for thread/worker isolation
impl Clone for SeleneSimpleRuntimeEngine {
    fn clone(&self) -> Self {
        // Create fully isolated instances for proper thread/worker isolation
        Self {
            runtime_library_path: self.runtime_library_path.clone(),
            plugin_interface: None, // Each clone loads its own plugin
            runtime: None,          // Each clone creates its own runtime instance
            program: self.program.clone(),
            interface_library: None, // Each clone loads its own interface plugin
            num_qubits: self.num_qubits,
            measurement_results: BTreeMap::new(),
            shot_count: 0,
            message_builder: ByteMessageBuilder::new(),
        }
    }
}

// Implement Send and Sync for threading
unsafe impl Send for SeleneSimpleRuntimeEngine {}
unsafe impl Sync for SeleneSimpleRuntimeEngine {}
