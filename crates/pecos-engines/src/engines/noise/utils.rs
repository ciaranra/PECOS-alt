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

//! Utility functions for noise models.
//!
//! The main responsibility of this file is to generate random operations
//! and convert them to quantum gates (suitable for adding to the `ByteMessage`).

#![allow(clippy::too_many_arguments)]

use crate::byte_message::{ByteMessage, ByteMessageBuilder, QuantumGate};
use crate::engines::noise::sampler::Sampler;
use crate::errors::QueueError;
use pecos_core::RngManageable;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// Default maximum qubit index to pre-cache for noise sampling
/// This is a reasonable default that balances memory usage with performance
/// A thread-safe wrapper for random number generators used in noise models
///
/// This struct encapsulates the common pattern of using an Arc<Mutex<ChaCha8Rng>>
/// for thread-safe access to the random number generator across all noise models.
///
/// It provides methods for common RNG operations and implements the `RngManageable` trait.
#[derive(Clone, Debug)]
pub struct NoiseRng {
    rng: Arc<Mutex<ChaCha8Rng>>,
}

impl NoiseRng {
    /// Create a new `NoiseRng` with a random seed
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: Arc::new(Mutex::new(ChaCha8Rng::from_os_rng())),
        }
    }

    /// Create a new `NoiseRng` with a specific seed
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: Arc::new(Mutex::new(ChaCha8Rng::seed_from_u64(seed))),
        }
    }

    /// Generate a random float between 0.0 and 1.0
    ///
    /// # Returns
    /// A random f64 value between 0.0 and 1.0
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    #[must_use]
    pub fn random_float(&self) -> f64 {
        let mut rng = self.rng.lock().unwrap();
        rng.random::<f64>()
    }

    /// Check if an event should occur with the given probability
    ///
    /// # Arguments
    /// * `probability` - The probability of the event occurring (between 0.0 and 1.0)
    ///
    /// # Returns
    /// true if the event should occur, false otherwise
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    #[must_use]
    pub fn occurs(&self, probability: f64) -> bool {
        self.random_float() < probability
    }

    /// Generate a random integer in the given range
    ///
    /// # Arguments
    /// * `range` - The range of values to choose from (inclusive start, exclusive end)
    ///
    /// # Returns
    /// A random integer in the specified range
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    #[must_use]
    pub fn random_int(&self, range: Range<usize>) -> usize {
        let mut rng = self.rng.lock().unwrap();
        rng.random_range(range)
    }

    /// Choose a key from a `HashMap` based on weighted probabilities
    ///
    /// # Arguments
    /// * `weighted_map` - `HashMap` where each key has a corresponding probability weight
    ///
    /// # Returns
    /// The chosen key, or None if the map is empty or all weights are zero
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    #[must_use]
    pub fn choose_weighted<K: Clone>(&self, weighted_map: &HashMap<K, f64>) -> Option<K> {
        if weighted_map.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: f64 = weighted_map.values().sum();
        if total_weight <= 0.0 {
            return None;
        }

        // Generate a random value between 0 and total_weight
        let rand_val = self.random_float() * total_weight;

        // Select a key based on weighted probability
        let mut cumulative = 0.0;
        for (key, weight) in weighted_map {
            cumulative += weight;
            if rand_val <= cumulative {
                return Some(key.clone());
            }
        }

        // If we get here, return the last key (should be rare due to floating-point precision)
        weighted_map.keys().next().cloned()
    }

    /// Set the seed for the random number generator
    ///
    /// This is a convenience method that wraps `RngManageable::set_seed` but returns
    /// a `QueueError` instead of `Box<dyn Error>` for backward compatibility.
    ///
    /// # Arguments
    /// * `seed` - The seed value
    ///
    /// # Returns
    /// `Ok(())` if successful
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    pub fn set_seed(&mut self, seed: u64) -> Result<(), QueueError> {
        // This implementation directly sets the RNG rather than using RngManageable::set_seed
        // to avoid unwrapping the Arc<Mutex<>> which would cause thread-safety issues
        let new_rng = ChaCha8Rng::seed_from_u64(seed);
        self.rng = Arc::new(Mutex::new(new_rng));
        Ok(())
    }

    /// Generate a random u32 in the given range
    ///
    /// # Arguments
    /// * `range` - The range of values to choose from (inclusive start, exclusive end)
    ///
    /// # Returns
    /// A random u32 in the specified range
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    #[must_use]
    pub fn random_u32(&self, range: Range<u32>) -> u32 {
        let mut rng = self.rng.lock().unwrap();
        rng.random_range(range)
    }
}

impl Default for NoiseRng {
    fn default() -> Self {
        Self::new()
    }
}

impl RngManageable for NoiseRng {
    type Rng = ChaCha8Rng;

