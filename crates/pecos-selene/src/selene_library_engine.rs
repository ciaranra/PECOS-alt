use libloading::{Library, Symbol};
/// Production implementation of `SeleneExecutableEngine` using library loading and callbacks
///
/// This engine loads Selene-compiled shared libraries and communicates via callbacks
/// for `ByteMessage` exchange while using TCP streams for final results.
use pecos_core::prelude::PecosError;
use pecos_engines::{ByteMessage, ClassicalEngine, ControlEngine, Data, Engine, EngineStage, Shot};
use std::collections::{BTreeMap, VecDeque};
use std::ffi::c_void;
use std::io::Read;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Import callback interface
use pecos_selene_bridge::callback_interface::pecos_reset_callback_state;

/// Shared state for callback communication
struct CallbackState {
    /// Queue of operations from Bridge simulator
    operation_queue: VecDeque<ByteMessage>,
    /// Queue of measurements to provide to Bridge
    measurement_queue: VecDeque<ByteMessage>,
    /// Indicates if execution is complete
    is_complete: bool,
    /// Indicates if Bridge is waiting for measurements
    is_waiting: bool,
}

/// Production Selene engine using library loading and callbacks
pub struct SeleneLibraryEngine {
    /// Path to the Selene-compiled shared library
    library_path: std::path::PathBuf,

    /// Loaded library handle
    library: Option<Library>,

    /// Number of qubits
    num_qubits: usize,

    /// Shared callback state
    callback_state: Arc<Mutex<CallbackState>>,

    /// Thread handle for quantum program execution
    execution_thread: Option<thread::JoinHandle<Result<(), PecosError>>>,

    /// Final results collected from execution
    final_results: BTreeMap<String, Data>,

    /// TCP result stream for capturing outputs
    result_stream: Option<ResultStreamCapture>,
}

/// Captures results from Selene's TCP stream
struct ResultStreamCapture {
    listener: TcpListener,
    port: u16,
    result_thread: Option<thread::JoinHandle<Vec<(String, Data)>>>,
}

impl ResultStreamCapture {
    /// Create a new TCP listener for capturing results
    fn new() -> Result<Self, PecosError> {
        // Bind to any available port on localhost
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| PecosError::Processing(format!("Failed to create TCP listener: {e}")))?;

        let port = listener
            .local_addr()
            .map_err(|e| PecosError::Processing(format!("Failed to get listener address: {e}")))?
            .port();

        log::info!("Created TCP result stream listener on port {port}");

