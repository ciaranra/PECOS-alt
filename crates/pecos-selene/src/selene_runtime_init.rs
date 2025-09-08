//! Proper initialization of Selene runtime
//!
//! This module provides proper initialization of the Selene runtime
//! by creating a configuration file and calling selene_load_config.

use std::ffi::{CString, c_char};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Represents a Selene instance
#[repr(C)]
pub struct SeleneInstance {
    _private: [u8; 0], // Opaque type
}

/// Result types matching Selene's FFI
#[repr(C)]
pub struct SeleneVoidResult {
    pub error_code: u32,
}

#[repr(C)]
pub struct SeleneU64Result {
    pub error_code: u32,
    pub value: u64,
}

// External functions from libselene.so
unsafe extern "C" {
    fn selene_load_config(
        instance: *mut *mut SeleneInstance,
        config_file: *const c_char,
    ) -> SeleneVoidResult;

    fn selene_on_shot_start(instance: *mut SeleneInstance, shot_index: u64) -> SeleneVoidResult;

    fn selene_on_shot_end(instance: *mut SeleneInstance) -> SeleneVoidResult;

    fn selene_shot_count(instance: *mut SeleneInstance) -> SeleneU64Result;

    fn selene_exit(instance: *mut SeleneInstance) -> SeleneVoidResult;
}

/// Wrapper for initialized Selene runtime
pub struct SeleneRuntime {
    instance: *mut SeleneInstance,
    config_path: PathBuf,
    temp_dir: Option<tempfile::TempDir>,
}

impl SeleneRuntime {
    /// Initialize Selene runtime with configuration
    pub fn new(num_qubits: usize, num_shots: usize) -> Result<Self, String> {
        // Create temp directory for configuration
        let temp_dir =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let config_path = temp_dir.path().join("selene_config.yaml");

        // Create configuration YAML
        let config_yaml = format!(
            r#"# Selene configuration for PECOS
n_qubits: {}
output_stream: "stdout"
shots:
  count: {}
  offset: 0
  increment: 1
simulator:
  name: "quest"
  file: ""
  args: []
error_model:
  name: "ideal"
  file: ""
  args: []
runtime:
  name: "simple"
  file: ""
  args: []
artifact_dir: "{}"
event_hooks:
  metrics: false
"#,
            num_qubits,
            num_shots,
            temp_dir.path().join("artifacts").display()
        );

        // Write configuration file
        let mut file = fs::File::create(&config_path)
            .map_err(|e| format!("Failed to create config file: {}", e))?;
        file.write_all(config_yaml.as_bytes())
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        // Create artifacts directory
        fs::create_dir_all(temp_dir.path().join("artifacts"))
            .map_err(|e| format!("Failed to create artifacts dir: {}", e))?;

        // Convert path to C string
        let config_cstring = CString::new(config_path.to_str().unwrap())
            .map_err(|e| format!("Failed to create C string: {}", e))?;

        // Initialize Selene
        let mut instance: *mut SeleneInstance = std::ptr::null_mut();
        let result = unsafe { selene_load_config(&mut instance, config_cstring.as_ptr()) };

        if result.error_code != 0 {
            return Err(format!(
                "selene_load_config failed with error code: {}",
                result.error_code
            ));
        }

        if instance.is_null() {
            return Err("selene_load_config returned null instance".to_string());
        }

        println!(
            "*** SELENE RUNTIME: Initialized with {} qubits, {} shots ***",
            num_qubits, num_shots
        );

        Ok(Self {
            instance,
            config_path,
            temp_dir: Some(temp_dir),
        })
    }

    /// Get the Selene instance pointer
    pub fn instance_ptr(&self) -> *mut SeleneInstance {
        self.instance
    }

    /// Start a shot
    pub fn start_shot(&mut self, shot_index: u64) -> Result<(), String> {
        let result = unsafe { selene_on_shot_start(self.instance, shot_index) };

        if result.error_code != 0 {
            return Err(format!(
                "selene_on_shot_start failed with error code: {}",
                result.error_code
            ));
        }

        Ok(())
    }

    /// End a shot
    pub fn end_shot(&mut self) -> Result<(), String> {
        let result = unsafe { selene_on_shot_end(self.instance) };

        if result.error_code != 0 {
            return Err(format!(
                "selene_on_shot_end failed with error code: {}",
                result.error_code
            ));
        }

        Ok(())
    }

    /// Get the number of shots configured
    pub fn shot_count(&self) -> Result<u64, String> {
        let result = unsafe { selene_shot_count(self.instance) };

        if result.error_code != 0 {
            return Err(format!(
                "selene_shot_count failed with error code: {}",
                result.error_code
            ));
        }

        Ok(result.value)
    }
}

impl Drop for SeleneRuntime {
    fn drop(&mut self) {
        // Clean up Selene instance
        if !self.instance.is_null() {
            unsafe {
                let _ = selene_exit(self.instance);
            }
        }
    }
}

/// Thread-local storage for the current Selene instance
/// This allows the plugin to access the instance via extern functions
thread_local! {
    static CURRENT_INSTANCE: std::cell::RefCell<Option<*mut SeleneInstance>> = const { std::cell::RefCell::new(None) };
}

/// Set the current Selene instance for this thread
pub fn set_current_instance(instance: *mut SeleneInstance) {
    CURRENT_INSTANCE.with(|i| {
        *i.borrow_mut() = Some(instance);
    });
}

/// Get the current Selene instance for this thread
pub fn get_current_instance() -> *mut SeleneInstance {
    CURRENT_INSTANCE.with(|i| i.borrow().unwrap_or(std::ptr::null_mut()))
}

/// Clear the current Selene instance for this thread
pub fn clear_current_instance() {
    CURRENT_INSTANCE.with(|i| {
        *i.borrow_mut() = None;
    });
}