    fn set_rng(&mut self, rng: ChaCha8Rng) -> Result<(), Box<dyn std::error::Error>> {
        self.rng = Arc::new(Mutex::new(rng));
        Ok(())
    }

    fn rng(&self) -> &Self::Rng {
        panic!("NoiseRng uses Arc<Mutex<>> and cannot provide a direct reference")
    }

    fn rng_mut(&mut self) -> &mut Self::Rng {
        panic!("NoiseRng uses Arc<Mutex<>> and cannot provide a direct mutable reference")
    }
}

/// Helper trait for validating probability values
pub trait ProbabilityValidator {
    /// Validate that a probability is between 0.0 and 1.0
    ///
    /// # Arguments
    /// * `probability` - The probability value to validate
    ///
    /// # Panics
    /// Panics if the probability is not between 0.0 and 1.0
    fn validate_probability(probability: f64) {
        assert!(
            (0.0..=1.0).contains(&probability),
            "Probability must be between 0.0 and 1.0"
        );
    }

    /// Validate a named probability
    ///
    /// # Arguments
    /// * `probability` - The probability value to validate
    /// * `name` - Name of the probability for error message
    ///
    /// # Panics
    /// Panics if the probability is not between 0.0 and 1.0
    fn validate_named_probability(probability: f64, name: &str) {
        assert!(
            (0.0..=1.0).contains(&probability),
            "Probability {name} must be between 0.0 and 1.0, but was {probability}"
        );
    }
}

/// Helper functions for working with quantum gates and noise
pub struct NoiseUtils;

/// Result of a single-qubit operation sampling that may include leakage
#[derive(Debug)]
pub struct SingleQubitNoiseResult {
    /// Optional quantum gate to apply (None if only leakage)
    pub gate: Option<QuantumGate>,
    /// Whether the qubit leaked
    pub qubit_leaked: bool,
}

impl SingleQubitNoiseResult {
    /// Creates a new `SingleQubitNoiseResult` with a Pauli gate and no leakage
    #[must_use]
    pub fn with_gate(gate: QuantumGate) -> Self {
        Self {
            gate: Some(gate),
            qubit_leaked: false,
        }
    }

    /// Creates a new `SingleQubitNoiseResult` with leakage and no gate
    #[must_use]
    pub fn with_leakage() -> Self {
        Self {
            gate: None,
            qubit_leaked: true,
        }
    }

    /// Whether this result includes leakage
    #[must_use]
    pub fn has_leakage(&self) -> bool {
        self.qubit_leaked
    }

    #[must_use]
    pub fn has_leakages(&self) -> Vec<bool> {
        vec![self.qubit_leaked]
    }
}

/// Result of a two-qubit operation sampling that may include leakage
#[derive(Debug)]
pub struct TwoQubitNoiseResult {
    /// Quantum gates to apply (None if operations are just leakage or identity)
    pub gates: Option<Vec<QuantumGate>>,
    /// Whether the first qubit leaked
    pub qubit0_leaked: bool,
    /// Whether the second qubit leaked
    pub qubit1_leaked: bool,
}

impl TwoQubitNoiseResult {
    /// Creates a new `TwoQubitNoiseResult` indicating leakage on a qubit
    #[must_use]
    pub fn with_leakage(
        qubit0_leaked: bool,
        qubit1_leaked: bool,
        gates: Option<Vec<QuantumGate>>,
    ) -> Self {
        Self {
            gates,
            qubit0_leaked,
            qubit1_leaked,
        }
    }

    /// Creates a new `TwoQubitNoiseResult` with just gates and no leakage
    #[must_use]
    pub fn with_gates(gates: Vec<QuantumGate>) -> Self {
        Self {
            gates: Some(gates),
            qubit0_leaked: false,
            qubit1_leaked: false,
        }
    }

    /// Whether any qubit leaked
    #[must_use]
    pub fn has_leakage(&self) -> bool {
        self.qubit0_leaked || self.qubit1_leaked
    }

    #[must_use]
    pub fn has_leakages(&self) -> Vec<bool> {
        vec![self.qubit0_leaked, self.qubit1_leaked]
    }

    /// Whether the first qubit leaked
    #[must_use]
    pub fn has_qubit0_leakage(&self) -> bool {
        self.qubit0_leaked
    }

    /// Whether the second qubit leaked
    #[must_use]
    pub fn has_qubit1_leakage(&self) -> bool {
        self.qubit1_leaked
    }
}

