//! Helios interface executor
//!
//! This module implements the `QisInterface` trait for Selene's Helios compiler.

use libloading::{Library, Symbol};
use pecos_qis_core::qis_interface::{InterfaceError, ProgramFormat, QisInterface};
use pecos_qis_ffi_types::OperationCollector;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

// FFI function type aliases for dlopen symbol lookup
type ResetInterfaceFn = unsafe extern "C" fn();
type GetOperationsFn = unsafe extern "C" fn() -> *mut OperationCollector;

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
        // Get the Helios library path from environment, or use compile-time default
        let helios_lib_path = std::env::var("HELIOS_LIB_PATH")
            .unwrap_or_else(|_| {
                // Fall back to compile-time path set by build.rs
                env!("HELIOS_LIB_PATH").to_string()
            });

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
                eprintln!("[HELIOS] Converting LLVM IR text to bitcode using llvm-as...");
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

                eprintln!("[HELIOS] About to spawn llvm-as subprocess...");
                let output = Command::new("llvm-as")
                    .arg("-o")
                    .arg(bitcode_file.path())
                    .arg(&ir_path)
                    .output()
                    .map_err(|e| {
                        InterfaceError::LoadError(format!("Failed to run llvm-as: {e}"))
                    })?;

                eprintln!("[HELIOS] llvm-as subprocess completed!");

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
        eprintln!("[HELIOS] Creating .so temp file...");
        let so_file = NamedTempFile::with_suffix(".so")
            .map_err(|e| InterfaceError::LoadError(format!("Failed to create .so file: {e}")))?;
        eprintln!("[HELIOS] Created .so temp file: {:?}", so_file.path());

        // Link using clang to create a shared library:
        // program.bc + libhelios.a → program.so
        // The resulting .so will:
        // - Export qmain symbol
        // - Have undefined selene_* symbols (to be resolved by our shim at runtime)
        eprintln!("[HELIOS] About to spawn clang subprocess for linking...");
        eprintln!("[HELIOS] Linking: {} + {} -> {}", program_temp_path.display(), helios_lib_path, so_file.path().display());
        let output = Command::new("clang")
            .arg("-shared") // Create shared library instead of executable
            .arg("-fPIC") // Position independent code
            .arg("-Wl,--export-dynamic") // Export all symbols including qmain
            .arg("-o")
            .arg(so_file.path())
            .arg(&program_temp_path)
            .arg(&helios_lib_path)
            .arg("-lm")
            .arg("-lpthread")
            .arg("-ldl")
            .output()
            .map_err(|e| InterfaceError::LoadError(format!("Failed to run clang: {e}")))?;

        eprintln!("[HELIOS] clang subprocess completed!");

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
        // The __quantum__* FFI symbols are in libpecos_qis_ffi.so (Rust cdylib from pecos-qis-ffi).
        // The selene_* symbols are in libpecos_selene.so (C shim).
        //
        // Symbol resolution chain:
        //   qmain() → ___qalloc() → selene_qalloc() → __quantum__rt__qubit_allocate()
        //
        // We need to load libs in order with RTLD_GLOBAL so symbols are visible:
        //   1. libpecos_qis_ffi.so (provides __quantum__*)
        //   2. libpecos_selene.so (provides selene_*, calls __quantum__*)
        //   3. program.so (provides qmain, calls selene_*)

        // Step 1: Find and load libpecos_qis_ffi.so with RTLD_GLOBAL
        // This provides the __quantum__* symbols for the shim to resolve
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
            .ok_or_else(|| {
                InterfaceError::ExecutionError("Failed to determine executable directory".to_string())
            })?;

        let mut candidate_paths = vec![
            exe_dir.join("libpecos_qis_ffi.so"),
            exe_dir.join("deps/libpecos_qis_ffi.so"),
        ];

        if let Some(parent) = exe_dir.parent() {
            candidate_paths.push(parent.join("libpecos_qis_ffi.so"));
            candidate_paths.push(parent.join("deps/libpecos_qis_ffi.so"));
        }

        if let Ok(current_dir) = std::env::current_dir() {
            candidate_paths.push(current_dir.join("target/debug/libpecos_qis_ffi.so"));
            candidate_paths.push(current_dir.join("target/debug/deps/libpecos_qis_ffi.so"));
            candidate_paths.push(current_dir.join("target/release/libpecos_qis_ffi.so"));
            candidate_paths.push(current_dir.join("target/release/deps/libpecos_qis_ffi.so"));

            // Search up the directory tree for workspace root (when running from Python)
            let mut search_dir = current_dir.as_path();
            for _ in 0..5 {  // Search up to 5 levels
                if let Some(parent) = search_dir.parent() {
                    candidate_paths.push(parent.join("target/debug/libpecos_qis_ffi.so"));
                    candidate_paths.push(parent.join("target/debug/deps/libpecos_qis_ffi.so"));
                    candidate_paths.push(parent.join("target/release/libpecos_qis_ffi.so"));
                    candidate_paths.push(parent.join("target/release/deps/libpecos_qis_ffi.so"));
                    search_dir = parent;
                } else {
                    break;
                }
            }
        }

        let pecos_qis_lib_path = candidate_paths
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| {
                InterfaceError::ExecutionError(format!(
                    "Failed to find libpecos_qis_ffi.so. Searched in: {:?}",
                    candidate_paths
                ))
            })?
            .clone();

        // Load with RTLD_GLOBAL so symbols are available for the shim
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

        // Load again for symbol lookup
        let pecos_qis_lib = unsafe {
            Library::new(&pecos_qis_lib_path).map_err(|e| {
                InterfaceError::ExecutionError(format!(
                    "Failed to load PECOS QIS cdylib for symbol lookup: {e}"
                ))
            })?
        };

        // Step 2: Reset the QIS interface via the cdylib
        // IMPORTANT: We call the cdylib's version to ensure we're using the same thread-local
        // storage instance that the shim will use
        let reset_fn: Symbol<ResetInterfaceFn> = unsafe {
            pecos_qis_lib.get(b"pecos_qis_reset_interface\0").map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to find reset function: {e}"))
            })?
        };
        unsafe { reset_fn() };

        // Step 3: Load our PECOS C shim with RTLD_GLOBAL
        // The shim has undefined __quantum__* symbols that will resolve to the cdylib
        // SAFETY: We're loading our own shim library that we compiled
        let _shim_lib_global = unsafe {
            libloading::os::unix::Library::open(
                Some(&shim_path),
                libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
            )
            .map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load PECOS C shim library: {e}"))
            })?
        };

        // Also load the shim for symbol lookup (needs to be kept alive)
        let shim_lib = unsafe {
            Library::new(&shim_path).map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load shim for symbol lookup: {e}"))
            })?
        };

        // Step 3: Load the program.so with RTLD_GLOBAL so it can resolve selene_* symbols
        // It will find selene_* symbols from our shim (loaded with RTLD_GLOBAL above)
        // SAFETY: We're loading a shared library we just created from trusted bitcode
        eprintln!("[HELIOS] Loading program.so with RTLD_GLOBAL...");
        let _program_lib_global = unsafe {
            libloading::os::unix::Library::open(
                Some(so_path),
                libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
            )
            .map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load program library with RTLD_GLOBAL: {e}"))
            })?
        };

        // Also load for symbol lookup
        let program_lib = unsafe {
            Library::new(so_path).map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to load program library: {e}"))
            })?
        };

        // Step 4: Get the qmain or main function symbol
        // Try qmain first (used by Guppy/HUGR-LLVM), then fall back to main (used by legacy LLVM)
        let qmain_fn: Symbol<extern "C" fn(u64) -> u64> = unsafe {
            program_lib.get(b"qmain\0")
                .or_else(|_| program_lib.get(b"main\0"))
                .map_err(|e| {
                    InterfaceError::ExecutionError(format!("Failed to find qmain or main entry point: {e}"))
                })?
        };

        // Step 5: Get our shim's wrapper function that sets up setjmp before calling qmain
        // This is CRITICAL for proper exception handling - without setjmp, longjmp will crash
        type CallQmainFn = unsafe extern "C" fn(extern "C" fn(u64) -> u64) -> u64;
        let call_with_setjmp: Symbol<CallQmainFn> = unsafe {
            shim_lib.get(b"pecos_call_qmain_with_setjmp\0").map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to find setjmp wrapper: {e}"))
            })?
        };

        // Step 6: Call qmain via our setjmp wrapper
        // The call chain will be:
        //   pecos_call_qmain_with_setjmp(qmain) [from our shim]
        //   → setjmp(user_program_jmpbuf) [saves stack state for longjmp]
        //   → qmain(0) [user code in program.so]
        //   → ___qalloc() [from libhelios.a linked into program.so]
        //   → selene_qalloc() [from libpecos_selene.so C shim]
        //   → __quantum__rt__qubit_allocate() [from libpecos_qis_ffi.so]
        //   → pecos_qis_ffi::with_interface() [thread-local in current process]
        // If an error occurs:
        //   → longjmp(user_program_jmpbuf, error_code) [jumps back to setjmp]
        //   → wrapper catches error and returns error code
        eprintln!("[HELIOS] About to call qmain via setjmp wrapper...");
        let result = unsafe { call_with_setjmp(*qmain_fn) };
        if result != 0 {
            return Err(InterfaceError::ExecutionError(format!(
                "qmain returned error code: {result}"
            )));
        }
        eprintln!("[HELIOS] qmain executed successfully!");

        // Step 6: Collect the operations from thread-local storage via the cdylib
        // IMPORTANT: We call the cdylib's version to get the operations from the same
        // thread-local storage instance that the shim used
        let get_ops_fn: Symbol<GetOperationsFn> = unsafe {
            pecos_qis_lib.get(b"pecos_qis_get_operations\0").map_err(|e| {
                InterfaceError::ExecutionError(format!("Failed to find get_operations function: {e}"))
            })?
        };
        let operations_ptr = unsafe { get_ops_fn() };
        let operations = unsafe { Box::from_raw(operations_ptr) };
        let operations = *operations;

        // Keep libraries loaded until we're done
        drop(program_lib);
        drop(_program_lib_global);
        drop(shim_lib);
        drop(_shim_lib_global);
        drop(pecos_qis_lib);
        drop(_pecos_qis_lib_global);

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
        _measurements: HashMap<usize, bool>,
    ) -> Result<OperationCollector, InterfaceError> {
        // TODO: Implement measurement support by pre-populating results via cdylib
        // For now, just execute the program normally
        self.execute_program()
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }

    fn name(&self) -> &'static str {
        "Helios (dlopen)"
    }

    fn reset(&mut self) -> Result<(), InterfaceError> {
        // Reset is not needed for this interface - it happens at the start of execute_program
        Ok(())
    }
}
