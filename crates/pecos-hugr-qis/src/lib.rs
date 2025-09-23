/*!
HUGR to QIS (Quantum Instruction Set) Compiler for PECOS

This crate compiles HUGR (Hierarchical Unified Graph Representation) to
QIS-compatible LLVM IR for execution on PECOS quantum simulators.

# Current Features

## HUGR to QIS LLVM Compilation

```text
HUGR JSON/Bytes → [pecos-hugr-qis] → QIS LLVM IR String/File
```

The generated LLVM IR can then be executed by any compatible execution engine (e.g., pecos-llvm-runtime).

# Example

```rust,no_run
use pecos_hugr_qis::{HugrCompiler, HugrCompilerConfig};

// Compile HUGR bytes to LLVM IR string
let hugr_bytes = b"example HUGR data";
let compiler = HugrCompiler::new();
let llvm_ir = compiler.compile_hugr_bytes_to_string(hugr_bytes)?;

// Or compile HUGR file to LLVM IR file
let config = HugrCompilerConfig {
    output_path: Some("output.ll".into()),
};
let compiler = HugrCompiler::with_config(config);
let llvm_path = compiler.compile_hugr("input.hugr")?;
# Ok::<(), pecos_core::errors::PecosError>(())
```
*/

pub mod array;
pub mod compiler;

// Re-export main functions
pub use compiler::compile_hugr_bytes_to_string;

// Convenience functions
use pecos_core::errors::PecosError;
use std::path::{Path, PathBuf};

/// Configuration for HUGR compilation (compatibility wrapper)
#[derive(Debug, Clone, Default)]
pub struct HugrCompilerConfig {
    /// Output path for the compiled LLVM IR
    pub output_path: Option<PathBuf>,
}

/// HUGR compiler (compatibility wrapper)
#[derive(Debug, Clone, Default)]
pub struct HugrCompiler {
    config: HugrCompilerConfig,
}

impl HugrCompiler {
    /// Create a new compiler with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new compiler with specified configuration
    pub fn with_config(config: HugrCompilerConfig) -> Self {
        Self { config }
    }

    /// Compile HUGR bytes to LLVM IR string
    pub fn compile_hugr_bytes_to_string(&self, hugr_bytes: &[u8]) -> Result<String, PecosError> {
        compile_hugr_bytes_to_string(hugr_bytes)
    }

    /// Compile HUGR bytes to file
    pub fn compile_hugr_bytes(&self, hugr_bytes: &[u8], output_path: &Path) -> Result<(), PecosError> {
        let llvm_ir = compile_hugr_bytes_to_string(hugr_bytes)?;
        std::fs::write(output_path, llvm_ir)
            .map_err(|e| PecosError::Generic(format!("Failed to write LLVM IR: {}", e)))?;
        Ok(())
    }

    /// Compile HUGR file to LLVM IR file
    pub fn compile_hugr<P: AsRef<Path>>(&self, hugr_path: P) -> Result<PathBuf, PecosError> {
        compile_hugr_to_llvm(hugr_path, self.config.output_path.clone())
    }
}

/// Compile a HUGR file to LLVM IR file
///
/// # Arguments
/// * `hugr_path` - Path to the HUGR file
/// * `output_path` - Optional output path (defaults to input with .ll extension)
///
/// # Returns
/// Path to the generated LLVM IR file
///
/// # Errors
/// Returns `PecosError` if compilation fails
pub fn compile_hugr_to_llvm<P: AsRef<Path>>(
    hugr_path: P,
    output_path: Option<PathBuf>,
) -> Result<PathBuf, PecosError> {
    // Read the HUGR file
    let hugr_bytes = std::fs::read(&hugr_path)
        .map_err(|e| PecosError::Generic(format!("Failed to read HUGR file: {}", e)))?;

    // Compile to LLVM IR
    let llvm_ir = compile_hugr_bytes_to_string(&hugr_bytes)?;

    // Determine output path
    let output = output_path.unwrap_or_else(|| {
        let mut path = hugr_path.as_ref().to_path_buf();
        path.set_extension("ll");
        path
    });

    // Write to file
    std::fs::write(&output, llvm_ir)
        .map_err(|e| PecosError::Generic(format!("Failed to write LLVM IR: {}", e)))?;

    Ok(output)
}

// The compile_hugr_bytes_to_string function is re-exported from compiler module
