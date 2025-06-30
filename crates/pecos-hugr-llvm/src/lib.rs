/*!
Pure HUGR to LLVM IR Compilation

This crate provides pure compilation functionality from HUGR (Hierarchical Unified Graph Representation)
to LLVM IR. It has no dependencies on execution engines or runtime systems - it focuses solely on 
translation between representations.

# Architecture

```text
HUGR JSON/Bytes → [pecos-hugr-llvm] → LLVM IR String/File
```

The generated LLVM IR can then be executed by any compatible execution engine (e.g., pecos-qir).

# Example

```rust,no_run
use pecos_hugr_llvm::{HugrCompiler, HugrCompilerConfig};

// Compile HUGR bytes to LLVM IR string
let hugr_bytes = b"example HUGR data";
let compiler = HugrCompiler::new();
let llvm_ir = compiler.compile_hugr_bytes_to_string(hugr_bytes)?;

// Or compile HUGR file to LLVM IR file
let config = HugrCompilerConfig {
    output_path: Some("output.ll".into()),
    debug_info: false,
};
let compiler = HugrCompiler::with_config(config);
let llvm_path = compiler.compile_hugr("input.hugr")?;
# Ok::<(), pecos_core::errors::PecosError>(())
```
*/

pub mod compiler;
pub mod extensions;
pub mod generators;
pub mod result_extractor;

// Re-export main types
pub use compiler::{HugrCompiler, HugrCompilerConfig};

// Convenience functions
use pecos_core::errors::PecosError;
use std::path::{Path, PathBuf};

/// Compile a HUGR file to LLVM IR file with default settings
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
    output_path: Option<PathBuf>
) -> Result<PathBuf, PecosError> {
    let mut compiler = HugrCompiler::new();
    
    if let Some(output) = output_path {
        compiler = compiler.with_output_path(output);
    }
    
    compiler.compile_hugr(hugr_path)
}

/// Compile HUGR bytes to LLVM IR string with default settings
///
/// # Arguments
/// * `hugr_bytes` - HUGR data as bytes
///
/// # Returns
/// LLVM IR as a string
///
/// # Errors
/// Returns `PecosError` if compilation fails
pub fn compile_hugr_bytes_to_string(hugr_bytes: &[u8]) -> Result<String, PecosError> {
    let compiler = HugrCompiler::new();
    compiler.compile_hugr_bytes_to_string(hugr_bytes)
}