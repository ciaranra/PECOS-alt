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

use crate::byte_message::ByteMessage;
use crate::byte_message::ByteMessageBuilder;
use crate::byte_message::{GateType, QuantumGate};
use crate::engines::noise::NoiseModel;
use crate::errors::QueueError;
use log::trace;
use pecos_core::RngManageable;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::any::Any;
use std::sync::{Arc, Mutex};

// TODO: Add idle noise

/// Implements general depolarizing channel noise for quantum simulations
///
/// This model applies different error probabilities to various quantum operations:
/// - `p_prep`: Preparation error probability
/// - `p_meas`: Measurement error probability
/// - `p1`: Single-qubit gate error probability
/// - `p2`: Two-qubit gate error probability
///
/// # Usage
///
/// ```rust
/// use pecos_engines::engines::monte_carlo::MonteCarloEngine;
/// use pecos_engines::engines::monte_carlo::engine::ExternalClassicalEngine;
/// use pecos_engines::engines::quantum::StateVecEngine;
/// use pecos_engines::engines::noise::GeneralDepolarizingNoise;
/// use pecos_engines::engines::noise::NoiseModel;
///
/// // Create with direct constructor
/// let mut noise_model = GeneralDepolarizingNoise::new(0.01, 0.02, 0.03, 0.04);
/// noise_model.set_seed(42).unwrap(); // For reproducibility
///
/// // Or use the builder pattern with separate probabilities
/// let noise_model = GeneralDepolarizingNoise::builder()
///     .with_prep_probability(0.01)
///     .with_meas_probability(0.02)
///     .with_single_qubit_probability(0.03)
///     .with_two_qubit_probability(0.04)
///     .with_seed(42)
///     .build();
///
/// // Or use the builder with uniform probability
/// let noise_model = GeneralDepolarizingNoise::builder()
///     .with_uniform_probability(0.01)
///     .build();
/// ```
#[derive(Clone)]
pub struct GeneralDepolarizingNoise {
    /// Probability of applying an error during preparation
    p_prep: f64,
    /// Probability of applying an error during measurement
    p_meas: f64,
    /// Probability of applying an error after single-qubit gates
    p1: f64,
    /// Probability of applying an error after two-qubit gates
    p2: f64,
    /// Shared random number generator
    rng: Arc<Mutex<ChaCha8Rng>>,
}

