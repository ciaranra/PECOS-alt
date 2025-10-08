//! Helios interface executor
//!
//! This module implements the `QisInterface` trait for Selene's Helios compiler.

use libloading::{Library, Symbol};
use pecos_qis_core::qis_interface::{InterfaceError, ProgramFormat, QisInterface};
use pecos_qis_ffi::OperationCollector;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

// FFI function type aliases for dlopen symbol lookup
type ResetInterfaceFn = unsafe extern "C" fn();
type GetOperationsFn = unsafe extern "C" fn() -> *mut OperationCollector;
type SetMeasurementsFn = unsafe extern "C" fn(*const (usize, bool), usize);

/// Helios interface implementation
///
/// This interface:
/// 1. Links program bitcode with libhelios.a to create an executable
/// 2. Loads the executable in-process using dlopen (libloading)
/// 3. Calls `qmain()` to execute the program
/// 4. Collects operations via thread-local storage in the PECOS shim
pub struct QisHeliosInterface {
    /// Path to the linked executable (if created)
    executable_path: Option<PathBuf>,

    /// The program bytes
    program: Vec<u8>,

    /// The program format
    format: ProgramFormat,

    /// Metadata about the interface
    metadata: HashMap<String, String>,

    /// Keep temporary files alive (`TempPath` auto-deletes when dropped)
    temp_files: Vec<tempfile::TempPath>,
}

impl QisHeliosInterface {
    /// Create a new Helios interface
    #[must_use]
    pub fn new() -> Self {
        Self {
            executable_path: None,
            program: Vec::new(),
            format: ProgramFormat::QisBitcode,
            metadata: HashMap::new(),
            temp_files: Vec::new(),
        }
    }

