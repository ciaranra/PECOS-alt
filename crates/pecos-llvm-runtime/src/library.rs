#[cfg(unix)]
use libloading::os::unix::Library as UnixLibrary;
use libloading::{Library, Symbol};
use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::byte_message::ByteMessage;
use pecos_engines::shot_results::{Data, Shot};
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// Import FFIShotData from runtime to avoid duplication
use crate::runtime::FFIShotData;
use crate::utils::{LLVM_LOG, log_error, retry_with_backoff, validate_library_file};

/// LLVM Library for executing quantum programs
///
/// This struct represents a loaded LLVM library that can be used to execute
/// quantum programs. It provides methods for calling functions in the library
/// and retrieving the generated quantum commands.
///
/// # Thread Safety
///
/// The LLVM Library is designed to be thread-safe and can be used from multiple
/// threads. Each thread gets its own copy of the library to avoid conflicts.
///
/// # Error Handling
///
/// Errors are propagated through the Result type and include context about
/// the operation that failed.
///
/// # Note
/// 
/// This is an internal implementation detail used by `LlvmEngine`. 
/// Users should interact with `LlvmEngine` instead of using `LlvmLibrary` directly.
/// 
/// # Example - Use LlvmEngine instead
///
/// ```no_run
/// use pecos_llvm_runtime::prelude::*;
/// use std::path::PathBuf;
///
/// // Create an LLVM engine (this handles library loading internally)
/// let mut engine = LlvmEngine::new(PathBuf::from("program.ll"));
///
/// // Run a single execution (the engine manages the library for you)
/// let shot = engine.process(()).unwrap();
/// println!("Result: {:?}", shot);
/// ```
pub struct LlvmLibrary {
    /// The loaded dynamic library wrapped in Arc for safe sharing
    library: Arc<Mutex<Library>>,

    /// Path to the library file
    path: PathBuf,
}

impl Clone for LlvmLibrary {
    fn clone(&self) -> Self {
        debug!("LLVM Library: Cloning library from {:?}", self.path);

        // Share the same library instance via Arc - no need to reload
        Self {
            library: Arc::clone(&self.library),
            path: self.path.clone(),
        }
    }
}

impl LlvmLibrary {
    /// Load an LLVM library from the given path
    ///
    /// This method loads a compiled LLVM library from the specified path and
    /// initializes the `LlvmLibrary` struct for interacting with it.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the compiled LLVM library
    ///
    /// # Returns
    ///
    /// * `Result<Self, PecosError>` - The loaded library if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::ResourceError` - If the library file does not exist or cannot be loaded
    ///
    /// # Thread Safety
    ///
    /// This method implements retry logic for handling "Text file busy" errors
    /// that can occur when multiple threads try to load the same library file
    /// simultaneously.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PecosError> {
        let path = path.as_ref();
        LLVM_LOG.debug(format!("Loading library from {}", path.display()));

        // Validate the library file
        let file_size = validate_library_file(path)?;
        LLVM_LOG.debug(format!(
            "Verified file {} (size: {} bytes)",
            path.display(),
            file_size
        ));

        // Load the library with retry logic
        let path_buf = path.to_path_buf();
        let library = retry_with_backoff(
            || Self::load_library_once(&path_buf),
            3,   // max attempts
            100, // initial delay in ms
        )?;