impl NoiseUtils {
    /// Add a gate to a builder based on its type
    ///
    /// This is a utility function to add a gate to a `ByteMessageBuilder`
    /// based on its type, handling all the common gate types.
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `gate` - The gate to add
    ///
    /// # Panics
    /// Panics if `gate.result_id` is `None` when processing a measurement gate.
    pub fn add_gate_to_builder(builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        use crate::byte_message::GateType;

        match gate.gate_type {
            GateType::X => {
                builder.add_x(&gate.qubits);
            }
            GateType::Y => {
                builder.add_y(&gate.qubits);
            }
            GateType::Z => {
                builder.add_z(&gate.qubits);
            }
            GateType::H => {
                builder.add_h(&gate.qubits);
            }
            GateType::CX => {
                if gate.qubits.len() >= 2 {
                    builder.add_cx(&[gate.qubits[0]], &[gate.qubits[1]]);
                }
            }
            GateType::RZZ => {
                if gate.qubits.len() >= 2 && !gate.params.is_empty() {
                    builder.add_rzz(gate.params[0], &[gate.qubits[0]], &[gate.qubits[1]]);
                }
            }
            GateType::SZZ => {
                if gate.qubits.len() >= 2 {
                    builder.add_szz(&[gate.qubits[0]], &[gate.qubits[1]]);
                }
            }
            GateType::SZZdg => {
                if gate.qubits.len() >= 2 {
                    builder.add_szzdg(&[gate.qubits[0]], &[gate.qubits[1]]);
                }
            }
            GateType::RZ => {
                if !gate.params.is_empty() {
                    builder.add_rz(gate.params[0], &gate.qubits);
                }
            }
            GateType::R1XY => {
                if gate.params.len() >= 2 {
                    builder.add_r1xy(gate.params[0], gate.params[1], &gate.qubits);
                }
            }
            GateType::Measure => {
                if !gate.qubits.is_empty() && gate.result_id.is_some() {
                    builder.add_measurements(&gate.qubits, &[gate.result_id.unwrap()]);
                }
            }
            GateType::Prep => {
                builder.add_prep(&gate.qubits);
            }
            GateType::Idle => {
                // Handle Idle gates
                let mut idle_qubits = Vec::with_capacity(gate.qubits.len());
                for &q in &gate.qubits {
                    idle_qubits.push(q);
                }
                builder.add_idle(gate.params[0], &idle_qubits);
            }
        }
    }

    /// Check if a message contains measurement results
    ///
    /// # Arguments
    /// * `message` - The `ByteMessage` to check
    ///
    /// # Returns
    /// true if the message contains measurement results, false otherwise
    #[must_use]
    pub fn has_measurements(message: &ByteMessage) -> bool {
        if let Ok(measurements) = message.parse_measurements() {
            !measurements.is_empty()
        } else {
            false
        }
    }

    /// Creates a new `ByteMessageBuilder` for quantum operations
    ///
    /// # Returns
    /// A `ByteMessageBuilder` configured for quantum operations
    #[must_use]
    pub fn create_quantum_builder() -> ByteMessageBuilder {
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder
    }

    /// Creates a new `ByteMessage` from a list of gates
    ///
    /// # Arguments
    /// * `gates` - The gates to include in the message
    ///
    /// # Returns
    /// A `ByteMessage` containing the gates
    #[must_use]
    pub fn create_gate_message(gates: &[QuantumGate]) -> ByteMessage {
        let mut builder = Self::create_quantum_builder();
        for gate in gates {
            Self::add_gate_to_builder(&mut builder, gate);
        }
        builder.build()
    }

    /// Applies X gate to a qubit via a builder
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `qubit` - The qubit to apply the gate to
    pub fn apply_x(builder: &mut ByteMessageBuilder, qubit: usize) {
        builder.add_x(&[qubit]);
    }

    /// Applies Y gate to a qubit via a builder
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `qubit` - The qubit to apply the gate to
    pub fn apply_y(builder: &mut ByteMessageBuilder, qubit: usize) {
        builder.add_y(&[qubit]);
    }

    /// Applies Z gate to a qubit via a builder
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `qubit` - The qubit to apply the gate to
    pub fn apply_z(builder: &mut ByteMessageBuilder, qubit: usize) {
        builder.add_z(&[qubit]);
    }

    /// Apply a Pauli gate based on the Pauli string identifier
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `pauli` - The Pauli gate identifier ("X", "Y", or "Z")
    /// * `qubit` - The qubit to apply the gate to
    ///
    /// # Returns
    /// `true` if a valid Pauli gate was applied, `false` otherwise
    pub fn apply_pauli(builder: &mut ByteMessageBuilder, pauli: &str, qubit: usize) -> bool {
        match pauli {
            "X" => {
                Self::apply_x(builder, qubit);
                true
            }
            "Y" => {
                Self::apply_y(builder, qubit);
                true
            }
            "Z" => {
                Self::apply_z(builder, qubit);
                true
            }
            _ => false,
        }
    }

