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

// Core traits - these are fundamental to using the unified API
pub use crate::{
    Engine,
    ClassicalEngine, 
    ControlEngine,
    ClassicalControlEngine,
    ClassicalControlEngineBuilder,  // Critical for .to_sim() method
};

// Quantum engines and builders
pub use crate::quantum::{SparseStabEngine, StateVecEngine, new_quantum_engine_arbitrary_qgate};
pub use crate::quantum_engine_builder::{state_vector, sparse_stabilizer, IntoQuantumEngineBuilder};

// Noise models - both traits and common implementations
pub use crate::noise::{
    NoiseModel,
    IntoNoiseModel,  // Needed for .noise() method to work smoothly
};
pub use crate::{
    PassThroughNoiseModel,
    DepolarizingNoiseModel,
    // Convenience structs for noise configuration
    PassThroughNoise,
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
};
pub use crate::noise::general::GeneralNoiseModel;

// Simulation builders
pub use crate::sim_builder::{sim, SimBuilder};  // For unified API

// Engine implementations
pub use crate::{
    HybridEngine,
    MonteCarloEngine,
    QuantumSystem,
    EngineSystem,
    EngineStage,
};

// Results and data structures
pub use crate::shot_results::{Data, Shot, ShotVec, ShotMap};
pub use crate::{
    ShotMapDisplay,
    ShotMapDisplayExt,
    ShotMapDisplayOptions,
    BitVecDisplayFormat,
};

// Message passing
pub use crate::{ByteMessage, ByteMessageBuilder};
pub use crate::byte_message::dump_batch;

// Legacy API (to be deprecated)
pub use crate::run_sim;

pub use serde_json::Value;
