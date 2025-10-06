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
//! use pecos_qis_core::{qis_control_engine, QisControlEngine};
//! use pecos_qis_native::native_runtime;
//! use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
//! use pecos_qis_ffi::{OperationCollector, QuantumOp};
//!
//! // Create an interface with quantum operations
//! let mut interface = OperationCollector::new();
//! let q0 = interface.allocate_qubit();
//! interface.operations.push(QuantumOp::H(q0).into());
//!
//! // Build engine with native runtime
//! let engine = qis_control_engine()
//!     .runtime(native_runtime())
//!     .program(interface)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(engine.num_qubits(), 1);
//! ```
//!
//! # Using Alternative Runtimes
//!
//! The QIS control engine can work with different runtime implementations.
//! This example shows using a custom runtime with an interface:
//!
//! ```rust
//! use pecos_qis_core::qis_control_engine;
//! use pecos_qis_native::native_runtime;
//! use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
//! use pecos_qis_ffi::OperationCollector;
//!
//! // Create a simple program with operations
//! let mut interface = OperationCollector::new();
//! let q0 = interface.allocate_qubit();
//! let q1 = interface.allocate_qubit();
//! interface.operations.push(pecos_qis_ffi::QuantumOp::H(q0).into());
//! interface.operations.push(pecos_qis_ffi::QuantumOp::CX(q0, q1).into());
//!
//! // Build engine with native runtime
//! let engine = qis_control_engine()
//!     .runtime(native_runtime())
//!     .program(interface)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(engine.num_qubits(), 2);
//! ```
//!
//! For Selene-based runtimes and interfaces (LLVM execution), see the
//! `pecos-qis-selene` and `pecos-qis-jit` crates.

pub mod builder;
pub mod ccengine;
pub mod interface_impl;
pub mod program;
pub mod qis_interface;
pub mod runtime;

pub use builder::{
    QisEngineBuilder,
    qis_control_engine,
};
pub use ccengine::QisControlEngine;

// Re-export QisInterface trait and related types
pub use qis_interface::{QisInterface, ProgramFormat, InterfaceError, BoxedInterface};
pub use interface_impl::SimpleQisInterface;

pub use program::{
    IntoQisInterface,
    InterfaceChoice,
    QisControlEngineProgram,
    QisInterfaceProvider,
    QisInterfaceBuilder,
    ProgramType,
};

// Re-export the runtime trait and types for convenience
pub use runtime::{QisRuntime, RuntimeError, ClassicalState, Shot, CallFrame, Value, Result as RuntimeResult};

use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngine;
use pecos_programs::QisProgram;
use std::path::Path;

/// Setup a QIS control engine for a program file with an explicit runtime
///
/// This function loads a QIS program from a file and creates a control engine
/// using the provided runtime.
///
/// # Parameters
///
/// - `program_path`: Path to the QIS program file (.ll or .bc)
/// - `runtime`: The QIS runtime to use (e.g., NativeRuntime, SeleneRuntime)
///
/// # Returns
///
/// Returns a boxed `ClassicalControlEngine` on success.
///
/// # Errors
///
/// - `PecosError::IO`: If the program file cannot be read
/// - `PecosError::Processing`: If the engine creation fails
pub fn setup_qis_control_engine_with_runtime(
    program_path: &Path,
    runtime: impl QisRuntime + 'static,
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    use pecos_engines::ClassicalControlEngineBuilder;

    log::debug!("Loading QIS program from: {}", program_path.display());
    // Load the QIS program from file
    let program = QisProgram::from_file(program_path)?;

    log::debug!("Creating QIS control engine with explicit runtime");
    let builder = qis_control_engine()
        .runtime(runtime)
        .try_program(program)
        .map_err(|e| {
            PecosError::Processing(format!(
                "Failed to load QIS program: {}",
                e
            ))
        })?;

    log::debug!("Building engine");
    let engine = builder.build()
        .map_err(|e| PecosError::Processing(format!("Failed to build engine: {}", e)))?;

    log::debug!("Engine built successfully");
    Ok(Box::new(engine) as Box<dyn ClassicalControlEngine>)
}

/// Setup a QIS control engine for a program file (deprecated)
///
/// **Deprecated**: This function is deprecated because it relied on implicit runtime selection.
/// Use `setup_qis_control_engine_with_runtime` instead and provide an explicit runtime.
///
/// This function attempts to load the program with the default Helios interface
/// and requires a runtime to be available. Since runtime selection is environment-dependent,
/// callers should use the explicit version.
///
/// # Parameters
///
/// - `program_path`: Path to the QIS program file (.ll or .bc)
///
/// # Returns
///
/// Returns an error directing users to use the explicit runtime version.
#[deprecated(
    since = "0.1.1",
    note = "Use setup_qis_control_engine_with_runtime with an explicit runtime instead"
)]
pub fn setup_qis_control_engine(
    _program_path: &Path,
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    Err(PecosError::Processing(
        "setup_qis_control_engine is deprecated.\n\
        \n\
        Please use setup_qis_control_engine_with_runtime and provide an explicit runtime:\n\
        \n\
        use pecos_qis_core::setup_qis_control_engine_with_runtime;\n\
        use pecos_qis_native::native_runtime;\n\
        \n\
        let engine = setup_qis_control_engine_with_runtime(path, native_runtime())?;".to_string()
    ))
}