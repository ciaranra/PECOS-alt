#![allow(dead_code)]
#![allow(clippy::same_functions_in_if_condition)]
#![allow(clippy::missing_panics_doc)]

use crate::byte_message::QuantumGate;
use crate::engines::noise::NoiseRng;
use crate::engines::noise::utils::{SingleQubitNoiseResult, TwoQubitNoiseResult};
use rand::distr::weighted::WeightedIndex;
use std::collections::HashMap;

// Define a constant for the normalization tolerance
const NORMALIZATION_TOLERANCE: f64 = 1e-5;

pub struct WeightedSampler<K: Clone> {
    keys: Vec<K>,
    distribution: WeightedIndex<f64>,
}

impl<K: Clone + std::fmt::Debug> WeightedSampler<K> {
    // Create a new sampler from a HashMap with default tolerance
    pub fn new(weighted_map: &HashMap<K, f64>) -> Self {
        Self::new_with_tolerance(weighted_map, NORMALIZATION_TOLERANCE)
    }

    // Create a new sampler with custom tolerance
    pub fn new_with_tolerance(weighted_map: &HashMap<K, f64>, tolerance: f64) -> Self {
        let normalized_weights = Self::validate_and_normalize(weighted_map, tolerance);

        let keys: Vec<K> = weighted_map.keys().cloned().collect();

        let distribution = WeightedIndex::new(&normalized_weights)
            .expect("WeightedSampler: failed to create weighted distribution");

        WeightedSampler { keys, distribution }
    }

    fn validate_and_normalize(weighted_map: &HashMap<K, f64>, tolerance: f64) -> Vec<f64> {
        assert!(
            !weighted_map.is_empty(),
            "WeightedSampler: weighted_map cannot be empty"
        );

        let total_weight: f64 = weighted_map.values().sum();

        assert!(
            total_weight > 0.0,
            "WeightedSampler: total weight must be positive, got {total_weight}"
        );

        // Check if weights need normalization
        if (total_weight - 1.0).abs() > tolerance {
            // Outside tolerance - panic with a clear message
            panic!(
                "WeightedSampler: total weight {total_weight} deviates from 1.0 by more than tolerance {tolerance}"
            );
        } else if (total_weight - 1.0).abs() > tolerance {
            // Within tolerance but not exactly 1.0 - normalize
            weighted_map.values().map(|&w| w / total_weight).collect()
        } else {
            // Already exactly 1.0 (within floating point precision)
            weighted_map.values().copied().collect()
        }
    }

    /// Sample from a precomputed `WeightedIndex` distribution with f64 weights
    ///
    /// # Arguments
    /// * `distribution` - A precomputed `WeightedIndex` distribution with f64 weights
    ///
    /// # Returns
    /// A random index selected according to the weights
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    pub fn sample(&self, rng: &NoiseRng) -> K {
        let idx = rng.sample_from_distribution(&self.distribution);
        // let mut guard = rng.get_guard();
        // let idx = self.distribution.sample(&mut *guard);
        self.keys[idx].clone()
    }
}

// Helper function to create a Pauli gate for a qubit
fn create_pauli_gate(op: char, qubit: usize) -> Option<QuantumGate> {
    match op {
        'X' => Some(QuantumGate::x(qubit)),
        'Y' => Some(QuantumGate::y(qubit)),
        'Z' => Some(QuantumGate::z(qubit)),
        _ => None, // Should never happen due to validation
    }
}

struct SingleQubitWeightedSampler {
    sampler: WeightedSampler<String>,
}

impl SingleQubitWeightedSampler {
    pub fn new(weighted_map: &HashMap<String, f64>) -> Self {
        Self::validate_pauli_leakage_keys(weighted_map);

        Self {
            sampler: WeightedSampler::new(weighted_map),
        }
    }

    fn validate_pauli_leakage_keys(weighted_map: &HashMap<String, f64>) {
        for key in weighted_map.keys() {
            let key_str = key.as_ref();
            match key_str {
                "X" | "Y" | "Z" | "L" => {} // Valid keys
                _ => panic!(
                    "WeightedSampler: invalid key '{key_str}' - must be one of \"X\", \"Y\", \"Z\", \"L\", or \"I\""
                ),
            }
        }
    }

