/*!
Unified LLVM-based quantum simulation with support for multiple input formats.

This crate provides a flexible builder pattern API for quantum circuit simulation that accepts
LLVM IR, HUGR, or files as input. It handles the compilation pipeline automatically and provides
consistent simulation capabilities with noise models, parallelization, and multiple quantum engines.

# Example

```rust,no_run
use pecos_llvm_sim::LlvmSim;

// From LLVM IR
let results = LlvmSim::new()
    .llvm("@main() { ret void }")
    .seed(42)
    .workers(8)
    .run(1000)?;

// From HUGR
let hugr = todo!(); // Get HUGR from somewhere
let results = LlvmSim::new()
    .hugr(hugr)
    .with_depolarizing_noise(0.01)
    .run(1000)?;
# Ok::<(), pecos_core::errors::PecosError>(())
```
*/

pub mod builder;
pub mod config;
pub mod simulation;
pub mod source;

// Re-export main types
pub use builder::LlvmSim;
pub use config::{NoiseModelConfig, QuantumEngineType};
pub use simulation::LlvmSimulation;
pub use source::LlvmSource;

// Re-export from pecos-llvm-runtime for backward compatibility
pub use pecos_llvm_runtime::LlvmEngine;

// No convenience functions - use the builder directly
