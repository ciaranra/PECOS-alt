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

//! Configuration structures for QASM simulation
//!
//! This module provides JSON-serializable configuration structures for
//! noise models and quantum engines used in QASM simulations.

use serde::{Deserialize, Serialize};

use crate::simulation::{
    BiasedDepolarizingNoise, DepolarizingCustomNoise, DepolarizingNoise, GeneralNoise,
    NoiseModelType, PassThroughNoise, QuantumEngineType,
};

/// Quantum engine configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuantumEngineConfig {
    /// State vector engine for general circuits
    StateVector,
    /// Sparse stabilizer engine for Clifford circuits
    SparseStabilizer,
}

impl From<QuantumEngineConfig> for QuantumEngineType {
    fn from(config: QuantumEngineConfig) -> Self {
        match config {
            QuantumEngineConfig::StateVector => QuantumEngineType::StateVector,
            QuantumEngineConfig::SparseStabilizer => QuantumEngineType::SparseStabilizer,
        }
    }
}

/// Noise model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NoiseConfig {
    /// No noise - ideal simulation
    PassThroughNoise,

    /// Standard depolarizing noise
    DepolarizingNoise {
        #[serde(default = "default_probability")]
        p: f64,
    },

    /// Custom depolarizing noise with per-operation probabilities
    DepolarizingCustomNoise {
        #[serde(default = "default_probability")]
        p_prep: f64,
        #[serde(default = "default_probability")]
        p_meas: f64,
        #[serde(default = "default_probability")]
        p1: f64,
        #[serde(default = "default_p2")]
        p2: f64,
    },

    /// Biased depolarizing noise
    BiasedDepolarizingNoise {
        #[serde(default = "default_probability")]
        p: f64,
    },

    /// General noise model
    GeneralNoise,
}

fn default_probability() -> f64 {
    0.001
}

fn default_p2() -> f64 {
    0.002
}

impl From<NoiseConfig> for NoiseModelType {
    fn from(config: NoiseConfig) -> Self {
        match config {
            NoiseConfig::PassThroughNoise => NoiseModelType::PassThrough(PassThroughNoise),
            NoiseConfig::DepolarizingNoise { p } => {
                NoiseModelType::Depolarizing(DepolarizingNoise { p })
            }
            NoiseConfig::DepolarizingCustomNoise {
                p_prep,
                p_meas,
                p1,
                p2,
            } => NoiseModelType::DepolarizingCustom(DepolarizingCustomNoise {
                p_prep,
                p_meas,
                p1,
                p2,
            }),
            NoiseConfig::BiasedDepolarizingNoise { p } => {
                NoiseModelType::BiasedDepolarizing(BiasedDepolarizingNoise { p })
            }
            NoiseConfig::GeneralNoise => NoiseModelType::General(GeneralNoise),
        }
    }
}
