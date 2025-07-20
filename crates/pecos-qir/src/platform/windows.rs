//! Windows-specific implementations for QIR compilation

use log::debug;
use pecos_core::errors::PecosError;
use std::fs;
use std::path::Path;
use std::process::Command;

#[path = "windows_stub_gen.rs"]
mod stub_gen;

/// Handle Windows-specific QIR compilation
pub struct WindowsCompiler;

impl WindowsCompiler {
    /// Compile QIR file to object file using clang
    ///
    /// Windows does not typically include llc.exe in standard LLVM installations
    /// so we use clang directly to compile the QIR file to an object file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - QIR file cannot be read
    /// - Temporary file cannot be written
    /// - Clang execution fails
    /// - Object file is not created at expected path
    pub fn compile_to_object_file(
        qir_file: &Path,
        object_file: &Path,
        clang_path: &Path,
        handle_command_error: impl Fn(
            std::io::Result<std::process::Output>,
            &str,
        ) -> Result<std::process::Output, PecosError>,
        handle_command_status: impl Fn(&std::process::Output, &str) -> Result<(), PecosError>,
    ) -> Result<(), PecosError> {
        debug!("QIR Compiler: Compiling QIR to object file with Windows-specific logic");

        // Read and modify QIR content to add Windows export attribute
        let mut qir_content = fs::read_to_string(qir_file).map_err(PecosError::IO)?;

        // Add dllexport attribute to main function
        qir_content = qir_content.replace(
            "define void @main() #0 {",
            "define dllexport void @main() #0 {",
        );

        // Create a temporary file in the parent directory of the object file
        let parent_dir = object_file.parent().unwrap_or(Path::new("."));
        let temp_qir_file = parent_dir.join("temp_qir.ll");

        fs::write(&temp_qir_file, qir_content).map_err(PecosError::IO)?;

        debug!(
            "QIR Compiler: Using clang at {} to compile LLVM IR directly",
            clang_path.display()
        );

        // Compile with clang - note we're using clang directly instead of llc
        // since many Windows LLVM installations don't include llc.exe
        let result = Command::new(clang_path)
            .args(["-c", "-O2", "-emit-llvm", "-o"]) // Add -emit-llvm flag to ensure proper LLVM IR processing
            .arg(object_file)
            .arg(&temp_qir_file)
            .output();

        // Clean up temporary file regardless of compilation result
        let _ = fs::remove_file(temp_qir_file);

        // Check compilation result
        let output = handle_command_error(result, "Failed to execute clang")?;
        handle_command_status(&output, "clang")?;

        // Verify output file exists
        if !object_file.exists() {
            return Err(PecosError::Processing(format!(
                "QIR compilation failed: Object file was not created at the expected path: {}",
                object_file.display()
            )));
        }

        debug!(
            "QIR Compiler: Successfully compiled QIR to object file with Windows-specific logic"
        );

        Ok(())
    }

    /// Link object file and runtime library into a shared library
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Definition file cannot be written
    /// - C stub file cannot be written
    /// - Object compilation of stub fails
    /// - Library linking fails
    /// - Library file is not created at expected path
    pub fn link_shared_library(
        object_file: &Path,
        _rust_runtime_lib: &Path, // Unused but kept for API compatibility
        library_file: &Path,
        clang_path: &Path,
        handle_command_error: impl Fn(
            std::io::Result<std::process::Output>,
            &str,
        ) -> Result<std::process::Output, PecosError>,
        handle_command_status: impl Fn(&std::process::Output, &str) -> Result<(), PecosError>,
    ) -> Result<(), PecosError> {
        debug!("QIR Compiler: Linking with Windows-specific logic");

        let parent_dir = library_file.parent().unwrap_or(Path::new("."));

        // Create temporary files
        let def_file_path = parent_dir.join("qir_runtime.def");
        let stub_c_path = parent_dir.join("qir_runtime_stub.c");
        let stub_obj_path = parent_dir.join("qir_runtime_stub.o");

        // Write DEF file for exporting symbols
        fs::write(&def_file_path, Self::generate_def_file())
            .map_err(|e| PecosError::Processing(format!("Failed to write DEF file: {e}")))?;

        // Write C stub implementation
        fs::write(&stub_c_path, Self::generate_c_stub())
            .map_err(|e| PecosError::Processing(format!("Failed to write stub .c file: {e}")))?;

        // Compile the C stub
        debug!("QIR Compiler: Compiling C stub file for QIR runtime on Windows");

        let result = Command::new(clang_path)
            .args(["-c", "-O2", "-fms-extensions", "-o"])
            .arg(&stub_obj_path)
            .arg(&stub_c_path)
            .output();

        let output = handle_command_error(result, "Failed to compile stub C file")?;
        handle_command_status(&output, "clang (stub compilation)")?;

        // Link everything together
        debug!("QIR Compiler: Linking QIR object file with C stubs and system libraries");

        let result = Command::new(clang_path)
            .args(["-shared", "-o"])
            .arg(library_file)
            .arg(object_file)
            .arg(&stub_obj_path)
            .arg("-fuse-ld=lld")
            .arg(format!("-Wl,/DEF:{}", def_file_path.to_string_lossy()))
            .args(Self::system_libraries())
            .output();

        // Clean up temporary files
        for file in [def_file_path, stub_c_path, stub_obj_path] {
            let _ = fs::remove_file(file);
        }

        // Check linking result
        let output = handle_command_error(result, "Failed to link QIR shared library")?;
        handle_command_status(&output, "clang (linking)")?;

        // Verify the library exists
        if !library_file.exists() {
            return Err(PecosError::Processing(format!(
                "Library file was not created at the expected path: {}",
                library_file.display()
            )));
        }

        debug!("QIR Compiler: Successfully linked with Windows-specific logic");

        Ok(())
    }

    /// Generate DEF file content dynamically
    fn generate_def_file() -> String {
        stub_gen::generate_def_file()
    }

    /// Generate C stub implementation dynamically
    fn generate_c_stub() -> String {
        stub_gen::generate_c_stub()
    }

    /// Get Windows system libraries for linking
    fn system_libraries() -> &'static [&'static str] {
        &[
            "-lws2_32",   // Windows Socket API
            "-lkernel32", // Windows kernel functions
            "-ladvapi32", // Advanced Windows API
            "-luserenv",  // User environment functions
            "-lntdll",    // NT API
            "-lmsvcrt",   // C runtime
        ]
    }
}