    /// Create a Pauli gate based on the Pauli string identifier
    ///
    /// # Arguments
    /// * `pauli` - The Pauli gate identifier ("X", "Y", or "Z")
    /// * `qubit` - The qubit to apply the gate to
    ///
    /// # Returns
    /// A `Result` containing either the created `QuantumGate` or an error if an invalid Pauli is provided
    ///
    /// # Errors
    /// Returns an error if the pauli string is not one of "X", "Y", or "Z"
    pub fn create_pauli_gate(pauli: &str, qubit: usize) -> Result<QuantumGate, String> {
        // QuantumGate::try_from_pauli(pauli, qubit)
        match pauli {
            "X" => Ok(QuantumGate::x(qubit)),
            "Y" => Ok(QuantumGate::y(qubit)),
            "Z" => Ok(QuantumGate::z(qubit)),
            _ => Err(format!("Invalid Pauli operator: {pauli}")),
        }
    }

    /// This function uses the adaptive sampling method which automatically selects the most
    /// efficient sampling strategy based on the characteristics of the model.
    #[must_use]
    pub fn sample_sq_pauli_model(
        rng: &NoiseRng,
        pauli_model: &HashMap<String, f64>,
        qubit: usize,
    ) -> SingleQubitNoiseResult {
        // Use the Sampler which automatically chooses the optimal method and precision
        let sampler = Sampler::new(pauli_model);

        sampler.sample_sq_noise(rng, qubit)
    }

    /// This function uses specialized two-qubit noise sampling that's optimized for
    /// multi-qubit operations, similar to how the `Sampler` class is used for single-qubit
    /// operations.
    #[must_use]
    pub fn sample_tq_pauli_model(
        rng: &NoiseRng,
        pauli_model: &HashMap<String, f64>,
        qubit0: usize,
        qubit1: usize,
    ) -> TwoQubitNoiseResult {
        // Use the Sampler to create a two-qubit sampler
        let sampler = Sampler::new_two_qubit(pauli_model);

        sampler.sample_tq_noise(rng, qubit0, qubit1)
    }

    /// Prepares a qubit in the |0⟩ state via a builder
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `qubit` - The qubit to prepare
    pub fn apply_prep_0(builder: &mut ByteMessageBuilder, qubit: usize) {
        builder.add_prep(&[qubit]);
    }

    /// Prepares a qubit in the |1⟩ state via a builder (applies prep followed by X)
    ///
    /// # Arguments
    /// * `builder` - The `ByteMessageBuilder` to add the gate to
    /// * `qubit` - The qubit to prepare
    pub fn apply_prep_1(builder: &mut ByteMessageBuilder, qubit: usize) {
        builder.add_prep(&[qubit]);
        builder.add_x(&[qubit]);
    }

    /// Sample single-qubit operations including possible leakage based on weighted distribution
    ///
    /// # Arguments
    /// * `rng` - The random number generator to use for sampling
    /// * `pauli_leakage_model` - `HashMap` containing the weights for different operations
    /// * `qubit` - The target qubit
    ///
    /// Valid operations in the model include:
    /// - Pauli operators ("X", "Y", "Z")
    /// - "L" for leakage
    /// - "I" for identity (no operation)
    ///
    /// # Returns
    /// A `SingleQubitNoiseResult` containing:
    /// - An optional quantum gate to apply
    /// - A flag indicating whether the qubit leaked
    ///
    /// # Panics
    /// Panics if:
    /// - The `pauli_leakage_model` is empty
    /// - All operations have zero weights
    /// - Any operation string in the model is invalid
    #[must_use]
    pub fn sample_sq_pauli_leakage_model(
        rng: &NoiseRng,
        pauli_leakage_model: &HashMap<String, f64>,
        qubit: usize,
    ) -> SingleQubitNoiseResult {
        // Create and use the Sampler which handles all validation
        let sampler = Sampler::new(pauli_leakage_model);

        sampler.sample_sq_noise(rng, qubit)
    }

    /// Randomly selects a single-qubit Pauli gate (X, Y, Z) or no gate (Identity) with equal probability
    ///
    /// # Arguments
    /// * `rng` - The random number generator to use for sampling
    /// * `qubit` - The target qubit for the gate
    ///
    /// # Returns
    /// An `Option<QuantumGate>` which may contain a Pauli gate (X, Y, Z) or None (representing identity)
    ///
    /// Each of the four outcomes (X, Y, Z, Identity) has a 25% probability.
    #[must_use]
    pub fn random_pauli_or_none(rng: &NoiseRng, qubit: usize) -> Option<QuantumGate> {
        // Generate a random number between 0 and 3
        let choice = rng.random_int(0..4);

        match choice {
            0 => Some(QuantumGate::x(qubit)),
            1 => Some(QuantumGate::y(qubit)),
            2 => Some(QuantumGate::z(qubit)),
            _ => None, // Identity: no gate applied
        }
    }

