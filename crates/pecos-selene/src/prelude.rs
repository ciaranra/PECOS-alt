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

//! A prelude for users of the `pecos-selene` crate.
//!
//! This prelude re-exports the most commonly used types, traits, and functions
//! needed for Selene integration in PECOS.
//!
//! ## Usage
//!
//! ```
//! use pecos_selene::prelude::*;
//!
//! // Now you can use all Selene types and functions
//! ```

// Main entry points for Selene engines
pub use crate::{selene_simple_runtime, SeleneSimpleRuntimeEngine, SeleneSimpleRuntimeEngineBuilder};

// Program types
pub use crate::program::SeleneProgram;
pub use pecos_programs::{LlvmProgram, HugrProgram};

// Engine traits - especially ClassicalControlEngineBuilder for .to_sim() (sim_builder() preferred)
pub use pecos_engines::{
    ClassicalControlEngineBuilder,
    ClassicalEngine,
    ControlEngine,
    Engine,
};

// For hybrid engines
pub use pecos_engines::hybrid::HybridEngineBuilder;

// Noise models - convenience structs
pub use pecos_engines::{
    DepolarizingNoise,
    BiasedDepolarizingNoise,
    PassThroughNoise,
};

// Quantum engines
pub use pecos_engines::quantum::{StateVecEngine, SparseStabEngine};

// Simulation builder for unified API
pub use pecos_engines::sim_builder;

// Result types
pub use pecos_engines::{Shot, ShotVec, ShotMap};

// Error types
pub use crate::error::SeleneError;
pub use pecos_core::errors::PecosError;