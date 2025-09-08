/*!
Unified LLVM-based quantum simulation using the unified engine builder API.

This crate provides the `llvm_engine()` and `llvm_sim()` functions that integrate with
the unified PECOS simulation API. The engine builder handles LLVM IR compilation and
provides consistent simulation capabilities with noise models, parallelization, and
multiple quantum engines.

# Example

```rust,no_run
use pecos_llvm_sim::{llvm_sim, llvm_engine};
use pecos_programs::LlvmProgram;
use pecos_engines::noise::DepolarizingNoiseModelBuilder;
use pecos_engines::ClassicalControlEngineBuilder;

// Using the convenience function
let results = llvm_sim("@main() { ret void }")
    .seed(42)
    .workers(8)
    .noise(DepolarizingNoiseModelBuilder::new().with_p1_probability(0.01))
    .run(1000)?;

// Using the full engine builder API
let results = llvm_engine()
    .program(LlvmProgram::from_string("@main() { ret void }"))
    .to_sim()
    .seed(42)
    .run(1000)?;
# Ok::<(), pecos_core::errors::PecosError>(())
```
*/

pub mod engine_builder;
pub mod prelude;
pub mod source;

// Re-export main types for the unified API
pub use engine_builder::{LlvmEngineBuilder, llvm_engine};

// Re-export from pecos-llvm-runtime for backward compatibility
pub use pecos_llvm_runtime::LlvmEngine;

/// Create a new LLVM simulation builder (thin wrapper around `llvm_engine().program().to_sim()`)
///
/// This function creates a `TypedSimBuilder` that uses the unified simulation API.
///
/// # Example
/// ```rust,no_run
/// use pecos_llvm_sim::llvm_sim;
/// use pecos_programs::LlvmProgram;
/// use pecos_engines::noise::DepolarizingNoiseModelBuilder;
///
/// let llvm_ir = "@main() { ret void }";
/// let noise = DepolarizingNoiseModelBuilder::new()
///     .with_p1_probability(0.01)
///     .with_p2_probability(0.01);
///
/// let results = llvm_sim(llvm_ir)
///     .seed(42)
///     .noise(noise)
///     .run(1000)?;
/// # Ok::<(), pecos_core::errors::PecosError>(())
/// ```
#[must_use]
pub fn llvm_sim(llvm_ir: impl Into<String>) -> pecos_engines::SimBuilder {
    use pecos_engines::ClassicalControlEngineBuilder;
    use pecos_programs::LlvmProgram;

    llvm_engine()
        .program(LlvmProgram::from_string(llvm_ir))
        .to_sim()
}