    /// Sample two-qubit operations including possible leakage based on weighted distribution
    ///
    /// # Arguments
    /// * `rng` - The random number generator to use for sampling
    /// * `model` - `HashMap` containing the weights for different operations
    /// * `qubit0` - The first qubit
    /// * `qubit1` - The second qubit
    ///
    /// Valid operations in the model include:
    /// - Two-character Pauli strings ("IX", "XY", "ZZ", etc.) where each character is one of "I", "X", "Y", or "Z"
    /// - "LI", "IL" for single-qubit leakage
    /// - "LL" for leakage on both qubits
    ///
    /// # Returns
    /// A `TwoQubitNoiseResult` containing:
    /// - Any quantum gates to apply (based on non-identity, non-leakage parts of the operation)
    /// - Flags indicating whether each qubit leaked
    ///
    /// # Panics
    /// Panics if:
    /// - The `model` is empty
    /// - All operations have zero weights
    /// - Any operation string in the model is invalid
    /// - "II" (identity on both qubits) is included
    #[must_use]
    pub fn sample_tq_pauli_leakage_model(
        rng: &NoiseRng,
        model: &HashMap<String, f64>,
        qubit0: usize,
        qubit1: usize,
    ) -> TwoQubitNoiseResult {
        // Use the Sampler to create a two-qubit sampler
        let sampler = Sampler::new_two_qubit(model);

        sampler.sample_tq_noise(rng, qubit0, qubit1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_message::GateType;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    // Constants used in multiple tests
    const SAMPLE_SIZE: usize = 10000;

    #[test]
    fn test_noise_rng_random_float() {
        let rng = NoiseRng::with_seed(42);
        let value = rng.random_float();
        assert!((0.0..=1.0).contains(&value));

        // Test with multiple calls to ensure we get different values
        let values: Vec<f64> = (0..10).map(|_| rng.random_float()).collect();

        // Don't use a HashSet for floats, instead check that at least some values are different
        let mut all_same = true;
        for i in 1..values.len() {
            if (values[0] - values[i]).abs() > f64::EPSILON {
                all_same = false;
                break;
            }
        }
        assert!(!all_same, "Random values should vary");
    }

    #[test]
    fn test_noise_rng_occurs() {
        let rng = NoiseRng::with_seed(42);

        // With probability 0, should never occur
        for _ in 0..100 {
            assert!(!rng.occurs(0.0));
        }

        // With probability 1, should always occur
        for _ in 0..100 {
            assert!(rng.occurs(1.0));
        }

        // With probability 0.5, should occur roughly half the time
        let occurs_count = (0..1000).filter(|_| rng.occurs(0.5)).count();
        assert!(occurs_count > 400 && occurs_count < 600);
    }

    #[test]
    fn test_noise_rng_random_int() {
        let rng = NoiseRng::with_seed(42);

        // Test with a range of 0..3
        for _ in 0..100 {
            let value = rng.random_int(0..3);
            assert!(value < 3);
        }

        // Check distribution with a larger number of samples
        let counts = (0..1000)
            .map(|_| rng.random_int(0..3))
            .fold([0, 0, 0], |mut acc, val| {
                acc[val] += 1;
                acc
            });

        // Each value should appear roughly 1/3 of the time
        for count in &counts {
            assert!(*count > 250 && *count < 400);
        }
    }

    #[test]
    fn test_noise_utils_create_quantum_builder() {
        let mut builder = NoiseUtils::create_quantum_builder();
        let message = builder.build();
        let result = message.parse_quantum_operations();
        assert!(result.is_ok());
    }

    #[test]
    fn test_noise_utils_create_gate_message() {
        use crate::byte_message::GateType;
        use crate::byte_message::QuantumGate;

        let gates = vec![
            QuantumGate {
                gate_type: GateType::X,
                qubits: vec![0],
                params: vec![],
                result_id: None,
                noiseless: false,
            },
            QuantumGate {
                gate_type: GateType::Y,
                qubits: vec![1],
                params: vec![],
                result_id: None,
                noiseless: false,
            },
        ];

        let message = NoiseUtils::create_gate_message(&gates);
        let parsed_gates = message.parse_quantum_operations().unwrap();
        assert_eq!(parsed_gates.len(), 2);
    }

    #[test]
    fn test_prep_functions() {
        use crate::byte_message::GateType;

        // Test preparation to |0⟩
        let mut builder = NoiseUtils::create_quantum_builder();
        NoiseUtils::apply_prep_0(&mut builder, 0);
        let message = builder.build();
        let parsed_gates = message.parse_quantum_operations().unwrap();

        // Should have one Prep gate
        assert_eq!(parsed_gates.len(), 1);
        assert_eq!(parsed_gates[0].gate_type, GateType::Prep);
        assert_eq!(parsed_gates[0].qubits, vec![0]);

        // Test preparation to |1⟩
        let mut builder = NoiseUtils::create_quantum_builder();
        NoiseUtils::apply_prep_1(&mut builder, 1);
        let message = builder.build();
        let parsed_gates = message.parse_quantum_operations().unwrap();

        // Should have two gates: Prep followed by X
        assert_eq!(parsed_gates.len(), 2);
        assert_eq!(parsed_gates[0].gate_type, GateType::Prep);
        assert_eq!(parsed_gates[0].qubits, vec![1]);
        assert_eq!(parsed_gates[1].gate_type, GateType::X);
        assert_eq!(parsed_gates[1].qubits, vec![1]);
    }

    #[test]
    fn test_sample_paulis() {
        let rng = NoiseRng::with_seed(42);

        // Test with a valid model
        let valid_model: HashMap<String, f64> = [
            ("X".to_string(), 0.5),
            ("Y".to_string(), 0.3),
            ("Z".to_string(), 0.2),
        ]
        .iter()
        .cloned()
        .collect();

        // Sample multiple times to ensure different outcomes
        let mut x_count = 0;
        let mut y_count = 0;
        let mut z_count = 0;

        for _ in 0..1000 {
            let gate = NoiseUtils::sample_sq_pauli_model(&rng, &valid_model, 0);

            match gate.gate {
                Some(gate) => match gate.gate_type {
                    GateType::X => x_count += 1,
                    GateType::Y => y_count += 1,
                    GateType::Z => z_count += 1,
                    _ => panic!("Unexpected gate type"),
                },
                None => panic!("Unexpected result: None"),
            }
        }

        // Given our weights, we expect roughly 50% X, 30% Y, 20% Z
        assert!(
            x_count > 400 && x_count < 600,
            "Expected ~500 X gates, got {x_count}"
        );
        assert!(
            y_count > 250 && y_count < 350,
            "Expected ~300 Y gates, got {y_count}"
        );
        assert!(
            z_count > 150 && z_count < 250,
            "Expected ~200 Z gates, got {z_count}"
        );

        // Test that invalid Pauli gate directly panics
        let result = catch_unwind(|| NoiseUtils::create_pauli_gate("INVALID", 0).unwrap());
        assert!(result.is_err(), "Should panic for invalid Pauli operator");

        // Test empty model should panic
        let empty_model: HashMap<String, f64> = HashMap::new();
        let result = catch_unwind(AssertUnwindSafe(|| {
            NoiseUtils::sample_sq_pauli_model(&rng, &empty_model, 0)
        }));
        assert!(result.is_err(), "Should panic for empty model");

        // Test model with all zero weights should panic
        let zero_weights: HashMap<String, f64> = [
            ("X".to_string(), 0.0),
            ("Y".to_string(), 0.0),
            ("Z".to_string(), 0.0),
        ]
        .iter()
        .cloned()
        .collect();
        let result = catch_unwind(AssertUnwindSafe(|| {
            NoiseUtils::sample_sq_pauli_model(&rng, &zero_weights, 0)
        }));
        assert!(result.is_err(), "Should panic for zero-weight model");
    }

    #[test]
    fn test_random_pauli_or_none() {
        use crate::byte_message::GateType;

        // Define margin for tests
        let margin = SAMPLE_SIZE / 20; // Allow 5% margin of error
        let expected = SAMPLE_SIZE / 4; // With equal 25% probability

        let rng = NoiseRng::with_seed(42);

        // Sample many times to check the distribution
        let mut x_count = 0;
        let mut y_count = 0;
        let mut z_count = 0;
        let mut none_count = 0;

        for _ in 0..SAMPLE_SIZE {
            match NoiseUtils::random_pauli_or_none(&rng, 0) {
                Some(gate) => match gate.gate_type {
                    GateType::X => x_count += 1,
                    GateType::Y => y_count += 1,
                    GateType::Z => z_count += 1,
                    _ => panic!("Unexpected gate type"),
                },
                None => none_count += 1,
            }
        }

        // Calculate absolute difference without using .abs()
        assert!(
            x_count.max(expected) - x_count.min(expected) < margin,
            "X count {x_count} deviates too much from expected {expected}"
        );
        assert!(
            y_count.max(expected) - y_count.min(expected) < margin,
            "Y count {y_count} deviates too much from expected {expected}"
        );
        assert!(
            z_count.max(expected) - z_count.min(expected) < margin,
            "Z count {z_count} deviates too much from expected {expected}"
        );
        assert!(
            none_count.max(expected) - none_count.min(expected) < margin,
            "None count {none_count} deviates too much from expected {expected}"
        );

        // Verify the sum is correct
        assert_eq!(x_count + y_count + z_count + none_count, SAMPLE_SIZE);
    }

    #[test]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::too_many_lines,
        clippy::no_effect_underscore_binding
    )]
    fn test_sample_tq_pauli_model() {
        use crate::byte_message::GateType;

        let rng = NoiseRng::with_seed(42);

        // Test with a valid two-qubit Pauli model
        let valid_model: HashMap<String, f64> = [
            ("IX".to_string(), 0.2),
            ("IY".to_string(), 0.1),
            ("IZ".to_string(), 0.1),
            ("XI".to_string(), 0.1),
            ("XX".to_string(), 0.1),
            ("XY".to_string(), 0.1),
            ("XZ".to_string(), 0.05),
            ("YI".to_string(), 0.05),
            ("YX".to_string(), 0.05),
            ("YY".to_string(), 0.05),
            ("YZ".to_string(), 0.02),
            ("ZI".to_string(), 0.02),
            ("ZX".to_string(), 0.02),
            ("ZY".to_string(), 0.02),
            ("ZZ".to_string(), 0.01),
        ]
        .iter()
        .cloned()
        .collect();

        // Count occurrences of each Pauli operation type
        let mut pauli_counts: HashMap<String, usize> = HashMap::new();

        // Count gates on each qubit
        let mut q0_x_count = 0;
        let mut _q0_y_count = 0;
        let mut _q0_z_count = 0;
        let mut _q0_i_count = 0;

        let mut q1_x_count = 0;
        let mut _q1_y_count = 0;
        let mut _q1_z_count = 0;
        let mut _q1_i_count = 0;

        let mut none_count = 0;
        let mut _one_gate_count = 0;
        let mut _two_gate_count = 0;

        // Sample multiple times to ensure proper distribution
        for _ in 0..1000 {
            let result = NoiseUtils::sample_tq_pauli_model(&rng, &valid_model, 0, 1);

            // Record the count of gates
            match &result.gates {
                None => none_count += 1, // Should never happen with our test model
                Some(gates) => match gates.len() {
                    0 => none_count += 1, // Should never happen with our test model
                    1 => _one_gate_count += 1,
                    2 => _two_gate_count += 1,
                    _ => panic!("Unexpected number of gates: {}", gates.len()),
                },
            }

            // Extract the Pauli operations from the gates
            let mut q0_op = "I";
            let mut q1_op = "I";

            if let Some(gates) = &result.gates {
                for gate in gates {
                    match gate.qubits.first() {
                        Some(&qubit) => {
                            // Handle the qubit based on its value
                            if qubit == 0 {
                                // Operations on qubit 0
                                match gate.gate_type {
                                    GateType::X => {
                                        q0_op = "X";
                                        q0_x_count += 1;
                                    }
                                    GateType::Y => q0_op = "Y",
                                    GateType::Z => q0_op = "Z",
                                    _ => panic!("Unexpected gate type on qubit 0"),
                                }
                            } else if qubit == 1 {
                                // Operations on qubit 1
                                match gate.gate_type {
                                    GateType::X => {
                                        q1_op = "X";
                                        q1_x_count += 1;
                                    }
                                    GateType::Y => q1_op = "Y",
                                    GateType::Z => q1_op = "Z",
                                    _ => panic!("Unexpected gate type on qubit 1"),
                                }
                            }
                        }
                        None => panic!("Missing qubit index"),
                    }
                }
            }

            // Construct the combined operation and update the count
            let combined_op = format!("{q0_op}{q1_op}");
            *pauli_counts.entry(combined_op.clone()).or_insert(0) += 1;
        }

        // Verify distributions
        // X operations on qubit 0 occur in XI, XX, XY, XZ (total weight 0.35)
        assert!(
            q0_x_count > 300 && q0_x_count < 400,
            "Expected ~350 X operations on qubit 0, got {q0_x_count}"
        );

        // X operations on qubit 1 occur in IX, XX, YX, ZX (total weight 0.37)
        assert!(
            q1_x_count > 320 && q1_x_count < 420,
            "Expected ~370 X operations on qubit 1, got {q1_x_count}"
        );

        // Verify that "II" was never generated (should be none_count = 0)
        assert_eq!(
            none_count, 0,
            "Got {none_count} samples with no gates, which shouldn't happen"
        );

        // Test invalid model: with "II"
        let model_with_ii: HashMap<String, f64> =
            [("II".to_string(), 1.0)].iter().cloned().collect();

        let result = catch_unwind(AssertUnwindSafe(|| {
            NoiseUtils::sample_tq_pauli_model(&rng, &model_with_ii, 0, 1)
        }));
        assert!(result.is_err(), "Should panic when model contains 'II'");

        // Test invalid model: too many characters
        let invalid_format_model: HashMap<String, f64> =
            [("XYZ".to_string(), 1.0)].iter().cloned().collect();

        let result = catch_unwind(AssertUnwindSafe(|| {
            NoiseUtils::sample_tq_pauli_model(&rng, &invalid_format_model, 0, 1)
        }));
        assert!(
            result.is_err(),
            "Should panic when model contains operations with wrong format"
        );

        // Test invalid model: invalid operator
        let invalid_op_model: HashMap<String, f64> =
            [("XQ".to_string(), 1.0)].iter().cloned().collect();

        let result = catch_unwind(AssertUnwindSafe(|| {
            NoiseUtils::sample_tq_pauli_model(&rng, &invalid_op_model, 0, 1)
        }));
        assert!(
            result.is_err(),
            "Should panic when model contains invalid Pauli operators"
        );

        // Test empty model
        let empty_model: HashMap<String, f64> = HashMap::new();
        let result = catch_unwind(AssertUnwindSafe(|| {
            NoiseUtils::sample_tq_pauli_model(&rng, &empty_model, 0, 1)
        }));
        assert!(result.is_err(), "Should panic for empty model");
    }

