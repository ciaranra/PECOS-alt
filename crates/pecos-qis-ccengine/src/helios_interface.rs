//! Selene Helios interface implementation
//!
//! This module implements the QisInterface trait using Selene's Helios compiler.
//! It links QIS/LLVM IR bitcode with the Helios interface library to create
//! an executable that can be run in a controlled manner.

use crate::interface_impl::{QisInterface, ProgramFormat};
use pecos_qis_interface::{QisInterface as OperationList};
use pecos_core::prelude::PecosError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;

/// Selene Helios interface implementation
///
/// This interface works by:
/// 1. Linking the program bitcode with the Helios interface library
/// 2. Creating an executable
/// 3. Running that executable in a controlled way
/// 4. Intercepting function calls via FFI to collect operations
pub struct QisSeleneHeliosInterface {
    /// Path to the linked executable (if created)
    executable_path: Option<PathBuf>,

    /// The program bytes
    program: Vec<u8>,

    /// The program format
    format: ProgramFormat,

    /// Collected operations from execution
    operations: OperationList,

    /// Metadata about the interface
    metadata: HashMap<String, String>,
}

impl QisSeleneHeliosInterface {
    /// Create a new Helios interface
    pub fn new() -> Self {
        Self {
            executable_path: None,
            program: Vec::new(),
            format: ProgramFormat::QisBitcode,
            operations: OperationList::new(),
            metadata: HashMap::new(),
        }
    }

    /// Link the program with Helios interface to create an executable
    fn create_executable(&mut self) -> Result<PathBuf, PecosError> {
        // Get the Helios library path from build script
        let helios_lib_path = std::env::var("HELIOS_LIB_PATH")
            .map_err(|_| PecosError::Generic(
                "HELIOS_LIB_PATH not set. Helios interface library not found.".to_string()
            ))?;

        // Create temporary files for the program and executable
        let mut program_file = NamedTempFile::new()
            .map_err(|e| PecosError::Generic(format!("Failed to create temp file: {}", e)))?;

        // Write the program based on format
        match self.format {
            ProgramFormat::QisBitcode | ProgramFormat::LlvmBitcode => {
                // Write bitcode directly
                program_file.write_all(&self.program)
                    .map_err(|e| PecosError::Generic(format!("Failed to write bitcode: {}", e)))?;
            }
            ProgramFormat::LlvmIrText => {
                // Need to convert text to bitcode first using llvm-as
                let ir_str = std::str::from_utf8(&self.program)
                    .map_err(|e| PecosError::Generic(format!("Invalid UTF-8 in LLVM IR: {}", e)))?;

                // Write IR to temp file
                program_file.write_all(ir_str.as_bytes())
                    .map_err(|e| PecosError::Generic(format!("Failed to write LLVM IR: {}", e)))?;

                // Convert to bitcode using llvm-as
                let bitcode_file = NamedTempFile::new()
                    .map_err(|e| PecosError::Generic(format!("Failed to create bitcode file: {}", e)))?;

                let output = Command::new("llvm-as")
                    .arg("-o")
                    .arg(bitcode_file.path())
                    .arg(program_file.path())
                    .output()
                    .map_err(|e| PecosError::Generic(format!("Failed to run llvm-as: {}", e)))?;

                if !output.status.success() {
                    return Err(PecosError::Generic(format!(
                        "llvm-as failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }

                // Update to use the bitcode file
                program_file = bitcode_file;
            }
            ProgramFormat::HugrBytes => {
                return Err(PecosError::Generic(
                    "HUGR bytes should be compiled to LLVM first".to_string()
                ));
            }
        }

        // Create executable path
        let exe_file = NamedTempFile::new()
            .map_err(|e| PecosError::Generic(format!("Failed to create exe file: {}", e)))?;
        let exe_path = exe_file.path().to_path_buf();

        // Link using clang or ld
        // We need to link:
        // 1. The program bitcode
        // 2. The Helios interface library (.a)
        // 3. Any required system libraries

        let output = Command::new("clang")
            .arg("-o")
            .arg(&exe_path)
            .arg(program_file.path())
            .arg(&helios_lib_path)
            .arg("-lm")  // Math library
            .arg("-lpthread")  // Threading
            .output()
            .map_err(|e| PecosError::Generic(format!("Failed to run clang: {}", e)))?;

        if !output.status.success() {
            return Err(PecosError::Generic(format!(
                "Linking failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Keep the temporary file alive by storing its path
        let exe_path = exe_file.into_temp_path().to_path_buf();
        self.executable_path = Some(exe_path.clone());

        self.metadata.insert("executable_path".to_string(), exe_path.display().to_string());
        self.metadata.insert("helios_lib".to_string(), helios_lib_path);

        Ok(exe_path)
    }

    /// Execute the linked program to collect operations
    fn execute_program(&mut self) -> Result<(), PecosError> {
        let exe_path = self.executable_path.as_ref()
            .ok_or_else(|| PecosError::Generic("No executable created".to_string()))?;

        // Set up FFI context to intercept function calls
        // This is where we'd set up the thread-local interface to capture operations
        pecos_qis_interface::reset_interface();

        // Run the executable
        // Note: In a real implementation, we might want to run this in a subprocess
        // or use dynamic loading to run it in-process
        let output = Command::new(exe_path)
            .output()
            .map_err(|e| PecosError::Generic(format!("Failed to run executable: {}", e)))?;

        if !output.status.success() {
            return Err(PecosError::Generic(format!(
                "Executable failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Collect the operations that were captured via FFI
        pecos_qis_interface::with_interface(|interface| {
            self.operations = interface.clone();
        });

        Ok(())
    }
}

impl QisInterface for QisSeleneHeliosInterface {
    fn load_program(&mut self, program_bytes: &[u8], format: ProgramFormat) -> Result<(), PecosError> {
        // Check if Helios can handle this format
        match format {
            ProgramFormat::QisBitcode | ProgramFormat::LlvmBitcode | ProgramFormat::LlvmIrText => {
                self.program = program_bytes.to_vec();
                self.format = format;

                // Create the executable by linking
                self.create_executable()?;

                Ok(())
            }
            ProgramFormat::HugrBytes => {
                // Would need to compile HUGR to LLVM first
                Err(PecosError::Generic(
                    "Helios interface requires HUGR to be compiled to LLVM first".to_string()
                ))
            }
        }
    }

    fn collect_operations(&mut self) -> Result<OperationList, PecosError> {
        // Reset operations
        self.operations = OperationList::new();

        // Execute the program in collection mode
        self.execute_program()?;

        Ok(self.operations.clone())
    }

    fn execute_with_measurements(
        &mut self,
        measurements: HashMap<usize, bool>,
    ) -> Result<OperationList, PecosError> {
        // Reset operations
        self.operations = OperationList::new();

        // Set up measurements in the measurement manager (same as JIT interface)
        pecos_qis_interface::runtime::with_measurement_manager_mut(|manager| {
            manager.reset();
            manager.enable_simulation_mode();
            for (id, value) in measurements {
                manager.set_measurement_result(id as i64, value);
            }
        });

        // Execute with the measurements
        self.execute_program()?;

        Ok(self.operations.clone())
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }

    fn name(&self) -> &'static str {
        "Selene Helios"
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.operations = OperationList::new();
        pecos_qis_interface::reset_interface();
        Ok(())
    }
}