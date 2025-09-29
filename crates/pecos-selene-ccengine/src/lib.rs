//! # PECOS Selene Classical Control Engine
//!
//! This crate provides a classical control engine that leverages Selene runtime plugins
//! for sophisticated control flow (loops, conditionals, etc.) while generating PECOS
//! ByteMessages for quantum operations.
//!
//! ## Architecture
//!
//! The engine works by:
//! 1. Loading a Selene runtime plugin (.so file) like `simple_runtime`
//! 2. Compiling QIS LLVM IR programs that call `___*` functions
//! 3. Bridging those calls to `selene_runtime_*` functions in the plugin
//! 4. Converting runtime operations to ByteMessages via callbacks
//! 5. Managing the execution flow as a PECOS ControlEngine
//!
//! ## Example
//!
//! ```rust,no_run
//! use pecos_selene_ccengine::{SeleneClassicalControlEngine, SeleneEngineConfig};
//! use pecos_engines::Engine;
//! use std::path::PathBuf;
//!
//! // Configure the engine
//! let config = SeleneEngineConfig {
//!     runtime_plugin_path: PathBuf::from("libselene_simple_runtime.so"),
//!     n_qubits: 10,
//!     verbose: true,
//! };
//!
//! // Create the engine
//! let mut engine = SeleneClassicalControlEngine::new(config)?;
//!
//! // Load a QIS program
//! engine.load_llvm_ir("program.ll")?;
//!
//! // Run the program
//! let shot = engine.process(())?;
//! # Ok::<(), pecos_core::errors::PecosError>(())
//! ```

pub mod bridge;
pub mod engine;
pub mod ffi_bridge;
pub mod runtime_plugin;

#[cfg(test)]
pub mod test_integration;

#[cfg(test)]
pub mod test_qis_integration;

// Re-export main types
pub use engine::{SeleneClassicalControlEngine, SeleneEngineConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let config = SeleneEngineConfig::default();
        // This will fail unless we have the runtime plugin available
        // Just test that we can create the config
        assert_eq!(config.n_qubits, 20);
    }
}
