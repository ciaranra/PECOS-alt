use crate::byte_message::QuantumGate;
use crate::engines::noise::NoiseRng;
use crate::engines::noise::utils::{SingleQubitNoiseResult, TwoQubitNoiseResult};
use rand::distr::weighted::WeightedIndex;
use std::collections::HashMap;

/// Tolerance for weight normalization - total weights should be within this amount of 1.0
const NORMALIZATION_TOLERANCE: f64 = 1e-5;
/// Small margin for floating-point equality comparisons
const FLOAT_EPSILON: f64 = 1e-10;

/// A sampler that selects keys with probability proportional to their weights
#[derive(Debug, Clone)]
pub struct WeightedSampler<K: Clone> {
    keys: Vec<K>,
    distribution: WeightedIndex<f64>,
    weighted_map: HashMap<K, f64>,
}

impl<K: Clone + std::fmt::Debug + std::hash::Hash + Eq> WeightedSampler<K> {
    /// Create a new sampler from a `HashMap` with default tolerance
    ///
    /// # Panics
    /// - If the weighted map is empty
    /// - If the total weight is not positive
    /// - If the total weight deviates from 1.0 by more than the tolerance
    /// - If the weighted index distribution cannot be created
    #[must_use]
    pub fn new(weighted_map: &HashMap<K, f64>) -> Self {
        Self::new_with_tolerance(weighted_map, NORMALIZATION_TOLERANCE)
    }

    /// Create a new sampler with custom tolerance
    ///
    /// # Panics
    /// - If the weighted map is empty
    /// - If the total weight is not positive
    /// - If the total weight deviates from 1.0 by more than the tolerance
    /// - If the weighted index distribution cannot be created
    #[must_use]
    pub fn new_with_tolerance(weighted_map: &HashMap<K, f64>, tolerance: f64) -> Self {
        let (normalized_weighted_map, normalized_weights) =
            Self::validate_and_normalize(weighted_map, tolerance);

        let keys: Vec<K> = weighted_map.keys().cloned().collect();

        let distribution = WeightedIndex::new(&normalized_weights)
            .expect("WeightedSampler: failed to create weighted distribution");

        WeightedSampler {
            keys,
            distribution,
            weighted_map: normalized_weighted_map,
        }
    }

    /// Validates that the weights are positive and approximately sum to 1.0
    /// Returns a normalized `HashMap` and a Vec of normalized weights for creating the distribution
    fn validate_and_normalize(
        weighted_map: &HashMap<K, f64>,
        tolerance: f64,
    ) -> (HashMap<K, f64>, Vec<f64>) {
        assert!(
            !weighted_map.is_empty(),
            "WeightedSampler: weighted_map cannot be empty"
        );

        let total_weight: f64 = weighted_map.values().sum();

        assert!(
            total_weight > 0.0,
            "WeightedSampler: total weight must be positive, got {total_weight}"
        );

        // Check if weights are within tolerance of 1.0
        assert!(
            (total_weight - 1.0).abs() <= tolerance, // Use <= instead of !( > )
            "WeightedSampler: total weight {total_weight} deviates from 1.0 by more than tolerance {tolerance}"
        );

        let normalized_weights = if (total_weight - 1.0).abs() > FLOAT_EPSILON {
            // Within tolerance but not exactly 1.0 - normalize
            weighted_map.values().map(|&w| w / total_weight).collect()
        } else {
            // Already exactly 1.0 (within floating point precision)
            weighted_map.values().copied().collect()
        };

        // Create normalized HashMap
        let mut normalized_map = HashMap::with_capacity(weighted_map.len());
        for (key, &value) in weighted_map {
            normalized_map.insert(
                key.clone(),
                if (total_weight - 1.0).abs() < FLOAT_EPSILON {
                    value
                } else {
                    value / total_weight
                },
            );
        }

        (normalized_map, normalized_weights)
    }

    /// Sample from the weighted distribution and return the corresponding key
    ///
    /// # Arguments
    /// * `rng` - Random number generator for sampling
    ///
    /// # Returns
    /// A random key selected according to the weights
    #[must_use]
    pub fn sample(&self, rng: &NoiseRng) -> K {
        let idx = rng.sample_from_distribution(&self.distribution);
        self.keys[idx].clone()
    }

    /// Get a reference to the normalized weighted map
    #[must_use]
    pub fn get_weighted_map(&self) -> &HashMap<K, f64> {
        &self.weighted_map
    }
}

