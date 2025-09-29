/*!
Unified LLVM-based quantum simulation using the unified engine builder API.

This crate provides the `qis_engine()` function that integrates with
the unified PECOS simulation API. The engine builder handles LLVM IR (QIS format) compilation and
provides consistent simulation capabilities with noise models, parallelization, and
multiple quantum engines.

# Example

```rust,no_run
use pecos_qis_sim::qis_engine;
use pecos_programs::QisProgram;
use pecos_engines::noise::DepolarizingNoiseModelBuilder;
use pecos_engines::ClassicalControlEngineBuilder;

// Using the engine builder API with the unified simulation
let results = qis_engine()
    .program(QisProgram::from_string("@main() { ret void }"))
    .to_sim()
    .seed(42)
    .workers(8)
    .noise(DepolarizingNoiseModelBuilder::new().with_p1_probability(0.01))
    .run(1000)?;
# Ok::<(), pecos_core::errors::PecosError>(())
```
*/

pub mod engine_builder;
pub mod prelude;
pub mod source;

// Re-export main types for the unified API
pub use engine_builder::{QisEngineBuilder, qis_engine};

// Re-export from pecos-qis-runtime for backward compatibility
pub use pecos_qis_runtime::QisEngine;
