/*!
HUGR Frontend for PECOS QIR

This module provides HUGR-specific functionality for compiling and executing
quantum programs represented in the HUGR (Hierarchical Unified Graph Representation) format.

HUGR is an intermediate representation used by quantum programming languages like Guppy.
This module bridges HUGR programs with the PECOS QIR execution infrastructure.

# Architecture

```text
Guppy/HUGR Source → HUGR IR → LLVM/QIR → PECOS Execution
                      ↑           ↑            ↑
                   hugr::*   quantum_ext   QirEngine
```

# Modules

- `compiler`: HUGR → QIR compilation pipeline
- `engine`: High-level engine creation from HUGR sources
- `result_extractor`: Extract measurement result names from HUGR graphs

# Example Usage

```rust
# #[cfg(feature = "hugr-llvm-pipeline")]
# fn example() -> Result<(), Box<dyn std::error::Error>> {
use pecos_qir::hugr::{Compiler, CompilerConfig};
use std::path::PathBuf;

// Create a compiler with custom configuration
let config = CompilerConfig {
    output_path: Some(PathBuf::from("output.ll")),
    debug_info: false,
    quantum_naming: pecos_qir::hugr::QuantumNamingConvention::StandardQir,
};

let compiler = Compiler::with_config(config);

// Or use the builder pattern
let compiler = Compiler::new()
    .with_output_path("output.ll")
    .with_debug_info(false);
# Ok(())
# }
```
*/

#[cfg(feature = "hugr-llvm-pipeline")]
pub mod compiler;
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod engine;
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod result_extractor;
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod simple_llvm_fallback;
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod standard_qir_generator;
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod type_transformer;
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod version_translator;

// Re-export main types for convenience
#[cfg(feature = "hugr-llvm-pipeline")]
pub use compiler::{
    HugrCompiler as Compiler, HugrCompilerConfig as CompilerConfig, QuantumNamingConvention,
};
#[cfg(feature = "hugr-llvm-pipeline")]
pub use engine::{compile_hugr_to_qir, create_hugr_qir_engine, setup_hugr_qir_engine};
#[cfg(feature = "hugr-llvm-pipeline")]
pub use result_extractor::{ResultNameExtractor, ResultNameMapping};

// Provide stubs when HUGR support is disabled
#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub mod compiler {
    use pecos_core::errors::PecosError;
    use std::path::{Path, PathBuf};

    pub struct HugrCompiler;

    #[derive(Debug, Clone)]
    pub struct HugrCompilerConfig {
        pub output_path: Option<PathBuf>,
        pub debug_info: bool,
        pub quantum_naming: QuantumNamingConvention,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum QuantumNamingConvention {
        StandardQir,
        Hugr,
        Pecos,
    }

    impl Default for HugrCompilerConfig {
        fn default() -> Self {
            Self {
                output_path: None,
                debug_info: false,
                quantum_naming: QuantumNamingConvention::StandardQir,
            }
        }
    }

    impl HugrCompiler {
        pub fn new() -> Self {
            Self
        }

        pub fn with_config(_config: HugrCompilerConfig) -> Self {
            Self
        }

        pub fn compile_hugr<P: AsRef<Path>>(&self, _: P) -> Result<PathBuf, PecosError> {
            Err(PecosError::with_context(
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "HUGR support not compiled in",
                ),
                "Enable 'hugr-llvm-pipeline' feature to use HUGR compilation",
            ))
        }

        pub fn compile_hugr_bytes(&self, _: &[u8], _: &Path) -> Result<PathBuf, PecosError> {
            Err(PecosError::with_context(
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "HUGR support not compiled in",
                ),
                "Enable 'hugr-llvm-pipeline' feature to use HUGR compilation",
            ))
        }

        pub fn compile_hugr_bytes_to_string(&self, _: &[u8]) -> Result<String, PecosError> {
            Err(PecosError::with_context(
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "HUGR support not compiled in",
                ),
                "Enable 'hugr-llvm-pipeline' feature to use HUGR compilation",
            ))
        }
    }

    impl Default for HugrCompiler {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub mod engine {
    use pecos_core::errors::PecosError;
    use pecos_engines::ClassicalEngine;
    use std::path::Path;

    pub fn create_hugr_qir_engine<P: AsRef<Path>>(
        _: P,
        _: Option<usize>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        Err(PecosError::with_context(
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "HUGR support not compiled in",
            ),
            "Enable 'hugr' feature to use HUGR compilation",
        ))
    }

    pub fn setup_hugr_qir_engine<P: AsRef<Path>>(
        hugr_path: P,
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        create_hugr_qir_engine(hugr_path, shots)
    }

    pub fn compile_hugr_to_qir<P: AsRef<Path>, Q: AsRef<Path>>(
        _: P,
        _: Q,
    ) -> Result<std::path::PathBuf, PecosError> {
        Err(PecosError::with_context(
            std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "HUGR support not compiled in",
            ),
            "Enable 'hugr' feature to use HUGR compilation",
        ))
    }
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub mod result_extractor {
    use std::collections::HashMap;

    pub type ResultNameMapping = HashMap<u32, String>; // Use u32 instead of Node
    pub struct ResultNameExtractor;
}