    #[test]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::too_many_lines,
        clippy::no_effect_underscore_binding
    )]
    fn test_sample_tq_pauli_leakage_model() {
        use crate::byte_message::GateType;

        // Define constants at the beginning
        const SAMPLE_SIZE: usize = 10000;

        let rng = NoiseRng::with_seed(42);

        // Test with a valid model including leakage
        let valid_model: HashMap<String, f64> = [
            ("X".to_string(), 0.4),
            ("Y".to_string(), 0.3),
            ("Z".to_string(), 0.2),
            ("L".to_string(), 0.1),
        ]
        .iter()
        .cloned()
        .collect();

        // Sample multiple times to test distribution
        let mut x_count = 0;
        let mut y_count = 0;
        let mut z_count = 0;
        let mut leakage_count = 0;

        for _ in 0..SAMPLE_SIZE {
            let SingleQubitNoiseResult { gate, qubit_leaked } =
                NoiseUtils::sample_sq_pauli_leakage_model(&rng, &valid_model, 0);
            if qubit_leaked {
                leakage_count += 1;
            } else if let Some(gate) = gate {
                match gate.gate_type {
                    GateType::X => x_count += 1,
                    GateType::Y => y_count += 1,
                    GateType::Z => z_count += 1,
                    _ => panic!("Unexpected gate type"),
                }
            }
        }

        // Check that the distributions are roughly as expected
        let expected_x = (SAMPLE_SIZE as f64 * 0.4) as usize;
        let expected_y = (SAMPLE_SIZE as f64 * 0.3) as usize;
        let expected_z = (SAMPLE_SIZE as f64 * 0.2) as usize;
        let expected_l = (SAMPLE_SIZE as f64 * 0.1) as usize;

        let margin = SAMPLE_SIZE / 10; // Allow 10% margin for safety

        assert!(
            x_count.max(expected_x) - x_count.min(expected_x) < margin,
            "X count {x_count} deviates too much from expected {expected_x}"
        );
        assert!(
            y_count.max(expected_y) - y_count.min(expected_y) < margin,
            "Y count {y_count} deviates too much from expected {expected_y}"
        );
        assert!(
            z_count.max(expected_z) - z_count.min(expected_z) < margin,
            "Z count {z_count} deviates too much from expected {expected_z}"
        );
        assert!(
            leakage_count.max(expected_l) - leakage_count.min(expected_l) < margin,
            "Leakage count {leakage_count} deviates too much from expected {expected_l}"
        );

        // Verify the sum is correct
        assert_eq!(x_count + y_count + z_count + leakage_count, SAMPLE_SIZE);

        // Test error cases with safe catch_unwind
        let empty_model: HashMap<String, f64> = HashMap::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // This should trigger an "empty model" panic
            let _result = NoiseUtils::sample_sq_pauli_leakage_model(&rng, &empty_model, 0);
        }));
        assert!(result.is_err(), "Empty model should cause panic");

        // Test invalid operation
        let invalid_model: HashMap<String, f64> = [
            ("X".to_string(), 0.3),
            ("INVALID".to_string(), 0.7), // Not a valid Pauli or L
        ]
        .iter()
        .cloned()
        .collect();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // This should trigger an "invalid operation" panic
            let _result = NoiseUtils::sample_sq_pauli_leakage_model(&rng, &invalid_model, 0);
        }));
        assert!(result.is_err(), "Invalid operation should cause panic");
    }
}
