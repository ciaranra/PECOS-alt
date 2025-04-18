// Copyright 2024 The PECOS Developers
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

pub mod builder;
pub mod engine;

pub use builder::HybridEngineBuilder;
pub use engine::HybridEngine;

/// Create a new `HybridEngine` with the given classical and quantum engines
///
/// # Deprecated
///
/// This function is maintained for backward compatibility.
/// New code should use `HybridEngineBuilder` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `HybridEngineBuilder` for a more flexible and maintainable API"
)]
#[must_use]
pub fn new(
    classical_engine: Box<dyn crate::engines::ClassicalEngine>,
    quantum_engine: Box<dyn crate::engines::QuantumEngine>,
) -> HybridEngine {
    HybridEngineBuilder::new()
        .with_classical_engine(classical_engine)
        .with_quantum_engine(quantum_engine)
        .build()
}

/// Create a new `HybridEngine` with the given classical engine, quantum engine, and noise model
///
/// # Deprecated
///
/// This function is maintained for backward compatibility.
/// New code should use `HybridEngineBuilder` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `HybridEngineBuilder` for a more flexible and maintainable API"
)]
#[must_use]
pub fn with_noise(
    classical_engine: Box<dyn crate::engines::ClassicalEngine>,
    quantum_engine: Box<dyn crate::engines::QuantumEngine>,
    noise_model: Box<dyn crate::engines::noise::NoiseModel>,
) -> HybridEngine {
    HybridEngineBuilder::new()
        .with_classical_engine(classical_engine)
        .with_quantum_engine(quantum_engine)
        .with_noise_model(noise_model)
        .build()
}

/// Creates a new `HybridEngine` with the specified classical engine and quantum system
///
/// # Deprecated
///
/// This function is maintained for backward compatibility.
/// New code should use `HybridEngineBuilder` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `HybridEngineBuilder` for a more flexible and maintainable API"
)]
#[must_use]
pub fn new_with_quantum_system(
    classical_engine: Box<dyn crate::engines::ClassicalEngine>,
    quantum_system: crate::quantum_system::QuantumSystem,
) -> HybridEngine {
    HybridEngineBuilder::new()
        .with_classical_engine(classical_engine)
        .with_quantum_system(quantum_system)
        .build()
}

/// Create a new `HybridEngine` with the given classical engine and a quantum system with depolarizing noise
///
/// # Deprecated
///
/// This function is maintained for backward compatibility.
/// New code should use `HybridEngineBuilder` instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `HybridEngineBuilder` for a more flexible and maintainable API"
)]
#[must_use]
pub fn with_depolarizing_noise(
    classical_engine: Box<dyn crate::engines::ClassicalEngine>,
    quantum_engine: Box<dyn crate::engines::QuantumEngine>,
    probability: f64,
) -> HybridEngine {
    HybridEngineBuilder::new()
        .with_classical_engine(classical_engine)
        .with_quantum_engine(quantum_engine)
        .with_depolarizing_noise(probability)
        .build()
}
