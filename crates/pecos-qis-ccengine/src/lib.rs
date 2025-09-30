//! QIS Classical Control Engine
//!
//! This crate provides the orchestration between QisInterface (linked programs)
//! and QisRuntime (interpreters), implementing ClassicalControlEngine for PECOS integration.
//!
//! It includes multiple runtime implementations:
//! - NativeRuntime: Pure Rust interpreter
//! - SeleneRuntime: FFI wrapper for Selene .so
//! - MockRuntime: Deterministic testing runtime
//!
//! # Example Usage
//!
//! ```rust
//! use pecos_qis_ccengine::{qis_control_engine, native_runtime, QisControlEngine};
//! use pecos_qis_interface::QisInterface;
//! use pecos_engines::{ClassicalEngine, ClassicalControlEngineBuilder};
//!
//! // Method 1: Using builder with default native runtime
//! let engine1 = qis_control_engine().build().unwrap();
//! assert_eq!(engine1.num_qubits(), 0);
//!
//! // Method 2: Specifying runtime explicitly
//! let engine2 = qis_control_engine()
//!     .runtime(native_runtime())
//!     .build()
//!     .unwrap();
//! assert_eq!(engine2.num_qubits(), 0);
//!
//! // Method 3: Direct construction
//! let runtime = Box::new(native_runtime());
//! let engine3 = QisControlEngine::new(runtime);
//! assert_eq!(engine3.num_qubits(), 0);
//! ```
//!
//! # Using Selene Runtimes
//!
//! ```rust
//! use pecos_qis_ccengine::{qis_control_engine, selene_simple_runtime, selene_soft_rz_runtime};
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
//!         // Runtime not available - use native runtime as fallback
//!         let engine = qis_control_engine().build().unwrap();
//!     }
//! }
//! ```

pub mod builder;
pub mod ccengine;
pub mod mock_runtime;
pub mod native_runtime;
pub mod program;
pub mod selene_runtime;
pub mod selene_runtimes;

pub use builder::{
    QisEngineBuilder,
    qis_control_engine,
    qis_control_engine_selene,
    native_runtime,
    selene_runtime,
};
pub use ccengine::QisControlEngine;
pub use mock_runtime::{MockRuntime, mock_bell_state_runtime, mock_all_ones_runtime, mock_pattern_runtime};
pub use native_runtime::NativeRuntime;
pub use program::{IntoQisInterface, QisControlEngineProgram};
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

// Re-export the runtime trait for convenience
pub use pecos_qis_runtime_trait::{QisRuntime, RuntimeError, ClassicalState, Shot};