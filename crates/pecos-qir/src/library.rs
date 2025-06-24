use libloading::{Library, Symbol};
#[cfg(unix)]
use libloading::os::unix::Library as UnixLibrary;
use log::{debug, warn};
use pecos_core::errors::PecosError;
use pecos_engines::byte_message::ByteMessage;
use pecos_engines::shot_results::{Data, Shot};
use std::ffi::{CStr, c_char};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// FFI struct for shot data (matches runtime.rs)
#[repr(C)]
struct FFIShotData {
    names: *mut *mut c_char,
    values: *mut i64,
    count: usize,
}

/// QIR Library for executing quantum programs
///
/// This struct represents a loaded QIR library that can be used to execute
/// quantum programs. It provides methods for calling functions in the library
/// and retrieving the generated quantum commands.
///
/// # Thread Safety
///
/// The QIR Library is designed to be thread-safe and can be used from multiple
/// threads. Each thread gets its own copy of the library to avoid conflicts.
///
/// # Error Handling
///
/// Errors are propagated through the Result type and include context about
/// the operation that failed.
///
/// # Examples
///
/// ```no_run
/// use pecos_qir::library::QirLibrary;
/// use std::path::Path;
///
/// // Load a QIR library from a file
/// let library = QirLibrary::load(Path::new("path/to/library.so")).unwrap();
///
/// // Call the main function in the library
/// library.call_function(b"main").unwrap();
///
/// // Get the generated quantum commands
/// let commands = library.get_binary_commands().unwrap();
///
/// // Reset the library state
/// library.reset().unwrap();
/// ```
pub struct QirLibrary {
    /// The loaded dynamic library wrapped in Arc for safe sharing
    library: Arc<Mutex<Library>>,

    /// Path to the library file
    path: PathBuf,
}

impl Clone for QirLibrary {
    fn clone(&self) -> Self {
        debug!("QIR Library: Cloning library from {:?}", self.path);

        // Share the same library instance via Arc - no need to reload
        Self {
            library: Arc::clone(&self.library),
            path: self.path.clone(),
        }
    }
}

impl QirLibrary {
    /// Load a QIR library from the given path
    ///
    /// This method loads a compiled QIR library from the specified path and
    /// initializes the `QirLibrary` struct for interacting with it.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the compiled QIR library
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
        debug!("QIR: Loading library from {:?}", path);

        // Perform thorough file verification before loading
        if !path.exists() {
            return Err(Self::log_error(
                "File not found",
                format!("Path: {}", path.display()),
            ));
        }

        // Check if the file is readable and has valid content
        match std::fs::metadata(path) {
            Ok(metadata) => {
                // Check if the file is a regular file
                if !metadata.is_file() {
                    return Err(Self::log_error(
                        "Not a regular file",
                        format!("Path: {}", path.display()),
                    ));
                }

                // Check if the file has reasonable size (at least 1KB for a valid library)
                let file_size = metadata.len();
                if file_size < 1024 {
                    return Err(Self::log_error(
                        "File too small to be a valid library",
                        format!("Path: {} (size: {} bytes)", path.display(), file_size),
                    ));
                }

                // Log file details for debugging
                debug!(
                    "QIR: Verified file {} (size: {} bytes)",
                    path.display(),
                    file_size
                );
            }
            Err(e) => {
                return Err(Self::log_error(
                    "Failed to get file metadata",
                    format!("Path: {}, Error: {}", path.display(), e),
                ));
            }
        }