impl GeneralDepolarizingNoise {
    /// Create a new general depolarizing noise model with the given probabilities
    ///
    /// # Arguments
    /// * `p_prep` - Probability of error during preparation (0.0 to 1.0)
    /// * `p_meas` - Probability of error during measurement (0.0 to 1.0)
    /// * `p1` - Probability of error after single-qubit gates (0.0 to 1.0)
    /// * `p2` - Probability of error after two-qubit gates (0.0 to 1.0)
    ///
    /// # Returns
    /// A new `GeneralDepolarizingNoise` model
    ///
    /// # Panics
    /// Panics if any of the probabilities are not between 0.0 and 1.0
    #[must_use]
    pub fn new(p_prep: f64, p_meas: f64, p1: f64, p2: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&p_prep),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p_meas),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p1),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p2),
            "Probability must be between 0.0 and 1.0"
        );

        let rng = ChaCha8Rng::from_os_rng();

        Self {
            p_prep,
            p_meas,
            p1,
            p2,
            rng: Arc::new(Mutex::new(rng)),
        }
    }

    /// Create a new noise model with uniform probability for all error types
    ///
    /// # Arguments
    /// * `probability` - Error probability to use for all operation types (0.0 to 1.0)
    ///
    /// # Returns
    /// A new `GeneralDepolarizingNoise` model with the same probability for all error types
    ///
    /// # Panics
    /// Panics if the probability is not between 0.0 and 1.0
    #[must_use]
    pub fn new_uniform(probability: f64) -> Self {
        Self::new(probability, probability, probability, probability)
    }

    /// Create a new builder for the general depolarizing noise model
    #[must_use]
    pub fn builder() -> GeneralDepolarizingNoiseBuilder {
        GeneralDepolarizingNoiseBuilder::new()
    }

    /// Set all probabilities of error
    ///
    /// # Arguments
    ///
    /// * `p_prep` - Probability of error during preparation (0.0 to 1.0)
    /// * `p_meas` - Probability of error during measurement (0.0 to 1.0)
    /// * `p1` - Probability of error after single-qubit gates (0.0 to 1.0)
    /// * `p2` - Probability of error after two-qubit gates (0.0 to 1.0)
    ///
    /// # Panics
    ///
    /// Panics if any probability is not between 0 and 1.
    pub fn set_probabilities(&mut self, p_prep: f64, p_meas: f64, p1: f64, p2: f64) {
        assert!(
            (0.0..=1.0).contains(&p_prep),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p_meas),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p1),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p2),
            "Probability must be between 0.0 and 1.0"
        );

        self.p_prep = p_prep;
        self.p_meas = p_meas;
        self.p1 = p1;
        self.p2 = p2;
    }

    /// Set a uniform probability for all error types
    ///
    /// # Arguments
    /// * `probability` - The probability value to set for all error types
    ///
    /// # Panics
    /// Panics if the probability is not between 0.0 and 1.0
    pub fn set_uniform_probability(&mut self, probability: f64) {
        self.set_probabilities(probability, probability, probability, probability);
    }

    /// Get the current error probabilities
    ///
    /// # Returns
    /// Tuple containing (`p_prep`, `p_meas`, `p1`, `p2`)
    #[must_use]
    pub fn probabilities(&self) -> (f64, f64, f64, f64) {
        (self.p_prep, self.p_meas, self.p1, self.p2)
    }

    /// Apply noise to a list of quantum gates
    fn apply_noise_to_gates(&self, gates: &[QuantumGate]) -> ByteMessage {
        // Create a new message builder
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Process each gate
        for gate in gates {
            // First, add the original gate to the message
            match gate.gate_type {
                GateType::X | GateType::Y | GateType::Z | GateType::H | GateType::R1XY => {
                    trace!("Applying single-qubit gate with possible fault");
                    self.apply_sq_faults(&mut builder, gate);
                }
                GateType::CX | GateType::RZZ | GateType::SZZ => {
                    trace!("Applying two-qubit gate with possible fault");
                    self.apply_tq_faults(&mut builder, gate);
                }
                GateType::RZ => {
                    builder.add_quantum_gate(gate);
                }
                GateType::Measure => {
                    trace!("Applying measurement with possible fault");
                    self.apply_meas_faults(&mut builder, gate);
                }
                GateType::Prep => {
                    trace!("Applying preparation with possible fault");
                    self.apply_prep_faults(&mut builder, gate);
                }
            }
        }

        builder.build()
    }

    fn apply_prep_faults(&self, builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        builder.add_quantum_gate(gate);

        let mut rng = self.rng.lock().unwrap();

        if rng.random::<f64>() < self.p_prep {
            trace!("Applying prep fault on qubits {:?}", gate.qubits);
            builder.add_x(&gate.qubits);
        }
    }

    fn apply_meas_faults(&self, builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        let mut rng = self.rng.lock().unwrap();

        if rng.random::<f64>() < self.p_meas {
            trace!("Applying meas fault on qubits {:?}", gate.qubits);
            builder.add_x(&gate.qubits);
        }

        builder.add_quantum_gate(gate);
    }

    fn apply_sq_faults(&self, builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        builder.add_quantum_gate(gate);

        let mut rng = self.rng.lock().unwrap();

        if rng.random::<f64>() < self.p1 {
            let fault_type = rng.random_range(0..3);
            match fault_type {
                0 => {
                    trace!("Applying X fault on qubits {:?}", gate.qubits);
                    builder.add_x(&gate.qubits);
                }
                1 => {
                    trace!("Applying Y fault on qubits {:?}", gate.qubits);
                    builder.add_y(&gate.qubits);
                }
                _ => {
                    trace!("Applying Z fault on qubits {:?}", gate.qubits);
                    builder.add_z(&gate.qubits);
                }
            }
        }
    }

    fn apply_tq_faults(&self, builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        builder.add_quantum_gate(gate);

        let mut rng = self.rng.lock().unwrap();

        if rng.random::<f64>() < self.p2 {
            let fault_type = rng.random_range(0..15);
            match fault_type {
                // IX
                0 => {
                    trace!("Applying IX fault on qubits {:?}", gate.qubits);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // IY
                1 => {
                    trace!("Applying IY fault on qubits {:?}", gate.qubits);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // IZ
                2 => {
                    trace!("Applying IZ fault on qubits {:?}", gate.qubits);
                    builder.add_z(&[gate.qubits[1]]);
                }
                // XI
                3 => {
                    trace!("Applying XI fault on qubits {:?}", gate.qubits);
                    builder.add_x(&[gate.qubits[0]]);
                }
                // XX
                4 => {
                    trace!("Applying XX fault on qubits {:?}", gate.qubits);
                    builder.add_x(&[gate.qubits[0]]);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // XY
                5 => {
                    trace!("Applying XY fault on qubits {:?}", gate.qubits);
                    builder.add_x(&[gate.qubits[0]]);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // XZ
                6 => {
                    trace!("Applying XZ fault on qubits {:?}", gate.qubits);
                    builder.add_x(&[gate.qubits[0]]);
                    builder.add_z(&[gate.qubits[1]]);
                }
                // YI
                7 => {
                    trace!("Applying YI fault on qubits {:?}", gate.qubits);
                    builder.add_y(&[gate.qubits[0]]);
                }
                // YX
                8 => {
                    trace!("Applying YX fault on qubits {:?}", gate.qubits);
                    builder.add_y(&[gate.qubits[0]]);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // YY
                9 => {
                    trace!("Applying YY fault on qubits {:?}", gate.qubits);
                    builder.add_y(&[gate.qubits[0]]);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // YZ
                10 => {
                    trace!("Applying YZ fault on qubits {:?}", gate.qubits);
                    builder.add_y(&[gate.qubits[0]]);
                    builder.add_z(&[gate.qubits[1]]);
                }
                // ZI
                11 => {
                    trace!("Applying ZI fault on qubits {:?}", gate.qubits);
                    builder.add_z(&[gate.qubits[0]]);
                }
                // ZX
                12 => {
                    trace!("Applying ZX fault on qubits {:?}", gate.qubits);
                    builder.add_z(&[gate.qubits[0]]);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // ZY
                13 => {
                    trace!("Applying ZY fault on qubits {:?}", gate.qubits);
                    builder.add_z(&[gate.qubits[0]]);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // ZZ
                _ => {
                    trace!("Applying ZZ fault on qubits {:?}", gate.qubits);
                    builder.add_z(&[gate.qubits[0]]);
                    builder.add_z(&[gate.qubits[1]]);
                }
            }
        }
    }
}

impl crate::engines::noise::NoiseModel for GeneralDepolarizingNoise {
    fn set_seed(&mut self, seed: u64) -> Result<(), QueueError> {
        // Use the RngManageable trait's set_rng method to set the seed
        RngManageable::set_rng(self, ChaCha8Rng::seed_from_u64(seed))
            .map_err(|e| QueueError::OperationError(e.to_string()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl RngManageable for GeneralDepolarizingNoise {
    type Rng = ChaCha8Rng;

    /// Replace the random number generator with a new one
    ///
    /// This method allows replacing the RNG without recreating the entire noise model,
    /// preserving its current configuration.
    ///
    /// # Arguments
    /// * `rng` - A new random number generator
    ///
    /// # Returns
    /// Result indicating success or failure
    fn set_rng(&mut self, rng: ChaCha8Rng) -> Result<(), Box<dyn std::error::Error>> {
        self.rng = Arc::new(Mutex::new(rng));
        Ok(())
    }

    /// Get a read-only reference to the internal random number generator
    ///
    /// # Returns
    /// A reference to the internal RNG
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    fn rng(&self) -> &Self::Rng {
        // Since we have the RNG behind an Arc<Mutex>, we can't return a direct reference.
        // This is a limitation of the current design and should be reconsidered.
        panic!(
            "GeneralDepolarizingNoise stores its RNG behind an Arc<Mutex> and cannot return a direct reference"
        )
    }

    /// Get a mutable reference to the internal random number generator
    ///
    /// # Returns
    /// A mutable reference to the internal RNG
    ///
    /// # Panics
    /// Panics if the mutex is poisoned
    fn rng_mut(&mut self) -> &mut Self::Rng {
        // Since we have the RNG behind an Arc<Mutex>, we can't return a direct mutable reference.
        // This is a limitation of the current design and should be reconsidered.
        panic!(
            "GeneralDepolarizingNoise stores its RNG behind an Arc<Mutex> and cannot return a direct mutable reference"
        )
    }
}

/// Builder for creating general depolarizing noise models
pub struct GeneralDepolarizingNoiseBuilder {
    p_prep: Option<f64>,
    p_meas: Option<f64>,
    p1: Option<f64>,
    p2: Option<f64>,
    seed: Option<u64>,
}

impl Default for GeneralDepolarizingNoiseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GeneralDepolarizingNoiseBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            p_prep: None,
            p_meas: None,
            p1: None,
            p2: None,
            seed: None,
        }
    }

    /// Set the same probability for all error types
    ///
    /// This is a convenience method to set all probabilities to the same value.
    ///
    /// # Arguments
    /// * `probability` - The probability value to set for all error types
    #[must_use]
    pub fn with_uniform_probability(mut self, probability: f64) -> Self {
        self.p_prep = Some(probability);
        self.p_meas = Some(probability);
        self.p1 = Some(probability);
        self.p2 = Some(probability);
        self
    }

    /// Set the probability of error during preparation
    #[must_use]
    pub fn with_prep_probability(mut self, probability: f64) -> Self {
        self.p_prep = Some(probability);
        self
    }

    /// Set the probability of error during measurement
    #[must_use]
    pub fn with_meas_probability(mut self, probability: f64) -> Self {
        self.p_meas = Some(probability);
        self
    }

    /// Set the probability of error after single-qubit gates
    #[must_use]
    pub fn with_p1_probability(mut self, probability: f64) -> Self {
        self.p1 = Some(probability);
        self
    }

    /// Set the probability of error after single-qubit gates
    ///
    /// This is an alias for `with_p1_probability` for API consistency.
    #[must_use]
    pub fn with_single_qubit_probability(self, probability: f64) -> Self {
        self.with_p1_probability(probability)
    }

    /// Set the probability of error after two-qubit gates
    #[must_use]
    pub fn with_p2_probability(mut self, probability: f64) -> Self {
        self.p2 = Some(probability);
        self
    }

    /// Set the probability of error after two-qubit gates
    ///
    /// This is an alias for `with_p2_probability` for API consistency.
    #[must_use]
    pub fn with_two_qubit_probability(self, probability: f64) -> Self {
        self.with_p2_probability(probability)
    }

    /// Set the seed for the random number generator
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Build the general depolarizing noise model
    ///
    /// # Panics
    ///
    /// Panics if any probabilities are not set or are not between 0 and 1.
    #[must_use]
    pub fn build(self) -> Box<dyn NoiseModel> {
        let p_prep = self.p_prep.expect("Preparation probability must be set");
        let p_meas = self.p_meas.expect("Measurement probability must be set");
        let p1 = self.p1.expect("Single-qubit probability must be set");
        let p2 = self.p2.expect("Two-qubit probability must be set");

        let mut noise = GeneralDepolarizingNoise::new(p_prep, p_meas, p1, p2);

        // Apply the seed if specified
        if let Some(seed) = self.seed {
            // Explicitly call the NoiseModel trait's set_seed method
            <GeneralDepolarizingNoise as NoiseModel>::set_seed(&mut noise, seed)
                .expect("Failed to set seed for GeneralDepolarizingNoise");
        }

        Box::new(noise)
    }
}

impl crate::engines::ControlEngine for GeneralDepolarizingNoise {
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        input: Self::Input,
    ) -> Result<crate::engines::EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // For quantum operations, apply gate noise
        trace!("GeneralDepolarizingNoise::start - applying noise to quantum operations");

        // Parse the input as quantum operations
        let gates: Vec<crate::byte_message::QuantumGate> = input.parse_quantum_operations()?;

        // Apply noise to the gates
        let noisy_gates = self.apply_noise_to_gates(&gates);

        // Return the noisy operations
        Ok(crate::engines::EngineStage::NeedsProcessing(noisy_gates))
    }

    fn continue_processing(
        &mut self,
        result: Self::EngineOutput,
    ) -> Result<crate::engines::EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // This noise model doesn't directly modify measurement results, just pass through
        trace!(
            "GeneralDepolarizingNoise::continue_processing - passing through measurement results"
        );
        Ok(crate::engines::EngineStage::Complete(result))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // No state to reset
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::{ControlEngine, EngineStage};

    #[test]
    fn test_probabilities_getter_and_setter() {
        // Create a noise model with initial probabilities
        let mut noise = GeneralDepolarizingNoise::new(0.01, 0.02, 0.03, 0.04);

        // Check initial probabilities
        let (p_prep, p_meas, p1, p2) = noise.probabilities();
        assert!((p_prep - 0.01).abs() < f64::EPSILON);
        assert!((p_meas - 0.02).abs() < f64::EPSILON);
        assert!((p1 - 0.03).abs() < f64::EPSILON);
        assert!((p2 - 0.04).abs() < f64::EPSILON);

        // Update probabilities and check they were updated
        noise.set_probabilities(0.05, 0.06, 0.07, 0.08);
        let (p_prep, p_meas, p1, p2) = noise.probabilities();
        assert!((p_prep - 0.05).abs() < f64::EPSILON);
        assert!((p_meas - 0.06).abs() < f64::EPSILON);
        assert!((p1 - 0.07).abs() < f64::EPSILON);
        assert!((p2 - 0.08).abs() < f64::EPSILON);
    }

    #[test]
    fn test_uniform_probability() {
        // Test the uniform probability constructor
        let noise = GeneralDepolarizingNoise::new_uniform(0.05);
        let (p_prep, p_meas, p1, p2) = noise.probabilities();
        assert!((p_prep - 0.05).abs() < f64::EPSILON);
        assert!((p_meas - 0.05).abs() < f64::EPSILON);
        assert!((p1 - 0.05).abs() < f64::EPSILON);
        assert!((p2 - 0.05).abs() < f64::EPSILON);

        // Test the uniform probability setter
        let mut noise = GeneralDepolarizingNoise::new(0.01, 0.02, 0.03, 0.04);
        noise.set_uniform_probability(0.07);
        let (p_prep, p_meas, p1, p2) = noise.probabilities();
        assert!((p_prep - 0.07).abs() < f64::EPSILON);
        assert!((p_meas - 0.07).abs() < f64::EPSILON);
        assert!((p1 - 0.07).abs() < f64::EPSILON);
        assert!((p2 - 0.07).abs() < f64::EPSILON);
    }

    #[test]
    #[should_panic(expected = "Probability must be between 0.0 and 1.0")]
    fn test_invalid_probability_panics() {
        let mut noise = GeneralDepolarizingNoise::new(0.1, 0.2, 0.3, 0.4);
        noise.set_probabilities(0.1, 0.2, 1.1, 0.4); // Should panic
    }

    #[test]
    fn test_builder() {
        // Create a noise model with the builder
        let mut noise = GeneralDepolarizingNoise::builder()
            .with_prep_probability(0.1)
            .with_meas_probability(0.2)
            .with_p1_probability(0.3)
            .with_p2_probability(0.4)
            .build();

        // Create a direct instance with the same probabilities
        let mut direct_noise = GeneralDepolarizingNoise::new(0.1, 0.2, 0.3, 0.4);

        // Create a simple message for testing
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let input = builder.build();

        // Process using the ControlEngine API instead of the old apply_noise method
        let result1 = noise
            .start(input.clone())
            .expect("Builder-created noise model failed");
        let result2 = direct_noise
            .start(input)
            .expect("Directly created noise model failed");

        // Verify we got a valid result that needs processing
        match result1 {
            EngineStage::NeedsProcessing(_) => (),
            EngineStage::Complete(_) => panic!("Expected NeedsProcessing stage"),
        }

        match result2 {
            EngineStage::NeedsProcessing(_) => (),
            EngineStage::Complete(_) => panic!("Expected NeedsProcessing stage"),
        }
    }

    #[test]
    fn test_builder_with_uniform_probability() {
        // Create a noise model with the builder using uniform probability
        let noise = GeneralDepolarizingNoise::builder()
            .with_uniform_probability(0.05)
            .build();

        // Create a direct instance with the same uniform probability
        let direct_noise = GeneralDepolarizingNoise::new_uniform(0.05);

        // Check that probabilities match
        let (p_prep1, p_meas1, p1_1, p2_1) = direct_noise.probabilities();

        // Get the boxed noise model's probabilities using any_ref downcast
        let noise_ref = noise
            .as_any()
            .downcast_ref::<GeneralDepolarizingNoise>()
            .unwrap();
        let (p_prep2, p_meas2, p1_2, p2_2) = noise_ref.probabilities();

        assert!((p_prep1 - p_prep2).abs() < f64::EPSILON);
        assert!((p_meas1 - p_meas2).abs() < f64::EPSILON);
        assert!((p1_1 - p1_2).abs() < f64::EPSILON);
        assert!((p2_1 - p2_2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_as_any_methods() {
        // Create a noise model
        let mut noise = GeneralDepolarizingNoise::new(0.1, 0.2, 0.3, 0.4);

        // Test as_any for type checking
        assert!(noise.as_any().is::<GeneralDepolarizingNoise>());

        // Test as_any_mut for downcasting and modifying
        let downcast_noise = noise
            .as_any_mut()
            .downcast_mut::<GeneralDepolarizingNoise>()
            .unwrap();
        downcast_noise.set_probabilities(0.5, 0.5, 0.5, 0.5);

        let (p_prep, p_meas, p1, p2) = noise.probabilities();
        assert!((p_prep - 0.5).abs() < f64::EPSILON);
        assert!((p_meas - 0.5).abs() < f64::EPSILON);
        assert!((p1 - 0.5).abs() < f64::EPSILON);
        assert!((p2 - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_builder_with_probability() {
        // Create a noise model with the builder
        let mut noise = GeneralDepolarizingNoise::builder()
            .with_prep_probability(0.01)
            .with_meas_probability(0.02)
            .with_p1_probability(0.03)
            .with_p2_probability(0.04)
            .build();

        // Create a direct instance with the same probabilities
        let mut direct_noise = GeneralDepolarizingNoise::new(0.01, 0.02, 0.03, 0.04);

        // Create a simple quantum operations message for testing
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let input = builder.build();

        // Process using the ControlEngine API instead of the old apply_noise method
        let result1 = noise
            .start(input.clone())
            .expect("Builder-created noise model failed");
        let result2 = direct_noise
            .start(input)
            .expect("Directly created noise model failed");

        // Verify we got a valid result that needs processing
        match result1 {
            EngineStage::NeedsProcessing(_) => (),
            EngineStage::Complete(_) => panic!("Expected NeedsProcessing stage"),
        }

        match result2 {
            EngineStage::NeedsProcessing(_) => (),
            EngineStage::Complete(_) => panic!("Expected NeedsProcessing stage"),
        }
    }
}