        Ok(Self {
            library: Arc::new(Mutex::new(library)),
            path: path_buf,
        })
    }

    /// Load the library once (used by retry logic)
    fn load_library_once(path: &Path) -> Result<Library, PecosError> {
        // Load library with proper isolation flags
        let library_result = if cfg!(unix) {
            #[cfg(unix)]
            {
                // Use RTLD_LOCAL for symbol isolation and RTLD_NODELETE to prevent segfaults
                LLVM_LOG.debug("Using RTLD_LOCAL | RTLD_NODELETE for library loading");
                unsafe {
                    UnixLibrary::open(
                        Some(path),
                        libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NODELETE,
                    )
                    .map(Library::from)
                }
            }
            #[cfg(not(unix))]
            {
                unsafe { Library::new(path) }
            }
        } else {
            unsafe { Library::new(path) }
        };

        library_result.map_err(|e| {
            PecosError::Resource(format!("Failed to load library {}: {}", path.display(), e))
        })
    }

    /// Check if a function exists in the loaded library
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function to check as a byte slice
    ///
    /// # Returns
    ///
    /// * `Result<bool, PecosError>` - True if function exists, false otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the function name contains null bytes
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned
    pub fn has_function(&self, name: &[u8]) -> Result<bool, PecosError> {
        let library_guard = self.library.lock().unwrap();
        let result: Result<Symbol<unsafe extern "C" fn() -> i32>, _> =
            unsafe { library_guard.get(name) };
        Ok(result.is_ok())
    }

    /// Calls a function in the loaded library
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function to call
    ///
    /// # Returns
    ///
    /// * `Result<i32, PecosError>` - The return value of the function if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::Resource` - If the function is not found in the library or the call fails
    ///
    /// # Panics
    ///
    /// This function will panic if the internal mutex is poisoned.
    pub fn call_function(&self, name: &[u8]) -> Result<i32, PecosError> {
        debug!("LLVM Library: Calling function {name:?}");

        unsafe {
            // Get the function pointer
            let library_guard = self.library.lock().unwrap();

            // Try different function signatures
            // First try standard QIR signature (returns i32)
            if let Ok(func) = library_guard.get::<Symbol<unsafe extern "C" fn() -> i32>>(name) {
                let result = func();
                debug!("LLVM Library: Function call returned {result}");
                Ok(result)
            }
            // Try HUGR tuple signatures
            else if let Ok(func) =
                library_guard.get::<Symbol<unsafe extern "C" fn() -> (i32, i32)>>(name)
            {
                let (a, b) = func();
                debug!("LLVM Library: Function returned tuple ({a}, {b})");
                // Store tuple values in the runtime state
                crate::runtime::core_runtime::store_tuple_return(&[a, b]);
                Ok(0) // Return 0 to indicate success
            } else if let Ok(func) =
                library_guard.get::<Symbol<unsafe extern "C" fn() -> (i32, i32, i32)>>(name)
            {
                let (a, b, c) = func();
                debug!("LLVM Library: Function returned tuple ({a}, {b}, {c})");
                // Store tuple values in the runtime state
                crate::runtime::core_runtime::store_tuple_return(&[a, b, c]);
                Ok(0) // Return 0 to indicate success
            }
            // Try void signature
            else if let Ok(func) = library_guard.get::<Symbol<unsafe extern "C" fn()>>(name) {
                func();
                debug!("LLVM Library: Function call completed (void return)");
                Ok(0)
            } else {
                Err(log_error(
                    "QIR Library",
                    "Failed to get function",
                    format!("Function {} not found", String::from_utf8_lossy(name)),
                ))
            }
        }
    }

    /// Resets the QIR runtime
    ///
    /// This method calls the `llvm_runtime_reset` function in the loaded library
    /// to reset the QIR runtime state.
    ///
    /// # Returns
    ///
    /// * `Result<(), PecosError>` - Success or error
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::Resource` - If the reset function is not found in the library or the call fails
    ///
    /// # Panics
    ///
    /// This function will panic if the internal mutex is poisoned.
    pub fn reset(&self) -> Result<(), PecosError> {
        debug!("LLVM Library: Resetting LLVM runtime");

        unsafe {
            // Get the function pointer
            let library_guard = self.library.lock().unwrap();
            let reset: Symbol<unsafe extern "C" fn()> = library_guard
                .get(b"llvm_runtime_reset")
                .map_err(|e| log_error("QIR Library", "Failed to get reset function", e))?;

            // Call the function
            reset();
            debug!("LLVM Library: Successfully reset LLVM runtime");
        }

        Ok(())
    }

    /// Gets the binary commands generated by the QIR runtime
    ///
    /// This method calls the `llvm_runtime_get_binary_commands` function in the loaded library
    /// to get the binary commands generated by the QIR runtime.
    ///
    /// # Returns
    ///
    /// * `Result<ByteMessage, PecosError>` - The binary commands if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::LibraryError` - If the function is not found in the library or the call fails
    ///
    /// # Panics
    ///
    /// This function will panic if the internal mutex is poisoned.
    pub fn get_binary_commands(&self) -> Result<ByteMessage, PecosError> {
        use crate::runtime::FFIByteData;

        debug!("LLVM Library: Getting binary commands");

        // Get the get_binary_commands function
        let library_guard = self.library.lock().unwrap();
        let get_binary_commands: Symbol<unsafe extern "C" fn() -> *mut FFIByteData> = unsafe {
            library_guard
                .get(b"llvm_runtime_get_binary_commands")
                .map_err(|e| {
                    log_error(
                        "QIR Library",
                        "Failed to get llvm_runtime_get_binary_commands symbol",
                        e,
                    )
                })?
        };

        // Get the free_binary_commands function
        let free_binary_commands: Symbol<unsafe extern "C" fn(*mut FFIByteData)> = unsafe {
            library_guard
                .get(b"llvm_runtime_free_binary_commands")
                .map_err(|e| {
                    log_error(
                        "QIR Library",
                        "Failed to get llvm_runtime_free_binary_commands symbol",
                        e,
                    )
                })?
        };

        // Call the get_binary_commands function
        let ffi_ptr = unsafe { get_binary_commands() };
        if ffi_ptr.is_null() {
            return Err(log_error(
                "QIR Library",
                "Got null pointer from llvm_runtime_get_binary_commands",
                "Cannot retrieve commands",
            ));
        }

        // Get the FFI data
        let ffi_data = unsafe { &*ffi_ptr };

        // Create ByteMessage from the aligned u32 data while preserving alignment
        let message =
            if ffi_data.byte_len > 0 && !ffi_data.data.is_null() && ffi_data.word_count > 0 {
                // Reconstruct aligned data from FFI
                let aligned_data =
                    unsafe { std::slice::from_raw_parts(ffi_data.data, ffi_data.word_count) };

                // Create ByteMessage directly from u32 data to maintain alignment
                ByteMessage::from_aligned_u32_data(aligned_data.to_vec(), ffi_data.byte_len)
            } else {
                ByteMessage::create_empty()
            };

        // Free the FFI data
        unsafe { free_binary_commands(ffi_ptr) };

        Ok(message)
    }

    /// Gets the shot results from the QIR runtime
    ///
    /// This method calls the `llvm_runtime_get_shot_results` function in the loaded library
    /// to retrieve the classical register values as a Shot.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Shot>, PecosError>` - The shot results if available, or None
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::Resource` - If the `get_shot_results` function is not found in the library
    ///
    /// # Panics
    ///
    /// This function will panic if the internal mutex is poisoned.
    pub fn get_shot_results(&self) -> Result<Option<Shot>, PecosError> {
        debug!("LLVM Library: Getting shot results");

        // Get the function pointers
        let (get_shot_results_ptr, free_shot_data_ptr) = {
            let library_guard = self.library.lock().unwrap();

            // Get the get_shot_results function
            let get_shot_results: Symbol<unsafe extern "C" fn() -> *mut FFIShotData> = unsafe {
                library_guard
                    .get(b"llvm_runtime_get_shot_results")
                    .map_err(|e| {
                        log_error(
                            "QIR Library",
                            "Failed to get llvm_runtime_get_shot_results symbol",
                            e,
                        )
                    })?
            };

            // Get the free function
            let free_shot_data: Symbol<unsafe extern "C" fn(*mut FFIShotData)> = unsafe {
                library_guard
                    .get(b"llvm_runtime_free_shot_data")
                    .map_err(|e| {
                        log_error(
                            "QIR Library",
                            "Failed to get llvm_runtime_free_shot_data symbol",
                            e,
                        )
                    })?
            };

            // Return raw function pointers
            unsafe {
                (
                    get_shot_results.into_raw().into_raw(),
                    free_shot_data.into_raw().into_raw(),
                )
            }
        };

        // Convert back to function pointers
        let get_shot_results: unsafe extern "C" fn() -> *mut FFIShotData =
            unsafe { std::mem::transmute(get_shot_results_ptr) };
        let free_shot_data: unsafe extern "C" fn(*mut FFIShotData) =
            unsafe { std::mem::transmute(free_shot_data_ptr) };

        // Call the get_shot_results function
        let ffi_ptr = unsafe { get_shot_results() };

        if ffi_ptr.is_null() {
            debug!("LLVM Library: No shot results available");
            return Ok(None);
        }

        // Convert FFI data to Shot
        let shot = unsafe {
            let ffi_data = &*ffi_ptr;
            let mut shot = Shot::default();

            for i in 0..ffi_data.count {
                // Get the name
                let name_ptr = *ffi_data.names.add(i);
                let name = CStr::from_ptr(name_ptr).to_string_lossy().into_owned();

                // Get the value
                let value = *ffi_data.values.add(i);

                // Insert into shot - always use I64 for consistency with QIR standard
                shot.data.insert(name, Data::I64(value));
            }

            shot
        };

        // Free the FFI data
        unsafe { free_shot_data(ffi_ptr) };

        debug!(
            "LLVM Library: Retrieved shot with {} registers",
            shot.data.len()
        );
        Ok(Some(shot))
    }

    /// Updates the measurement results in the QIR runtime
    ///
    /// This method calls the `llvm_runtime_update_measurement_results` function to
    /// provide measurement results from the quantum system to the runtime.
    ///
    /// # Arguments
    ///
    /// * `results` - A slice of alternating `result_id` and `measurement_value` (0 or 1)
    ///
    /// # Returns
    ///
    /// * `Result<(), PecosError>` - Success or error
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::Resource` - If the update function is not found in the library
    ///
    /// # Panics
    ///
    /// This function will panic if the internal mutex is poisoned.
    pub fn update_measurement_results(&self, results: &[u32]) -> Result<(), PecosError> {
        debug!(
            "LLVM Library: Updating {} measurement results",
            results.len() / 2
        );

        unsafe {
            // Get the update function
            let library_guard = self.library.lock().unwrap();
            let update_fn: Symbol<unsafe extern "C" fn(*const u32, usize)> = library_guard
                .get(b"llvm_runtime_update_measurement_results")
                .map_err(|e| {
                    log_error(
                        "QIR Library",
                        "Failed to get update_measurement_results function",
                        e,
                    )
                })?;

            // Call the function with the results data
            // The second parameter is the number of result pairs (not total array length)
            update_fn(results.as_ptr(), results.len() / 2);

            debug!("LLVM Library: Measurement results updated");
            Ok(())
        }
    }

    /// Finalizes the shot after measurements have been processed
    ///
    /// This method should be called after measurement results have been updated
    /// to finalize the classical register values.
    ///
    /// # Returns
    ///
    /// * `Result<(), PecosError>` - Success or error
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::Resource` - If the finalize function is not found in the library
    ///
    /// # Panics
    ///
    /// This function will panic if the internal mutex is poisoned.
    pub fn finalize_shot(&self) -> Result<(), PecosError> {
        debug!("LLVM Library: Finalizing shot");

        unsafe {
            // Get the finalize function
            let library_guard = self.library.lock().unwrap();
            let finalize: Symbol<unsafe extern "C" fn()> = library_guard
                .get(b"llvm_runtime_finalize_shot")
                .map_err(|e| log_error("QIR Library", "Failed to get finalize_shot function", e))?;

            // Call the function
            finalize();

            debug!("LLVM Library: Shot finalized");
            Ok(())
        }
    }
}

impl Drop for LlvmLibrary {
    fn drop(&mut self) {
        let strong_count = Arc::strong_count(&self.library);
        debug!(
            "QIR Library: Dropping library reference (remaining references: {})",
            strong_count - 1
        );

        // Check if we're in a test environment
        let is_python_test = std::env::var("PYTEST_CURRENT_TEST").is_ok()
            || std::env::var("PYTHON_TEST_MODE").is_ok();

        // If this is the last reference, set shutdown flag to prevent runtime re-initialization
        if strong_count == 1 {
            crate::runtime::registry::set_shutting_down();

            // Skip reset entirely during drop to avoid segfaults
            // The runtime will be cleaned up when the library is unloaded
            debug!("QIR Library: Skipping reset during drop to avoid segfaults");
        }

        if is_python_test {
            debug!(
                "QIR Library: Test environment - allowing normal library unload for proper cleanup"
            );
        } else {
            debug!("QIR Library: Production environment - RTLD_NODELETE prevents crashes");
        }
    }
}

// No longer needed - we now pass raw bytes across the FFI boundary
