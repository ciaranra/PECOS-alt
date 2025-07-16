//! Trait for building classical control engines and converting to simulation builders
//!
//! This module provides the core trait that all engine builders must implement
//! to participate in the unified simulation API.

use crate::sim_builder::SimBuilder;
use crate::ClassicalControlEngine;
use pecos_core::errors::PecosError;

/// Trait for building classical control engines
///
/// This trait must be implemented by all engine builders (QASM, LLVM, Selene, etc.)
/// to enable the unified simulation API through the `.to_sim()` method.
pub trait ClassicalControlEngineBuilder: Sized {
    /// The type of engine this builder creates
    type Engine: ClassicalControlEngine + Clone + 'static;

    /// Build the classical control engine
    ///
    /// This method is called internally by `SimBuilder` when `.build()` or `.run()` is called.
    fn build(self) -> Result<Self::Engine, PecosError>;

    /// Convert this engine builder to a simulation builder
    ///
    /// This enables the fluent API pattern:
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # // This is a conceptual example showing the API pattern
    /// # // In practice, you would use the actual engine builders from specific crates
    /// # use pecos_engines::ClassicalControlEngineBuilder;
    /// # 
    /// # // Example using a hypothetical qasm_engine function
    /// # // In real code, use: use pecos_qasm::unified_engine_builder::qasm_engine;
    /// # mod example {
    /// #     use pecos_engines::{ClassicalControlEngineBuilder, sim_builder::SimBuilder};
    /// #     use pecos_engines::monte_carlo::engine::ExternalClassicalEngine;
    /// #     
    /// #     pub struct QasmEngineBuilder;
    /// #     
    /// #     impl ClassicalControlEngineBuilder for QasmEngineBuilder {
    /// #         type Engine = ExternalClassicalEngine;
    /// #         
    /// #         fn build(self) -> Result<Self::Engine, pecos_core::errors::PecosError> {
    /// #             Ok(ExternalClassicalEngine::new())
    /// #         }
    /// #     }
    /// #     
    /// #     impl QasmEngineBuilder {
    /// #         pub fn qasm(self, _qasm: &str) -> Self { self }
    /// #     }
    /// #     
    /// #     pub fn qasm_engine() -> QasmEngineBuilder { QasmEngineBuilder }
    /// # }
    /// # use example::qasm_engine;
    /// #
    /// let results = qasm_engine()
    ///     .qasm("H q[0];")
    ///     .to_sim()
    ///     .seed(42)
    ///     .run(1000)?;
    /// # Ok(())
    /// # }
    /// ```
    fn to_sim(self) -> SimBuilder<Self> {
        SimBuilder::new(self)
    }
}