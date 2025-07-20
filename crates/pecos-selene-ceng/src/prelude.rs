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

//! A prelude for users of the `pecos-selene-ceng` crate.
//!
//! This prelude re-exports the most commonly used types, traits, and functions
//! needed for Selene-based classical control engines in PECOS.
//!
//! ## Usage
//!
//! ```
//! use pecos_selene_ceng::prelude::*;
//!
//! // Now you can use all Selene engine types and functions
//! ```

// Main entry points for Selene engines
pub use crate::{selene_engine, SeleneEngine, SeleneEngineBuilder};

// Program types
pub use crate::program::SeleneProgram;
pub use pecos_programs::{LlvmProgram, HugrProgram};

// Engine traits - especially ClassicalControlEngineBuilder for .to_sim()
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
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
    PassThroughNoise,
};

// Quantum engines
pub use pecos_engines::quantum::{StateVecEngine, SparseStabEngine};

// Result types
pub use pecos_engines::{Shot, ShotVec, ShotMap};

// Error types
pub use crate::error::SeleneError;
pub use pecos_core::errors::PecosError;