//! Selene-based classical control engine (CENG) for PECOS
//!
//! This crate provides integration between PECOS and the Selene quantum emulation platform,
//! allowing PECOS to leverage Selene's classical control capabilities for quantum-classical
//! hybrid algorithms and control flow.
//!
//! The "ceng" suffix indicates this provides **Classical Engines** that implement
//! PECOS's `ClassicalEngine` and `ControlEngine` traits using Selene components.
//!
//! For quantum simulation backends using Selene, see the future `pecos-selene-qeng` crate.
//!
//! # Example
//!
//! ```rust,no_run
//! # use pecos_selene::prelude::*;
//! # use pecos_selene::{selene_executable, SeleneExecutableEngine};
//! # use pecos_engines::Engine;
//! # fn main() -> Result<(), PecosError> {
//! // Simple LLVM IR for a Hadamard gate and measurement
//! let simple_llvm = r#"
//! declare void @__quantum__qis__h__body(i64)
//! declare i32 @__quantum__qis__m__body(i64, i64)
//!
//! define void @test() #0 {
//!     call void @__quantum__qis__h__body(i64 0)
//!     %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
//!     ret void
//! }
//!
//! attributes #0 = { "EntryPoint" }
//! "#;
//!
//! // Method 1: Using the builder pattern with LLVM IR
//! let engine = selene_executable()
//!     .program(LlvmProgram::from_ir(simple_llvm))
//!     .qubits(1)
//!     .build()?;
//!
//! // Method 2: Direct construction
//! let mut engine2 = SeleneExecutableEngine::new(1)?
//!     .with_llvm_program(LlvmProgram::from_ir(simple_llvm));
//!
//! // Use with PECOS quantum engines
//! let shot = engine2.process(())?;
//! # Ok(())
//! # }
//! ```

// Selene FFI to ByteMessage bridge - provides the FFI functions that plugins expect
// and converts them directly to ByteMessages for PECOS
pub mod selene_ffi_to_bytemessage;

pub mod error;
pub mod prelude;
pub mod program;
pub mod selene_executable_builder;
pub mod selene_executable_engine;
pub mod selene_in_process_engine;
pub mod selene_library_engine;
pub mod selene_runtime_init;
// Simple runtime removed - use selene_executable instead
pub mod simulator_plugin_template;

#[cfg(feature = "hugr-013")]
pub mod hugr_013_support;

#[cfg(feature = "hugr-013")]
pub mod hugr_llvm_compiler;

#[cfg(feature = "hugr-013")]
pub mod hugr_qis_lowering;

#[cfg(feature = "hugr-013")]
pub mod hugr_to_llvm;

#[cfg(feature = "hugr-013")]
pub mod hugr_to_llvm_cfg_support;

pub mod selene_hugr_compiler;

// Use Selene's SeleneInstance directly - this is the natural way to use Selene
// SeleneInstance provides all FFI functions and manages the execution context

// Note: The old selene_sim() API has been removed. Use selene_executable() instead.
// Noise models and quantum engine types are now provided by pecos-engines.

// Export the new bridge-based approach
pub use selene_executable_builder::{SeleneExecutableEngineBuilder, selene_executable};
pub use selene_executable_engine::{SeleneExecutableConfig, SeleneExecutableEngine};

// Export the in-process engine
pub use selene_in_process_engine::{SeleneInProcessConfig, SeleneInProcessEngine};

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
        use pecos_programs::LlvmProgram;

        // Test that builder succeeds with a program
        let builder = selene_executable()
            .program(LlvmProgram::from_string("define void @main() { ret void }"))
            .qubits(5);

        let result = builder.build();
        assert!(result.is_ok());

        // Verify the engine has the correct number of qubits
        let engine = result.unwrap();
        assert_eq!(engine.num_qubits(), 5);
    }
}
