pub mod engine;
pub mod error_handling; // Enhanced error handling and debugging for QIR execution
pub mod library;
pub mod linker; // Links QIR programs with runtime library
pub mod panic_handler; // QIR-aware panic handler for better diagnostics
pub mod platform;
pub mod prelude;
pub mod python_api;
pub mod qir_compat; // QIR compatibility layer (alternative naming conventions)
pub mod qir_utils; // QIR utilities for entry point detection
pub mod runtime; // Python-friendly API functions
pub mod runtime_context; // Thread-local runtime context for isolation
pub mod runtime_state; // Instance-based runtime state
pub mod runtime_registry; // Runtime registry for thread isolation
pub mod runtime_cleanup; // Runtime cleanup utilities

// HUGR-LLVM pipeline functionality
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod hugr; // HUGR frontend (compiler, engine, etc.)

// PMIR (PECOS MLIR) - Alternative compilation pipeline via MLIR
#[cfg(feature = "pmir-pipeline")]
pub mod pmir; // HUGR → PAST (RON) → PMIR (MLIR) → LLVM pipeline

// Quantum extension modules for quantum IR→LLVM compilation
#[cfg(feature = "hugr-llvm-pipeline")]
pub mod quantum_extension; // Quantum operation extensions

// Internal modules for compilation
pub(crate) mod runtime_builder; // Builds the static runtime library

pub use engine::QirEngine;

// HUGR-LLVM pipeline re-exports (only available with hugr-llvm-pipeline feature)
#[cfg(feature = "hugr-llvm-pipeline")]
pub use hugr::compiler::{
    HugrCompiler, HugrCompilerConfig, QuantumLlvmConvention,
};
#[cfg(feature = "hugr-llvm-pipeline")]
pub use hugr::engine::{compile_hugr_to_qir, create_hugr_qir_engine, setup_hugr_qir_engine};

// PMIR pipeline re-exports (only available with pmir-pipeline feature)
#[cfg(feature = "pmir-pipeline")]
pub use pmir::{
    compile_hugr_via_pmir, hugr_to_past_ron, hugr_to_pmir_mlir, past_ron_to_pmir_mlir, PmirConfig,
};

// Provide stubs when hugr-llvm-pipeline is not enabled
#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub mod hugr_stubs {
    use pecos_core::errors::PecosError;
    use std::path::Path;
    
    pub struct HugrCompiler;
    pub struct HugrCompilerConfig;
    pub struct QuantumLlvmConvention;
    
    impl HugrCompiler {
        pub fn new(_config: HugrCompilerConfig) -> Result<Self, PecosError> {
            Err(PecosError::Feature("HUGR-LLVM pipeline not available".to_string()))
        }
    }
    
    pub fn compile_hugr_to_qir(_hugr_path: &Path, _output_path: &Path) -> Result<(), PecosError> {
        Err(PecosError::Compilation("HUGR-LLVM pipeline not available".to_string()))
    }
    
    pub fn create_hugr_qir_engine(_hugr_path: &Path) -> Result<crate::QirEngine, PecosError> {
        Err(PecosError::Compilation("HUGR-LLVM pipeline not available".to_string()))
    }
    
    pub fn setup_hugr_qir_engine(_hugr_path: &Path) -> Result<Box<dyn pecos_engines::ClassicalEngine>, PecosError> {
        Err(PecosError::Compilation("HUGR-LLVM pipeline not available".to_string()))
    }
}

#[cfg(not(feature = "hugr-llvm-pipeline"))]
pub use hugr_stubs::{HugrCompiler, HugrCompilerConfig, QuantumLlvmConvention, compile_hugr_to_qir, create_hugr_qir_engine, setup_hugr_qir_engine};

// Provide stubs when pmir-pipeline is not enabled
#[cfg(not(feature = "pmir-pipeline"))]
pub mod pmir_stubs {
    use pecos_core::errors::PecosError;
    
    #[derive(Debug, Clone)]
    pub struct PmirConfig {
        pub debug_output: bool,
        pub optimization_level: u8,
        pub target_triple: Option<String>,
    }
    
    impl Default for PmirConfig {
        fn default() -> Self {
            Self {
                debug_output: false,
                optimization_level: 2,
                target_triple: None,
            }
        }
    }
    
    pub fn compile_hugr_via_pmir(_hugr_json: &str, _config: &PmirConfig) -> Result<String, PecosError> {
        Err(PecosError::Feature("PMIR pipeline not available".to_string()))
    }
    
    pub fn hugr_to_past_ron(_hugr_json: &str) -> Result<String, PecosError> {
        Err(PecosError::Feature("PMIR pipeline not available".to_string()))
    }
    
    pub fn hugr_to_pmir_mlir(_hugr_json: &str, _config: &PmirConfig) -> Result<String, PecosError> {
        Err(PecosError::Feature("PMIR pipeline not available".to_string()))
    }
    
    pub fn past_ron_to_pmir_mlir(_past_ron: &str, _config: &PmirConfig) -> Result<String, PecosError> {
        Err(PecosError::Feature("PMIR pipeline not available".to_string()))
    }
}

#[cfg(not(feature = "pmir-pipeline"))]
pub use pmir_stubs::{compile_hugr_via_pmir, hugr_to_past_ron, hugr_to_pmir_mlir, past_ron_to_pmir_mlir, PmirConfig};

use log::debug;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::path::Path;

/// Sets up a basic QIR engine.
///
/// This function creates a QIR engine from the provided path.
///
/// # Parameters
///
/// - `program_path`: A reference to the path of the QIR program file
/// - `shots`: Optional number of shots to assign to the engine
///
/// # Returns
///
/// Returns a `Box<dyn ClassicalEngine>` containing the QIR engine
///
/// # Errors
///
/// This function may return the following errors:
/// - `PecosError::Compilation`: If the QIR file cannot be compiled
/// - `PecosError::Processing`: If the QIR engine fails to process commands
pub fn setup_qir_engine(
    program_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    debug!("Setting up QIR engine for: {}", program_path.display());

    // Create a QirEngine from the path
    let mut engine = QirEngine::new(program_path.to_path_buf());

    // Set the number of shots assigned to this engine if specified
    if let Some(num_shots) = shots {
        engine.set_assigned_shots(num_shots);
    }

    // Pre-compile the QIR library for efficient cloning
    engine.pre_compile()?;

    Ok(Box::new(engine))
}
