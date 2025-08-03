// Copyright 2025 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! A prelude for users of the `pecos-llvm-sim` crate.
//!
//! This prelude re-exports the most commonly used types, traits, and functions
//! needed for LLVM-based quantum simulation in PECOS.
//!
//! ## Usage
//!
//! ```
//! use pecos_llvm_sim::prelude::*;
//!
//! // Now you can use all LLVM simulation types and functions
//! ```

// Main entry points for LLVM simulation
pub use crate::{llvm_engine, llvm_sim, LlvmEngineBuilder};

// Re-export LlvmEngine from pecos-llvm-runtime
pub use pecos_llvm_runtime::LlvmEngine;

// Program types
pub use pecos_programs::LlvmProgram;

// Engine traits - especially ClassicalControlEngineBuilder for .to_sim() (sim_builder() preferred)
pub use pecos_engines::{
    ClassicalControlEngineBuilder,
    ClassicalEngine,
    Engine,
};

// Noise models - convenience structs
pub use pecos_engines::{
    DepolarizingNoise,
    BiasedDepolarizingNoise,
    PassThroughNoise,
};

// Quantum engine builders
pub use pecos_engines::quantum_engine_builder::{state_vector, sparse_stabilizer};

// Simulation builder for unified API
pub use pecos_engines::sim_builder;

// Result types
pub use pecos_engines::{Shot, ShotVec, ShotMap};

// Error type
pub use pecos_core::errors::PecosError;