        // Try to load the library with retries
        let max_retries = 3;
        Self::load_library_with_retries(path, max_retries)
    }

    /// Helper function to implement exponential backoff
    fn sleep_with_backoff(retry_count: usize) {
        let sleep_duration =
            Duration::from_millis(100 * 2u64.pow(u32::try_from(retry_count).unwrap_or(0)));
        debug!("QIR: Sleeping for {:?} before retry", sleep_duration);
        thread::sleep(sleep_duration);
    }

    /// Helper function to load a library with retries
    ///
    /// This function attempts to load a library from the given path, with retries
    /// if the initial attempt fails due to "Text file busy" errors.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the library file
    /// * `max_retries` - Maximum number of retry attempts
    /// * `thread_id` - Thread ID for logging
    ///
    /// # Returns
    ///
    /// * `Result<Self, PecosError>` - The loaded library if successful
    fn load_library_with_retries(path: &Path, max_retries: usize) -> Result<Self, PecosError> {
        let mut retry_count = 0;

        while retry_count < max_retries {
            debug!(
                "QIR: Loading library attempt {}/{}",
                retry_count + 1,
                max_retries
            );

            // Load library with proper isolation flags
            let library_result = if cfg!(unix) {
                #[cfg(unix)]
                {
                    // Use RTLD_LOCAL for symbol isolation and RTLD_NODELETE to prevent segfaults
                    // RTLD_NODELETE prevents the library from being unloaded during cleanup
                    debug!("QIR: Using RTLD_LOCAL | RTLD_NODELETE for library loading");
                    unsafe {
                        UnixLibrary::open(Some(path), libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NODELETE)
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
            
            match library_result {
                Ok(library) => {
                    debug!("QIR: Successfully loaded library from {:?}", path);
                    return Ok(Self {
                        library: Arc::new(Mutex::new(library)),
                        path: path.to_path_buf(),
                    });
                }
                Err(e) => {
                    Self::log_error(
                        "Failed to load library",
                        format!("Attempt {}/{}: {}", retry_count + 1, max_retries, e),
                    );

                    // Sleep before retrying, with exponential backoff
                    Self::sleep_with_backoff(retry_count);
                    retry_count += 1;
                }
            }
        }

        // If we get here, all attempts failed
        Err(Self::log_error(
            "Failed to load library after multiple attempts",
            format!("Max retries ({max_retries}) exceeded"),
        ))
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
        debug!("QIR Library: Calling function {:?}", name);

        unsafe {
            // Get the function pointer
            let library_guard = self.library.lock().unwrap();

            // Try different function signatures
            // First try standard QIR signature (returns i32)
            if let Ok(func) = library_guard.get::<Symbol<unsafe extern "C" fn() -> i32>>(name) {
                let result = func();
                debug!("QIR Library: Function call returned {}", result);
                Ok(result)
            }
            // Try HUGR signature (returns tuple, but we'll treat as void)
            else if let Ok(func) = library_guard.get::<Symbol<unsafe extern "C" fn()>>(name) {
                func();
                debug!("QIR Library: Function call completed (void return)");
                Ok(0)
            } else {
                Err(Self::log_error(
                    "Failed to get function",
                    format!("Function {} not found", String::from_utf8_lossy(name)),
                ))
            }
        }
    }

    /// Resets the QIR runtime
    ///
    /// This method calls the `qir_runtime_reset` function in the loaded library
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
        debug!("QIR Library: Resetting QIR runtime");

        unsafe {
            // Get the function pointer
            let library_guard = self.library.lock().unwrap();
            let reset: Symbol<unsafe extern "C" fn()> = library_guard
                .get(b"qir_runtime_reset")
                .map_err(|e| Self::log_error("Failed to get reset function", e))?;

            // Call the function
            reset();
            debug!("QIR Library: Successfully reset QIR runtime");
        }

        Ok(())
    }

    /// Gets the binary commands generated by the QIR runtime
    ///
    /// This method calls the `qir_runtime_get_binary_commands` function in the loaded library
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

        debug!("QIR Library: Getting binary commands");

        // Get the get_binary_commands function
        let library_guard = self.library.lock().unwrap();
        let get_binary_commands: Symbol<unsafe extern "C" fn() -> *mut FFIByteData> = unsafe {
            library_guard
                .get(b"qir_runtime_get_binary_commands")
                .map_err(|e| {
                    Self::log_error("Failed to get qir_runtime_get_binary_commands symbol", e)
                })?
        };

        // Get the free_binary_commands function
        let free_binary_commands: Symbol<unsafe extern "C" fn(*mut FFIByteData)> = unsafe {
            library_guard
                .get(b"qir_runtime_free_binary_commands")
                .map_err(|e| {
                    Self::log_error("Failed to get qir_runtime_free_binary_commands symbol", e)
                })?
        };

        // Call the get_binary_commands function
        let ffi_ptr = unsafe { get_binary_commands() };
        if ffi_ptr.is_null() {
            return Err(Self::log_error(
                "Got null pointer from qir_runtime_get_binary_commands",
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
    /// This method calls the `qir_runtime_get_shot_results` function in the loaded library
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
        debug!("QIR Library: Getting shot results");

        // Get the function pointers
        let (get_shot_results_ptr, free_shot_data_ptr) = {
            let library_guard = self.library.lock().unwrap();

            // Get the get_shot_results function
            let get_shot_results: Symbol<unsafe extern "C" fn() -> *mut FFIShotData> = unsafe {
                library_guard
                    .get(b"qir_runtime_get_shot_results")
                    .map_err(|e| {
                        Self::log_error("Failed to get qir_runtime_get_shot_results symbol", e)
                    })?
            };

            // Get the free function
            let free_shot_data: Symbol<unsafe extern "C" fn(*mut FFIShotData)> = unsafe {
                library_guard
                    .get(b"qir_runtime_free_shot_data")
                    .map_err(|e| {
                        Self::log_error("Failed to get qir_runtime_free_shot_data symbol", e)
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
            debug!("QIR Library: No shot results available");
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
            "QIR Library: Retrieved shot with {} registers",
            shot.data.len()
        );
        Ok(Some(shot))
    }

    /// Updates the measurement results in the QIR runtime
    ///
    /// This method calls the `qir_runtime_update_measurement_results` function to
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
            "QIR Library: Updating {} measurement results",
            results.len() / 2
        );

        unsafe {
            // Get the update function
            let library_guard = self.library.lock().unwrap();
            let update_fn: Symbol<unsafe extern "C" fn(*const u32, usize)> = library_guard
                .get(b"qir_runtime_update_measurement_results")
                .map_err(|e| {
                    Self::log_error("Failed to get update_measurement_results function", e)
                })?;

            // Call the function with the results data
            // The second parameter is the number of result pairs (not total array length)
            update_fn(results.as_ptr(), results.len() / 2);

            debug!("QIR Library: Measurement results updated");
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
        debug!("QIR Library: Finalizing shot");

        unsafe {
            // Get the finalize function
            let library_guard = self.library.lock().unwrap();
            let finalize: Symbol<unsafe extern "C" fn()> = library_guard
                .get(b"qir_runtime_finalize_shot")
                .map_err(|e| Self::log_error("Failed to get finalize_shot function", e))?;

            // Call the function
            finalize();

            debug!("QIR Library: Shot finalized");
            Ok(())
        }
    }

    /// Helper function to log errors with thread ID context
    fn log_error<E: std::fmt::Display>(context: &str, error: E) -> PecosError {
        let error_msg = format!("{context}: {error}");
        warn!("QIR Library: {}", error_msg);
        PecosError::Resource(error_msg.to_string())
    }
}

impl Drop for QirLibrary {
    fn drop(&mut self) {
        let strong_count = Arc::strong_count(&self.library);
        debug!("QIR Library: Dropping library reference (remaining references: {})", strong_count - 1);
        
        // Check if we're in a test environment
        let is_python_test = std::env::var("PYTEST_CURRENT_TEST").is_ok() 
            || std::env::var("PYTHON_TEST_MODE").is_ok();
        
        // If this is the last reference, set shutdown flag to prevent runtime re-initialization
        if strong_count == 1 {
            crate::runtime_registry::set_shutting_down();
            
            // Skip reset entirely during drop to avoid segfaults
            // The runtime will be cleaned up when the library is unloaded
            debug!("QIR Library: Skipping reset during drop to avoid segfaults");
        }
        
        if is_python_test {
            debug!("QIR Library: Test environment - allowing normal library unload for proper cleanup");
        } else {
            debug!("QIR Library: Production environment - RTLD_NODELETE prevents crashes");
        }
    }
}

// No longer needed - we now pass raw bytes across the FFI boundary
