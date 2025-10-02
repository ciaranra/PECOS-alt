//! QIS Classical Control Engine
//!
//! This crate provides the orchestration between QisInterface (linked programs)
//! and QisRuntime (interpreters), implementing ClassicalControlEngine for PECOS integration.
//!
//! It includes multiple runtime implementations:
//! - NativeRuntime: Pure Rust interpreter
//! - SeleneRuntime: FFI wrapper for Selene .so
//!
//! # Example Usage
//!
//! ```rust
//! use pecos_qis_ccengine::{qis_control_engine, native_runtime, QisControlEngine};
//! use pecos_qis_interface::QisInterface;
//! use pecos_engines::{ClassicalEngine, ClassicalControlEngineBuilder};
//!
//! // Method 1: Using builder with default runtime (Selene if available, otherwise native)
//! let engine1 = qis_control_engine().build().unwrap();
//! assert_eq!(engine1.num_qubits(), 0);
//!
//! // Method 2: Explicitly use native runtime
//! let engine2 = qis_control_engine()
//!     .runtime(native_runtime())
//!     .build()
//!     .unwrap();
//! assert_eq!(engine2.num_qubits(), 0);
//!
//! // Method 3: Direct construction (not recommended - use builder instead)
//! // Note: This requires both interface and runtime
//! // let interface = Box::new(qis_jit_interface());
//! // let runtime = Box::new(native_runtime());
//! // let engine3 = QisControlEngine::new(interface, runtime);
//! ```
//!
//! # Using Selene Runtimes
//!
//! ```rust
//! use pecos_qis_ccengine::{qis_control_engine, selene_simple_runtime, selene_soft_rz_runtime, native_runtime};
//! use pecos_engines::ClassicalControlEngineBuilder;
//!
//! // Try to use Selene simple runtime (if available)
//! match selene_simple_runtime() {
//!     Ok(runtime) => {
//!         let engine = qis_control_engine()
//!             .runtime(runtime)
//!             .build()
//!             .unwrap();
//!         // Engine is ready with Selene simple runtime
//!     }
//!     Err(_) => {
//!         // Runtime not available - use explicit native runtime
//!         let engine = qis_control_engine().runtime(native_runtime()).build().unwrap();
//!     }
//! }
//! ```

pub mod builder;
pub mod ccengine;
pub mod helios_interface;
pub mod interface_impl;
pub mod jit_executor;
pub mod jit_interface;
pub mod native_runtime;
pub mod program;
pub mod selene_runtime;
pub mod selene_runtimes;
pub mod selene_library_runtime;

pub use builder::{
    QisEngineBuilder,
    qis_control_engine,
    qis_control_engine_selene,
    qis_jit_interface,
    qis_selene_helios_interface,
    native_runtime,
    selene_runtime,
};
pub use ccengine::QisControlEngine;
pub use helios_interface::QisSeleneHeliosInterface;
pub use interface_impl::{QisInterface as QisInterfaceTrait, BoxedInterface, ProgramFormat};
pub use jit_executor::JitExecutor;
pub use jit_interface::QisJitInterface;
pub use native_runtime::NativeRuntime;
pub use program::{
    IntoQisInterface,
    InterfaceChoice,
    QisControlEngineProgram,
    QisInterfaceProvider,
    QisInterfaceBuilder,
    JitInterfaceBuilder,
    HeliosInterfaceBuilder,
    QisJitInterface as OldQisJitInterface,
    QisSeleneHeliosInterface as OldQisSeleneHeliosInterface,
    ProgramType,
    HeliosConfig,
};
pub use selene_runtime::SeleneRuntime;
pub use selene_runtimes::{
    // Selene runtime plugins
    selene_simple_runtime,
    selene_soft_rz_runtime,
    // Utility functions
    selene_runtime_auto,
    find_selene_runtime,
    RuntimeFetchError,
};
pub use selene_library_runtime::{
    QisSeleneLibraryRuntime,
    QisSeleneSimpleRuntime,
    SeleneRuntimeConfig,
    selene_simple_runtime as selene_simple_runtime_v2,
    selene_simple_runtime_from_path,
    selene_library_runtime,
};

// Re-export the runtime trait for convenience
pub use pecos_qis_runtime_trait::{QisRuntime, RuntimeError, ClassicalState, Shot};

use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngine;
use pecos_programs::QisProgram;
use std::path::Path;

/// Setup a QIS control engine for a program file
///
/// This function loads a QIS program from a file and creates a control engine.
/// It uses the default runtime (Selene simple). If not available, returns an error with instructions.
///
/// # Parameters
///
/// - `program_path`: Path to the QIS program file (.ll or .bc)
///
/// # Returns
///
/// Returns a boxed `ClassicalControlEngine` on success.
///
/// # Errors
///
/// - `PecosError::IO`: If the program file cannot be read
/// - `PecosError::Processing`: If the engine creation fails
pub fn setup_qis_control_engine(
    program_path: &Path,
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    use pecos_engines::ClassicalControlEngineBuilder;

    log::debug!("Loading QIS program from: {}", program_path.display());
    // Load the QIS program from file
    let program = QisProgram::from_file(program_path)?;

    log::debug!("Creating QIS control engine");
    // Use Selene interface (default for QIS programs)
    let builder = qis_control_engine().try_program(program.clone())
        .map_err(|e: PecosError| {
            PecosError::Processing(format!(
                "Failed to load QIS program with Selene interface: {}\n\n\
                The Selene interface is the default for QIS programs. If you want to use the JIT interface instead, \
                please use the explicit JIT interface functions:\n\
                \n\
                use pecos_qis_ccengine::{{qis_control_engine, qis_jit_interface, native_runtime}};\n\
                \n\
                // Create JIT interface explicitly\n\
                let interface = qis_jit_interface(program);\n\
                let engine = qis_control_engine()\n\
                    .runtime(native_runtime())\n\
                    .program(interface)\n\
                    .build()?;\n\
                \n\
                To fix the Selene interface, ensure Selene runtime is properly installed and configured.",
                e
            ))
        })?;

    log::debug!("Building engine");
    let engine = builder.build()
        .map_err(|e| PecosError::Processing(format!("Failed to build engine: {}", e)))?;

    log::debug!("Engine built successfully");
    Ok(Box::new(engine) as Box<dyn ClassicalControlEngine>)
}