        Ok(Self {
            listener,
            port,
            result_thread: None,
        })
    }

    /// Get the URI for Selene to connect to
    fn get_uri(&self) -> String {
        format!("tcp://127.0.0.1:{}", self.port)
    }

    /// Start listening for results in a separate thread
    fn start_capture(&mut self) -> Result<(), PecosError> {
        let listener = self
            .listener
            .try_clone()
            .map_err(|e| PecosError::Processing(format!("Failed to clone listener: {e}")))?;

        self.result_thread = Some(thread::spawn(move || {
            let mut results = Vec::new();

            // Accept connection from Selene
            if let Ok((mut stream, _)) = listener.accept() {
                log::info!("Accepted connection from Selene");

                let mut buffer = Vec::new();

                // Read all data from the stream
                if let Err(e) = stream.read_to_end(&mut buffer) {
                    log::error!("Failed to read from result stream: {e}");
                    return results;
                }

                // Parse the results
                // Format is typically: "TAG:TYPE:name=value\n"
                let data = String::from_utf8_lossy(&buffer);
                for line in data.lines() {
                    if let Some((_tag, rest)) = line.split_once(':')
                        && let Some((typ, value_str)) = rest.split_once(':')
                        && let Some((name, value)) = value_str.split_once('=')
                    {
                        // Parse based on type
                        let parsed_value = match typ {
                            "BOOL" => value.parse::<bool>().ok().map(Data::Bool),
                            "INT" => value.parse::<i64>().ok().map(Data::I64),
                            _ => None,
                        };

                        if let Some(val) = parsed_value {
                            log::debug!("Captured result: {name} = {val:?}");
                            results.push((name.to_string(), val));
                        }
                    }
                }
            }

            results
        }));

        Ok(())
    }

    /// Get the captured results
    fn get_results(mut self) -> Vec<(String, Data)> {
        if let Some(thread) = self.result_thread.take() {
            thread.join().unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}

impl SeleneLibraryEngine {
    /// Create a new engine for the given library
    #[must_use]
    pub fn new(library_path: std::path::PathBuf, num_qubits: usize) -> Self {
        Self {
            library_path,
            library: None,
            num_qubits,
            callback_state: Arc::new(Mutex::new(CallbackState {
                operation_queue: VecDeque::new(),
                measurement_queue: VecDeque::new(),
                is_complete: false,
                is_waiting: false,
            })),
            execution_thread: None,
            final_results: BTreeMap::new(),
            result_stream: None,
        }
    }

    /// Load the library and set up callbacks
    fn load_and_setup(&mut self) -> Result<(), PecosError> {
        log::info!("Loading Selene library from {:?}", self.library_path);

        // Create TCP result stream first
        let mut result_stream = ResultStreamCapture::new()?;
        let result_uri = result_stream.get_uri();
        log::info!("Result stream URI: {result_uri}");

        // Start capturing results
        result_stream.start_capture()?;
        self.result_stream = Some(result_stream);

        // Load the shared library
        let lib = unsafe {
            Library::new(&self.library_path)
                .map_err(|e| PecosError::Processing(format!("Failed to load library: {e}")))?
        };

        // Get the callback setup function
        let set_callbacks: Symbol<
            extern "C" fn(
                *mut c_void,
                extern "C" fn(*mut c_void, *const u8, usize) -> i32,
                extern "C" fn(*mut c_void, *mut *mut u8, *mut usize) -> i32,
            ),
        > = unsafe {
            lib.get(b"pecos_bridge_set_engine_callbacks").map_err(|e| {
                PecosError::Processing(format!("Failed to get callback function: {e}"))
            })?
        };

        // Register our callbacks
        let state_ptr = Arc::as_ptr(&self.callback_state) as *mut c_void;
        set_callbacks(
            state_ptr,
            Self::handle_send_operation,
            Self::handle_receive_measurements,
        );

        // Configure Selene with the result stream URI
        // Check if library has a configuration function
        unsafe {
            if let Ok(configure_fn) = lib
                .get::<extern "C" fn(*const std::os::raw::c_char)>(b"pecos_bridge_configure_output")
            {
                use std::ffi::CString;
                let uri_cstr = CString::new(result_uri).map_err(|e| {
                    PecosError::Processing(format!("Failed to create CString: {e}"))
                })?;
                configure_fn(uri_cstr.as_ptr());
                log::info!("Configured output stream URI");
            }
        }

        log::info!("Callbacks registered successfully");
        self.library = Some(lib);
        Ok(())
    }

    /// Start execution of the quantum program
    fn start_execution(&mut self) -> Result<(), PecosError> {
        let lib = self
            .library
            .as_ref()
            .ok_or_else(|| PecosError::Processing("Library not loaded".to_string()))?;

        // Get the main entry point
        let qmain: Symbol<extern "C" fn() -> i32> = unsafe {
            lib.get(b"qmain")
                .or_else(|_| lib.get(b"main"))
                .map_err(|e| PecosError::Processing(format!("Failed to get entry point: {e}")))?
        };

        // Clone what we need for the thread
        let qmain = *qmain;
        let state = self.callback_state.clone();

        // Start execution in a separate thread
        self.execution_thread = Some(thread::spawn(move || {
            log::info!("Starting quantum program execution");
            let result = qmain();

            // Mark as complete
            state.lock().unwrap().is_complete = true;

            if result == 0 {
                log::info!("Quantum program completed successfully");
                Ok(())
            } else {
                Err(PecosError::Processing(format!(
                    "Program exited with code {result}"
                )))
            }
        }));

        Ok(())
    }

    /// Callback when Bridge sends quantum operations
    extern "C" fn handle_send_operation(context: *mut c_void, data: *const u8, len: usize) -> i32 {
        let state = unsafe { &*(context as *const Arc<Mutex<CallbackState>>) };
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        let message = ByteMessage::new(bytes);

        state.lock().unwrap().operation_queue.push_back(message);
        log::debug!("Received operation from Bridge ({len} bytes)");
        0 // Success
    }

    /// Callback when Bridge requests measurements
    extern "C" fn handle_receive_measurements(
        context: *mut c_void,
        data_out: *mut *mut u8,
        len_out: *mut usize,
    ) -> i32 {
        let state = unsafe { &*(context as *const Arc<Mutex<CallbackState>>) };
        let mut state_lock = state.lock().unwrap();

        if let Some(message) = state_lock.measurement_queue.pop_front() {
            let bytes = message.as_bytes();

            // Allocate memory for the result
            let buffer = bytes.to_vec().into_boxed_slice();
            let len = buffer.len();
            let ptr = Box::into_raw(buffer).cast::<u8>();

            unsafe {
                *data_out = ptr;
                *len_out = len;
            }

            state_lock.is_waiting = false;
            log::debug!("Provided measurements to Bridge ({len} bytes)");
            len as i32
        } else {
            state_lock.is_waiting = true;
            log::debug!("No measurements available, Bridge waiting");
            0 // No measurements available
        }
    }

    /// Get the next operations from the Bridge
    fn get_pending_operations(&mut self) -> Option<ByteMessage> {
        self.callback_state
            .lock()
            .unwrap()
            .operation_queue
            .pop_front()
    }

    /// Provide measurements to the Bridge
    fn provide_measurements(&mut self, measurements: ByteMessage) {
        self.callback_state
            .lock()
            .unwrap()
            .measurement_queue
            .push_back(measurements);
    }

    /// Check if Bridge is waiting for measurements
    fn is_bridge_waiting(&self) -> bool {
        self.callback_state.lock().unwrap().is_waiting
    }

    /// Check if execution is complete
    fn is_execution_complete(&self) -> bool {
        self.callback_state.lock().unwrap().is_complete
    }

    /// Wait for operations with timeout
    fn wait_for_operations(&mut self, timeout_ms: u64) -> Option<ByteMessage> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < u128::from(timeout_ms) {
            if let Some(ops) = self.get_pending_operations() {
                return Some(ops);
            }
            if self.is_execution_complete() {
                return None;
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }
}

impl ClassicalEngine for SeleneLibraryEngine {
    fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        // Not used - we implement ControlEngine instead
        Ok(ByteMessage::create_empty())
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        self.provide_measurements(message);
        Ok(())
    }

    fn get_results(&self) -> Result<Shot, PecosError> {
        // Convert final_results to Shot
        Ok(Shot {
            data: self.final_results.clone(),
        })
    }

    fn compile(&self) -> Result<(), PecosError> {
        // Compilation already done by Selene
        Ok(())
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        // Reset state for new shot
        pecos_reset_callback_state();
        *self.callback_state.lock().unwrap() = CallbackState {
            operation_queue: VecDeque::new(),
            measurement_queue: VecDeque::new(),
            is_complete: false,
            is_waiting: false,
        };
        self.final_results.clear();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ControlEngine for SeleneLibraryEngine {
    type Input = ();
    type Output = Shot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        log::info!("Starting Selene library engine");

        // Load library and set up callbacks if not already done
        if self.library.is_none() {
            self.load_and_setup()?;
        }

        // Reset callback state
        pecos_reset_callback_state();

        // Start quantum program execution
        self.start_execution()?;

        // Wait for first operations
        if let Some(ops) = self.wait_for_operations(5000) {
            log::info!("Got initial operations from Bridge");
            Ok(EngineStage::NeedsProcessing(ops))
        } else if self.is_execution_complete() {
            log::info!("Execution complete immediately");
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            Err(PecosError::Processing(
                "Timeout waiting for initial operations".to_string(),
            ))
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
        log::debug!("Continuing with measurements");

        // Provide measurements to Bridge
        self.provide_measurements(measurements);

        // Give Bridge time to process
        thread::sleep(Duration::from_millis(50));

        // Check for more operations
        if let Some(ops) = self.wait_for_operations(1000) {
            log::debug!("Got more operations");
            Ok(EngineStage::NeedsProcessing(ops))
        } else if self.is_execution_complete() {
            // Wait for thread to finish
            if let Some(thread) = self.execution_thread.take() {
                thread.join().map_err(|_| {
                    PecosError::Processing("Execution thread panicked".to_string())
                })??;
            }

            // Collect results from TCP stream
            if let Some(stream) = self.result_stream.take() {
                let tcp_results = stream.get_results();
                for (name, value) in tcp_results {
                    self.final_results.insert(name, value);
                }
            }

            log::info!(
                "Execution complete with {} results",
                self.final_results.len()
            );
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            // Check if Bridge is still waiting
            if self.is_bridge_waiting() {
                Err(PecosError::Processing(
                    "Bridge still waiting after providing measurements".to_string(),
                ))
            } else {
                // Assume complete
                Ok(EngineStage::Complete(self.get_results()?))
            }
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}

// Implement Clone for worker isolation
impl Clone for SeleneLibraryEngine {
    fn clone(&self) -> Self {
        Self {
            library_path: self.library_path.clone(),
            library: None, // Each clone loads its own library
            num_qubits: self.num_qubits,
            callback_state: Arc::new(Mutex::new(CallbackState {
                operation_queue: VecDeque::new(),
                measurement_queue: VecDeque::new(),
                is_complete: false,
                is_waiting: false,
            })),
            execution_thread: None,
            final_results: BTreeMap::new(),
            result_stream: None,
        }
    }
}

// Implement Engine trait
impl Engine for SeleneLibraryEngine {
    type Input = ();
    type Output = Shot;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        // Use the ControlEngine implementation
        match self.start(())? {
            EngineStage::Complete(shot) => Ok(shot),
            EngineStage::NeedsProcessing(_ops) => {
                // This shouldn't happen for library engine
                Err(PecosError::Processing(
                    "Unexpected NeedsProcessing state".to_string(),
                ))
            }
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        <Self as ClassicalEngine>::reset(self)
    }
}
