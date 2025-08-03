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
//! ```rust
//! # use pecos_selene::prelude::*;
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
//! let engine = selene_engine()
//!     .program(LlvmProgram::from_ir(simple_llvm))
//!     .qubits(1)
//!     .optimize(true)
//!     .to_sim()
//!     .build()?;
//!
//! // Method 2: Direct construction using real Selene components
//! let mut engine2 = SeleneEngine::new(
//!     SeleneProgram::LlvmIr(simple_llvm.to_string()),
//!     1, // num_qubits
//!     true // optimize
//! );
//!
//! // Use with PECOS quantum engines
//! let shot = engine2.process(())?;
//! # Ok(())
//! # }
//! ```


pub mod selene_engine;
pub mod engine_builder;
pub mod error;
pub mod prelude;
pub mod program;
pub mod simulator_plugin_template;

#[cfg(feature = "hugr")]
pub mod hugr_compiler;

// Note: The old selene_sim() API has been removed. Use selene_engine().to_sim() instead.
// Noise models and quantum engine types are now provided by pecos-engines.
pub use engine_builder::{selene_engine, SeleneEngineBuilder};
pub use selene_engine::SeleneEngine;
pub use error::SeleneError;
pub use program::SeleneProgram;

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_engines::ClassicalControlEngineBuilder;

    #[test]
    fn test_selene_sim_builder_creation() {
        let builder = selene_engine();
        assert!(builder.build().is_err()); // Should fail without program
    }
    
    #[test]
    fn test_selene_engine_integration() {
        use pecos_engines::{Engine, ClassicalEngine, ControlEngine};
        use crate::program::SeleneProgram;
        
        // Create the Selene engine using real selene-core components
        let engine = SeleneEngine::new(
            SeleneProgram::LlvmIr("bell_state_llvm_ir".to_string()),
            2,
            true,
        );
        
        // Verify it implements all required PECOS traits
        fn assert_is_engine<T: Engine>(_: &T) {}
        fn assert_is_classical<T: ClassicalEngine>(_: &T) {}
        fn assert_is_control<T: ControlEngine>(_: &T) {}
        fn assert_is_send_sync<T: Send + Sync>(_: &T) {}
        fn assert_is_clone<T: Clone>(_: &T) {}
        
        assert_is_engine(&engine);
        assert_is_classical(&engine);
        assert_is_control(&engine);
        assert_is_send_sync(&engine);
        assert_is_clone(&engine);
        
        // Verify basic properties
        assert_eq!(engine.num_qubits(), 2);
        
        // Test that compilation works
        assert!(engine.compile().is_ok());
    }
    
    #[test]
    fn test_clean_api_demonstration() {
        use crate::program::SeleneProgram;
        use pecos_programs::LlvmProgram;
        
        // Demonstrate the clean, simple API  
        let _engine1 = SeleneEngine::new(
            SeleneProgram::LlvmIr("bell_state".to_string()),
            2,
            true
        );
        
        let _engine2 = SeleneEngine::new(
            SeleneProgram::Hugr(hugr::Hugr::default()),
            4,
            false
        );
        
        // Builder pattern still works too
        let _engine3 = selene_engine()
            .program(LlvmProgram::from_ir("simple_circuit"))
            .qubits(1)
            .to_sim()
            .build()
            .expect("Should build successfully");
    }

}