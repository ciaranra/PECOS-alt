//! Selene-based classical control engine (CENG) for PECOS
//!
//! This crate provides integration between PECOS and the Selene quantum emulation platform,
//! allowing PECOS to leverage Selene's classical control capabilities for quantum-classical
//! hybrid algorithms and control flow.
//!
//! The "ceng" suffix indicates this provides **Classical Engines** that implement
//! PECOS's `ClassicalEngine` and `ControlEngine` traits using Selene components.
//!
//! For quantum simulation backends using Selene, see the future `pecos-selene-engine-qeng` crate.
//!
//! # Example
//!
//! ```rust
//! use pecos_programs::QisProgram;
//!
//! // Simple QIS IR for a basic quantum circuit
//! let simple_qis = r#"
//! define void @main() {
//!     ret void
//! }
//! "#;
//!
//! // Create a QisProgram from the IR string
//! let program = QisProgram::from_string(simple_qis.to_string());
//!
//! // The program is created and ready to use
//! // It can be used with selene_executable() or other engine builders
//! ```

// Selene FFI to ByteMessage bridge - provides the FFI functions that plugins expect
// and converts them directly to ByteMessages for PECOS
pub mod selene_ffi_to_bytemessage;

pub mod error;
pub mod prelude;
pub mod program;
pub mod selene_executable_builder;
pub mod selene_executable_engine;
pub mod selene_library_engine;
pub mod selene_runtime_init;
// QIS bridge removed - we transform QIS to Helios interface instead
// Simple runtime removed - use selene_executable instead

// HUGR 0.13 support has been removed - using tket's HUGR 0.22 instead

// Use Selene's SeleneInstance directly - this is the natural way to use Selene
// SeleneInstance provides all FFI functions and manages the execution context

// Note: The old selene_sim() API has been removed. Use selene_executable() instead.
// Noise models and quantum engine types are now provided by pecos-engines.

// Export the new bridge-based approach
pub use selene_executable_builder::{SeleneExecutableEngineBuilder, selene_executable};
pub use selene_executable_engine::{SeleneExecutableConfig, SeleneExecutableEngine};

// Legacy aliases removed - use selene_executable() instead
pub use error::SeleneError;
pub use program::SeleneProgram;

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};

    #[test]
    fn test_selene_executable_builder_creation() {
        // Test that builder can be created
        let builder = selene_executable();

        // Build should fail without a program
        let result = builder.build();
        assert!(result.is_err());

        // Check the error message
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("No program specified"));
        }
    }

    #[test]
    fn test_selene_executable_builder_with_program() {
        use pecos_programs::QisProgram;

        // Test that builder succeeds with a program
        let builder = selene_executable()
            .program(QisProgram::from_string("define void @main() { ret void }"))
            .qubits(5);

        let result = builder.build();
        assert!(result.is_ok());

        // Verify the engine has the correct number of qubits
        let engine = result.unwrap();
        assert_eq!(engine.num_qubits(), 5);
    }
}