    /// Link the program with Helios interface to create a shared library
    fn create_shared_library(&mut self) -> Result<PathBuf, InterfaceError> {
        // Get the Helios library path from environment
        let helios_lib_path = std::env::var("HELIOS_LIB_PATH").map_err(|_| {
            InterfaceError::LoadError(
                "HELIOS_LIB_PATH not set. Set it to point to libhelios.a".to_string(),
            )
        })?;

        // Create temporary files for the program
        let mut program_file = NamedTempFile::new()
            .map_err(|e| InterfaceError::LoadError(format!("Failed to create temp file: {e}")))?;

        // Get the program file path that we'll pass to clang
        // We need to keep the TempPath alive until after clang finishes
        let program_temp_path = match self.format {
            ProgramFormat::QisBitcode | ProgramFormat::LlvmBitcode => {
                // Write bitcode directly
                program_file.write_all(&self.program).map_err(|e| {
                    InterfaceError::LoadError(format!("Failed to write bitcode: {e}"))
                })?;
                program_file.into_temp_path()
            }
            ProgramFormat::LlvmIrText => {
                // Convert text to bitcode using llvm-as
                program_file.write_all(&self.program).map_err(|e| {
                    InterfaceError::LoadError(format!("Failed to write LLVM IR: {e}"))
                })?;
                program_file.flush().map_err(|e| {
                    InterfaceError::LoadError(format!("Failed to flush LLVM IR: {e}"))
                })?;

                let ir_path = program_file.into_temp_path();

                let bitcode_file = NamedTempFile::with_suffix(".bc").map_err(|e| {
                    InterfaceError::LoadError(format!("Failed to create bitcode file: {e}"))
                })?;

                let output = Command::new("llvm-as")
                    .arg("-o")
                    .arg(bitcode_file.path())
                    .arg(&ir_path)
                    .output()
                    .map_err(|e| {
                        InterfaceError::LoadError(format!("Failed to run llvm-as: {e}"))
                    })?;

                if !output.status.success() {
                    return Err(InterfaceError::LoadError(format!(
                        "llvm-as failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }

                // Convert bitcode file to persistent path and keep it alive
                bitcode_file.into_temp_path()
            }
            ProgramFormat::HugrBytes => {
                return Err(InterfaceError::InvalidFormat(
                    "HUGR bytes should be compiled to LLVM first".to_string(),
                ));
            }
        };

        // Create shared library path (.so)
        let so_file = NamedTempFile::with_suffix(".so")
            .map_err(|e| InterfaceError::LoadError(format!("Failed to create .so file: {e}")))?;

        // Link using clang to create a shared library:
        // program.bc + libhelios.a → program.so
        // The resulting .so will:
        // - Export qmain symbol
        // - Have undefined selene_* symbols (to be resolved by our shim at runtime)
        let output = Command::new("clang")
            .arg("-shared") // Create shared library instead of executable
            .arg("-fPIC") // Position independent code
            .arg("-o")
            .arg(so_file.path())
            .arg(&program_temp_path)
            .arg(&helios_lib_path)
            .arg("-lm")
            .arg("-lpthread")
            .arg("-ldl")
            .output()
            .map_err(|e| InterfaceError::LoadError(format!("Failed to run clang: {e}")))?;

        if !output.status.success() {
            return Err(InterfaceError::LoadError(format!(
                "Linking failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Keep the temporary files alive by storing the TempPaths
        let so_temp_path = so_file.into_temp_path();
        let so_path = so_temp_path.to_path_buf();

        // Store both the program bitcode and the .so file to keep them alive
        self.temp_files.push(program_temp_path);
        self.temp_files.push(so_temp_path);

        self.executable_path = Some(so_path.clone());

        self.metadata
            .insert("library_path".to_string(), so_path.display().to_string());
        self.metadata
            .insert("helios_lib".to_string(), helios_lib_path);

        Ok(so_path)
    }

    /// Execute the program by loading it in-process and calling `qmain()`
    fn execute_program(&mut self) -> Result<OperationCollector, InterfaceError> {
        let so_path = self.executable_path.as_ref().ok_or_else(|| {
            InterfaceError::ExecutionError("No shared library created".to_string())
        })?;

        // Get the path to our PECOS selene shim library
        let shim_path = crate::shim::get_shim_library_path().ok_or_else(|| {
            InterfaceError::ExecutionError(
                "PECOS selene shim library not found - build script may have failed".to_string(),
            )
        })?;

        // Architecture note:
        // The __quantum__* FFI symbols are in libpecos_qis_selene.so (Rust cdylib).
        // The selene_* symbols are in libpecos_selene.so (C shim).
        //
        // Symbol resolution chain:
        //   qmain() → ___qalloc() → selene_qalloc() → __quantum__rt__qubit_allocate()
        //
        // We need to load libs in order with RTLD_GLOBAL so symbols are visible:
        //   1. libpecos_qis_selene.so (provides __quantum__*)
        //   2. libpecos_selene.so (provides selene_*, calls __quantum__*)
        //   3. program.so (provides qmain, calls selene_*)

        // Step 1: Load libpecos_qis_selene.so (the Rust cdylib with __quantum__* symbols)
        // This is needed when running from test binary which uses rlib, not cdylib
        let pecos_qis_lib_path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
            .map(|dir| dir.join("libpecos_qis_selene.so"))
            .ok_or_else(|| {
                InterfaceError::ExecutionError("Failed to find libpecos_qis_selene.so".to_string())
            })?;

        // Load with RTLD_GLOBAL first using unix-specific API
        let _pecos_qis_lib_global = unsafe {
            libloading::os::unix::Library::open(
                Some(&pecos_qis_lib_path),
                libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
            )
            .map_err(|e| {
                InterfaceError::ExecutionError(format!(
                    "Failed to load PECOS QIS cdylib with RTLD_GLOBAL: {e}"
                ))
            })?
        };

        // Now load again with portable API for symbol lookup
        let pecos_qis_lib = unsafe {
            Library::new(&pecos_qis_lib_path).map_err(|e| {
                InterfaceError::ExecutionError(format!(
                    "Failed to load PECOS QIS cdylib for symbol lookup: {e}"
                ))
            })?
        };

        // Get the reset_interface function from the cdylib
        // This ensures we reset the cdylib's thread-local storage, not the rlib's
        let reset_interface_fn: Symbol<ResetInterfaceFn> = unsafe {
            pecos_qis_lib
                .get(b"pecos_qis_reset_interface\0")
                .map_err(|e| {
                    InterfaceError::ExecutionError(format!(
                        "Failed to find pecos_qis_reset_interface symbol: {e}"
                    ))
                })?
        };

        // Reset the cdylib's thread-local storage
        unsafe { reset_interface_fn() };

        // Step 2: Load our PECOS C shim with RTLD_GLOBAL
        // This makes the selene_* symbols available for program.so to find
        // SAFETY: We're loading our own shim library that we compiled
        let shim_lib = unsafe {
            libloading::os::unix::Library::open(
                Some(&shim_path),
                libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
            )
            .map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load PECOS C shim library: {e}"))
            })?
        };

        // Step 3: Load the program.so
        // It will find selene_* symbols from our shim (loaded with RTLD_GLOBAL above)
        // SAFETY: We're loading a shared library we just created from trusted bitcode
        let program_lib = unsafe {
            Library::new(so_path).map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load program library: {e}"))
            })?
        };

        // Step 4: Get the qmain function symbol
        // qmain has signature: extern "C" fn(u64) -> u64
        let qmain: Symbol<extern "C" fn(u64) -> u64> = unsafe {
            program_lib.get(b"qmain\0").map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to find qmain symbol: {e}"))
            })?
        };

        // Step 5: Call qmain(0) to execute the program
        // The call chain will be:
        //   qmain(0) [user code in program.so]
        //   → ___qalloc() [from libhelios.a linked into program.so]
        //   → selene_qalloc() [from libpecos_selene.so C shim]
        //   → __quantum__rt__qubit_allocate() [from libpecos_qis_selene.so loaded with RTLD_GLOBAL]
        //   → pecos_qis_ffi::with_interface() [thread-local in current process]
        let _result = qmain(0);

        // Step 6: Collect the operations from the cdylib's thread-local storage
        let get_operations_fn: libloading::Symbol<GetOperationsFn> = unsafe {
            pecos_qis_lib
                .get(b"pecos_qis_get_operations\0")
                .map_err(|e| {
                    InterfaceError::ExecutionError(format!(
                        "Failed to find pecos_qis_get_operations symbol: {e}"
                    ))
                })?
        };

        let operations_ptr = unsafe { get_operations_fn() };
        let operations = unsafe { Box::from_raw(operations_ptr) };
        let operations = *operations;

        // Keep libraries loaded until we're done
        drop(program_lib);
        drop(shim_lib);
        drop(pecos_qis_lib);

        Ok(operations)
    }
}

impl Default for QisHeliosInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl QisInterface for QisHeliosInterface {
    fn load_program(
        &mut self,
        program_bytes: &[u8],
        format: ProgramFormat,
    ) -> Result<(), InterfaceError> {
        // Check if Helios can handle this format
        match format {
            ProgramFormat::QisBitcode | ProgramFormat::LlvmBitcode | ProgramFormat::LlvmIrText => {
                self.program = program_bytes.to_vec();
                self.format = format;

                // Create the shared library by linking
                self.create_shared_library()?;

                Ok(())
            }
            ProgramFormat::HugrBytes => Err(InterfaceError::InvalidFormat(
                "Helios interface requires HUGR to be compiled to LLVM first".to_string(),
            )),
        }
    }

    fn collect_operations(&mut self) -> Result<OperationCollector, InterfaceError> {
        // Execute the program and collect operations
        self.execute_program()
    }

    fn execute_with_measurements(
        &mut self,
        measurements: HashMap<usize, bool>,
    ) -> Result<OperationCollector, InterfaceError> {
        let so_path = self.executable_path.as_ref().ok_or_else(|| {
            InterfaceError::ExecutionError("No shared library created".to_string())
        })?;

        // Load the PECOS QIS cdylib to access its functions
        let pecos_qis_lib_path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
            .map(|dir| dir.join("libpecos_qis_selene.so"))
            .ok_or_else(|| {
                InterfaceError::ExecutionError("Failed to find libpecos_qis_selene.so".to_string())
            })?;

        let _pecos_qis_lib_global = unsafe {
            libloading::os::unix::Library::open(
                Some(&pecos_qis_lib_path),
                libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
            )
            .map_err(|e| {
                InterfaceError::ExecutionError(format!(
                    "Failed to load PECOS QIS cdylib with RTLD_GLOBAL: {e}"
                ))
            })?
        };

        let pecos_qis_lib = unsafe {
            Library::new(&pecos_qis_lib_path).map_err(|e| {
                InterfaceError::ExecutionError(format!(
                    "Failed to load PECOS QIS cdylib for symbol lookup: {e}"
                ))
            })?
        };

        // Set measurements in the cdylib's thread-local storage
        let set_measurements_fn: Symbol<SetMeasurementsFn> = unsafe {
            pecos_qis_lib
                .get(b"pecos_qis_set_measurements\0")
                .map_err(|e| {
                    InterfaceError::ExecutionError(format!(
                        "Failed to find pecos_qis_set_measurements symbol: {e}"
                    ))
                })?
        };

        // Convert measurements HashMap to array for FFI
        let measurements_vec: Vec<(usize, bool)> = measurements.into_iter().collect();
        unsafe {
            set_measurements_fn(measurements_vec.as_ptr(), measurements_vec.len());
        }

        // Now execute the program with the measurements set
        // Reuse most of execute_program logic but we've already loaded pecos_qis_lib
        let shim_path = crate::shim::get_shim_library_path().ok_or_else(|| {
            InterfaceError::ExecutionError(
                "PECOS selene shim library not found - build script may have failed".to_string(),
            )
        })?;

        let shim_lib = unsafe {
            libloading::os::unix::Library::open(
                Some(&shim_path),
                libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
            )
            .map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load PECOS C shim library: {e}"))
            })?
        };

        let program_lib = unsafe {
            Library::new(so_path).map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load program library: {e}"))
            })?
        };

        let qmain: Symbol<extern "C" fn(u64) -> u64> = unsafe {
            program_lib.get(b"qmain\0").map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to find qmain symbol: {e}"))
            })?
        };

        let _result = qmain(0);

        // Collect operations from cdylib
        let get_operations_fn: Symbol<GetOperationsFn> = unsafe {
            pecos_qis_lib
                .get(b"pecos_qis_get_operations\0")
                .map_err(|e| {
                    InterfaceError::ExecutionError(format!(
                        "Failed to find pecos_qis_get_operations symbol: {e}"
                    ))
                })?
        };

        let operations_ptr = unsafe { get_operations_fn() };
        let operations = unsafe { Box::from_raw(operations_ptr) };
        let operations = *operations;

        drop(program_lib);
        drop(shim_lib);
        drop(pecos_qis_lib);

        Ok(operations)
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }

    fn name(&self) -> &'static str {
        "Helios (dlopen)"
    }

    fn reset(&mut self) -> Result<(), InterfaceError> {
        pecos_qis_ffi::reset_interface();
        Ok(())
    }
}
