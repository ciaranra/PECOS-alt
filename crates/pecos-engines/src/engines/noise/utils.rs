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

use crate::byte_message::{ByteMessage, ByteMessageBuilder, QuantumGate};
use crate::errors::QueueError;
use pecos_core::RngManageable;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// A thread-safe wrapper for random number generators used in noise models
///
/// This struct encapsulates the common pattern of using an Arc<Mutex<ChaCha8Rng>>
/// for thread-safe access to the random number generator across all noise models.
///
/// It provides methods for common RNG operations and implements the `RngManageable` trait.
#[derive(Clone)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
