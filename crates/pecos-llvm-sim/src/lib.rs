/*!
Unified LLVM-based quantum simulation with support for multiple input formats.

This crate provides a flexible builder pattern API for quantum circuit simulation that accepts
LLVM IR, HUGR, or files as input. It handles the compilation pipeline automatically and provides
consistent simulation capabilities with noise models, parallelization, and multiple quantum engines.

# Example

```rust,no_run
use pecos_llvm_sim::LlvmSim;

// From LLVM IR
use pecos_llvm_sim::{llvm_sim, DepolarizingNoise, QuantumEngineType};

let results = llvm_sim()
    .llvm_ir("@main() { ret void }")
    .seed(42)
    .workers(8)
    .noise(DepolarizingNoise { p: 0.01 })
    .quantum_engine(QuantumEngineType::StateVector)
    .run(1000)?;

// From HUGR
let hugr = todo!(); // Get HUGR from somewhere
let results = llvm_sim()
    .hugr(hugr)
    .noise(DepolarizingNoise { p: 0.01 })
    .run(1000)?;
# Ok::<(), pecos_core::errors::PecosError>(())
```
*/

pub mod builder;
pub mod config;
pub mod engine_builder;
pub mod simulation;
pub mod source;

// Re-export main types
pub use builder::LlvmSim;
pub use config::{
    NoiseModelConfig, QuantumEngineType,
    PassThroughNoise, DepolarizingNoise, DepolarizingCustomNoise, BiasedDepolarizingNoise,
};
pub use engine_builder::{llvm_engine, LlvmEngineBuilder};
pub use simulation::LlvmSimulation;
pub use source::LlvmSource;

// Re-export from pecos-llvm-runtime for backward compatibility
pub use pecos_llvm_runtime::LlvmEngine;

/// Convenience function to create a new LLVM simulation builder.
///
/// This provides a consistent API with qasm_sim() and selene_sim().
///
/// # Example
/// ```rust,no_run
/// use pecos_llvm_sim::{llvm_sim, DepolarizingNoise, QuantumEngineType};
///
/// let results = llvm_sim()
///     .llvm_ir("@main() { ret void }")
///     .seed(42)
///     .noise(DepolarizingNoise { p: 0.01 })
///     .quantum_engine(QuantumEngineType::StateVector)
///     .run(1000)?;
/// # Ok::<(), pecos_core::errors::PecosError>(())
/// ```
pub fn llvm_sim() -> LlvmSim {
    LlvmSim::new()
}