/// Helper function to create a Pauli gate for a qubit
fn create_pauli_gate(op: char, qubit: usize) -> Option<QuantumGate> {
    match op {
        'X' => Some(QuantumGate::x(qubit)),
        'Y' => Some(QuantumGate::y(qubit)),
        'Z' => Some(QuantumGate::z(qubit)),
        _ => None,
    }
}

/// Samples single qubit noise operations (Pauli gates or leakage)
#[derive(Clone, Debug)]
pub struct SingleQubitWeightedSampler {
    sampler: WeightedSampler<String>,
}

impl SingleQubitWeightedSampler {
    /// Create a new single qubit sampler from a weighted map
    ///
    /// Valid keys are: "X", "Y", "Z", "L" (for leakage)
    ///
    /// # Panics
    /// - If the weighted map contains invalid keys
    /// - If the weighted map is empty
    /// - If the total weight is not positive
    /// - If the total weight deviates from 1.0 by more than the tolerance
    #[must_use]
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
                    "SingleQubitWeightedSampler: invalid key '{key_str}' - must be one of \"X\", \"Y\", \"Z\", or \"L\""
                ),
            }
        }
    }

    /// Get a reference to the normalized weighted map
    #[must_use]
    pub fn get_weighted_map(&self) -> &HashMap<String, f64> {
        self.sampler.get_weighted_map()
    }

    /// Sample a raw key from the distribution
    #[must_use]
    pub fn sample_keys(&self, rng: &NoiseRng) -> String {
        self.sampler.sample(rng)
    }

    /// Sample a gate operation for the given qubit
    ///
    /// # Panics
    /// - If the sampled key is invalid (this should never happen if the sampler was created properly)
    #[must_use]
    pub fn sample_gates(&self, rng: &NoiseRng, qubit: usize) -> SingleQubitNoiseResult {
        let key = self.sample_keys(rng);

        match key.as_str() {
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
                "SingleQubitWeightedSampler: invalid key '{key}' - must be one of \"X\", \"Y\", \"Z\", or \"L\""
            ),
        }
    }
}

/// Samples two-qubit noise operations (pairs of Pauli gates or leakage)
#[derive(Clone, Debug)]
pub struct TwoQubitWeightedSampler {
    sampler: WeightedSampler<String>,
}

impl TwoQubitWeightedSampler {
    /// Create a new two-qubit sampler from a weighted map
    ///
    /// Valid keys are two-character strings where each character is one of:
    /// "X", "Y", "Z", "I" (identity), or "L" (leakage)
    /// Note: "II" is not allowed as it represents no operation
    ///
    /// # Panics
    /// - If the weighted map contains invalid keys
    /// - If the weighted map is empty
    /// - If the total weight is not positive
    /// - If the total weight deviates from 1.0 by more than the tolerance
    #[must_use]
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
            assert_eq!(
                key_str.len(),
                2,
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
            assert_ne!(
                key_str, "II",
                "TwoQubitWeightedSampler: key 'II' is not allowed as it represents no operation"
            );
        }
    }

    /// Get a reference to the normalized weighted map
    #[must_use]
    pub fn get_weighted_map(&self) -> &HashMap<String, f64> {
        self.sampler.get_weighted_map()
    }

    /// Sample a raw key from the distribution
    fn sample_keys(&self, rng: &NoiseRng) -> String {
        self.sampler.sample(rng)
    }

    /// Sample gate operations for the given qubit pair
    ///
    /// # Panics
    /// - If the sampled key is invalid (this should never happen if the sampler was created properly)
    #[must_use]
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

        // If both qubits leaked, return early
        if qubit0_leaked && qubit1_leaked {
            return TwoQubitNoiseResult::with_leakage(true, true, None);
        }

        // Build gates based on the operations
        let mut gates = Vec::new();

        // Add gates for non-leaked qubits with non-identity operations
        if !qubit0_leaked {
            if let Some(gate) = create_pauli_gate(op0, qubit0) {
                gates.push(gate);
            }
        }

        if !qubit1_leaked {
            if let Some(gate) = create_pauli_gate(op1, qubit1) {
                gates.push(gate);
            }
        }

        let gates = if gates.is_empty() { None } else { Some(gates) };

        TwoQubitNoiseResult::with_leakage(qubit0_leaked, qubit1_leaked, gates)
    }
}