    pub fn sample_keys(&self, rng: &NoiseRng) -> String {
        self.sampler.sample(rng)
    }

    pub fn sample_gates(&self, rng: &NoiseRng, qubit: usize) -> SingleQubitNoiseResult {
        let binding = self.sample_keys(rng).clone();
        let key_str = binding.as_ref();

        match key_str {
            "X" => SingleQubitNoiseResult {
                gate: Some(QuantumGate::x(qubit)),
                qubit_leaked: false,
            },
            "Y" => SingleQubitNoiseResult {
                gate: Some(QuantumGate::y(qubit)),
                qubit_leaked: false,
            },
            "Z" => SingleQubitNoiseResult {
                gate: Some(QuantumGate::z(qubit)),
                qubit_leaked: false,
            },
            "L" => SingleQubitNoiseResult {
                gate: None,
                qubit_leaked: true,
            },
            _ => panic!(
                "WeightedSampler: invalid key '{key_str}' - must be one of \"X\", \"Y\", \"Z\", \"L\", or \"I\""
            ),
        }
    }
}

pub struct TwoQubitWeightedSampler {
    sampler: WeightedSampler<String>,
}

impl TwoQubitWeightedSampler {
    pub fn new(weighted_map: &HashMap<String, f64>) -> Self {
        Self::validate_two_qubit_keys(weighted_map);

        Self {
            sampler: WeightedSampler::new(weighted_map),
        }
    }

    fn validate_two_qubit_keys(weighted_map: &HashMap<String, f64>) {
        for key in weighted_map.keys() {
            let key_str: &str = key.as_ref();

            // Key should be exactly 2 characters long
            assert!(
                (key_str.len() == 2),
                "TwoQubitWeightedSampler: invalid key '{key_str}' - must be exactly 2 characters"
            );

            // Each character should be one of the valid operators
            let chars: Vec<char> = key_str.chars().collect();
            for &c in &chars {
                match c {
                    'X' | 'Y' | 'Z' | 'I' | 'L' => {} // Valid characters
                    _ => panic!(
                        "TwoQubitWeightedSampler: invalid character '{c}' in key '{key_str}' - each character must be one of \"X\", \"Y\", \"Z\", \"I\", or \"L\""
                    ),
                }
            }

            // Special case: "II" is not allowed (it would represent no operation)
            assert!(
                (key_str != "II"),
                "TwoQubitWeightedSampler: key 'II' is not allowed as it represents no operation"
            );
        }
    }

    pub fn sample_keys(&self, rng: &NoiseRng) -> String {
        self.sampler.sample(rng)
    }

    pub fn sample_gates(
        &self,
        rng: &NoiseRng,
        qubit0: usize,
        qubit1: usize,
    ) -> TwoQubitNoiseResult {
        let key_str = self.sample_keys(rng);

        // Extract the two characters from the key
        let chars: Vec<char> = key_str.chars().collect();
        let op0 = chars[0];
        let op1 = chars[1];

        // Check for leakage
        let qubit0_leaked = op0 == 'L';
        let qubit1_leaked = op1 == 'L';

        // If both qubits leaked, return early with the helper method
        if qubit0_leaked && qubit1_leaked {
            return TwoQubitNoiseResult::with_leakage(true, true, None);
        }

        // Build gates based on the operations
        let mut gates = Vec::new();

        // Add gates for non-leaked qubits with non-identity operations
        if !qubit0_leaked && op0 != 'I' {
            if let Some(gate) = create_pauli_gate(op0, qubit0) {
                gates.push(gate);
            }
        }

        if !qubit1_leaked && op1 != 'I' {
            if let Some(gate) = create_pauli_gate(op1, qubit1) {
                gates.push(gate);
            }
        }

        // If we have gates but no leakage, use the with_gates helper
        if !qubit0_leaked && !qubit1_leaked && !gates.is_empty() {
            TwoQubitNoiseResult::with_gates(gates)
        } else if !gates.is_empty() {
            // Gates with leakage
            TwoQubitNoiseResult::with_leakage(qubit0_leaked, qubit1_leaked, Some(gates))
        } else {
            // No gates, just leakage
            TwoQubitNoiseResult::with_leakage(qubit0_leaked, qubit1_leaked, None)
        }
    }
}
