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

/// Implements depolarizing channel noise for quantum simulations
///
/// The depolarizing channel randomly applies Pauli errors (X, Y, Z) to qubits with
/// specified probability, simulating quantum decoherence effects:
///
/// - X errors: Bit-flips (|0⟩ ↔ |1⟩)
/// - Y errors: Combined bit and phase flips
/// - Z errors: Phase-flips
///
/// Each error type is applied with equal probability (p/3), giving a total error rate of p.
///
/// # Usage
///
/// ```rust
/// use pecos_engines::engines::monte_carlo::MonteCarloEngine;
/// use pecos_engines::engines::monte_carlo::engine::ExternalClassicalEngine;
/// use pecos_engines::engines::quantum::StateVecEngine;
/// use pecos_engines::engines::noise::DepolarizingNoise;
/// use pecos_engines::engines::noise::NoiseModel;
///
/// // With Monte Carlo engine
/// let classical_engine = Box::new(ExternalClassicalEngine::new());
/// let quantum_engine = Box::new(StateVecEngine::new(2));
///
/// let mut engine = MonteCarloEngine::builder()
///     .with_classical_engine(classical_engine)
///     .with_quantum_engine(quantum_engine)
///     .with_depolarizing_noise(0.01) // 1% noise rate
///     .build();
///
/// // Directly
/// let mut noise_model = DepolarizingNoise::new(0.05); // 5% error rate
/// noise_model.set_seed(42).unwrap(); // For reproducibility
/// ```
#[derive(Clone)]
pub struct DepolarizingNoise {
    /// Probability of applying a random Pauli error
    probability: f64,
    /// Shared random number generator
    rng: Arc<Mutex<ChaCha8Rng>>,
}

impl DepolarizingNoise {
    /// Create a new depolarizing noise model with the given probability
    /// of applying a random Pauli error.
    #[must_use]
    pub fn new(probability: f64) -> Self {
        Self::new_with_options(probability)
    }

    /// Create a new depolarizing noise model with the given probability.
    ///
    /// # Arguments
    /// * `probability` - Probability of applying a random Pauli error
    ///
    /// # Note
    /// To set a specific seed for deterministic behavior, use the `set_seed` method
    /// after creating the noise model.
    #[must_use]
    pub fn new_with_options(probability: f64) -> Self {
        // Create an RNG from entropy (for non-deterministic behavior by default)
        let rng = ChaCha8Rng::from_os_rng();

        Self {
            probability,
            rng: Arc::new(Mutex::new(rng)),
        }
    }

    /// Set the probability of applying a random Pauli error
    ///
    /// # Arguments
    ///
    /// * `probability` - New probability value (between 0.0 and 1.0)
    ///
    /// # Panics
    ///
    /// Panics if the probability is not between 0 and 1.
    pub fn set_probability(&mut self, probability: f64) {
        assert!(
            (0.0..=1.0).contains(&probability),
            "Probability must be between 0 and 1"
        );
        self.probability = probability;
    }

    /// Get the current probability of applying a random Pauli error
    #[must_use]
    pub fn probability(&self) -> f64 {
        self.probability
    }

    /// Create a new builder for the depolarizing noise model
    #[must_use]
    pub fn builder() -> DepolarizingNoiseBuilder {
        DepolarizingNoiseBuilder::new()
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
                    if gate.qubits.len() >= 2 {
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

            // Apply gate error with probability `self.probability`
            let mut rng = self.rng.lock().unwrap();
            if rng.random::<f64>() < self.probability {
                // Choose a random error type (X, Y, or Z)
                let error_type = rng.random_range(0..3);
                match error_type {
                    0 => {
                        trace!("Applying X noise to qubit {}", gate.qubits[0]);
                        builder.add_x(&[gate.qubits[0]]);
                    }
                    1 => {
                        trace!("Applying Y noise to qubit {}", gate.qubits[0]);
                        builder.add_y(&[gate.qubits[0]]);
                    }
                    _ => {
                        trace!("Applying Z noise to qubit {}", gate.qubits[0]);
                        builder.add_z(&[gate.qubits[0]]);
                    }
                }
            }
        }

        builder.build()
    }
}

impl crate::engines::noise::NoiseModel for DepolarizingNoise {
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

impl RngManageable for DepolarizingNoise {
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
            "DepolarizingNoise stores its RNG behind an Arc<Mutex> and cannot return a direct reference"
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
            "DepolarizingNoise stores its RNG behind an Arc<Mutex> and cannot return a direct mutable reference"
        )
    }
}

/// Builder for creating depolarizing noise models
pub struct DepolarizingNoiseBuilder {
    probability: Option<f64>,
    seed: Option<u64>,
}

impl Default for DepolarizingNoiseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DepolarizingNoiseBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            probability: None,
            seed: None,
        }
    }

    /// Set the probability of applying a random Pauli error
    #[must_use]
    pub fn with_probability(mut self, probability: f64) -> Self {
        self.probability = Some(probability);
        self
    }

    /// Set the seed for the random number generator
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Build the depolarizing noise model
    ///
    /// # Panics
    ///
    /// Panics if the probability is not set or is not between 0 and 1.
    #[must_use]
    pub fn build(self) -> Box<dyn NoiseModel> {
        let probability = self.probability.expect("Probability must be set");
        assert!(
            (0.0..=1.0).contains(&probability),
            "Probability must be between 0 and 1"
        );

        let mut noise = DepolarizingNoise::new_with_options(probability);

        // Apply the seed if specified
        if let Some(seed) = self.seed {
            // Explicitly call the NoiseModel trait's set_seed method
            <DepolarizingNoise as NoiseModel>::set_seed(&mut noise, seed)
                .expect("Failed to set seed for DepolarizingNoise");
        }

        Box::new(noise)
    }
}

impl crate::engines::ControlEngine for DepolarizingNoise {
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        input: Self::Input,
    ) -> Result<crate::engines::EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // For quantum operations, apply gate noise
        trace!("DepolarizingNoise::start - applying noise to quantum operations");

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
        // Depolarizing noise doesn't modify measurement results, just pass through
        trace!("DepolarizingNoise::continue_processing - passing through measurement results");
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
    fn test_probability_getter_and_setter() {
        // Create a noise model with initial probability
        let mut noise = DepolarizingNoise::new(0.01);

        // Check initial probability
        assert!((noise.probability() - 0.01).abs() < f64::EPSILON);

        // Update probability and check it was updated
        noise.set_probability(0.05);
        assert!((noise.probability() - 0.05).abs() < f64::EPSILON);

        // Update to boundary values
        noise.set_probability(0.0);
        assert!((noise.probability() - 0.0).abs() < f64::EPSILON);

        noise.set_probability(1.0);
        assert!((noise.probability() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    #[should_panic(expected = "Probability must be between 0 and 1")]
    fn test_invalid_probability_panics() {
        let mut noise = DepolarizingNoise::new(0.5);
        noise.set_probability(1.1); // Should panic
    }

    #[test]
    fn test_builder_with_probability() {
        // Create a noise model with the builder
        let mut noise = DepolarizingNoise::builder().with_probability(0.3).build();

        // Create a direct instance with the same probability
        let mut direct_noise = DepolarizingNoise::new(0.3);

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

    #[test]
    fn test_as_any_methods() {
        // Create a noise model
        let mut noise = DepolarizingNoise::new(0.01);

        // Test as_any for type checking
        assert!(noise.as_any().is::<DepolarizingNoise>());

        // Test as_any_mut for downcasting and modifying
        let downcast_noise = noise
            .as_any_mut()
            .downcast_mut::<DepolarizingNoise>()
            .unwrap();
        downcast_noise.set_probability(0.05);
        assert!((noise.probability() - 0.05).abs() < f64::EPSILON);

        // Test with boxed trait object
        let mut boxed_noise: Box<dyn NoiseModel> = Box::new(DepolarizingNoise::new(0.01));
        assert!(boxed_noise.as_any().is::<DepolarizingNoise>());

        // Downcast and modify through the boxed trait object
        let downcast_boxed = boxed_noise
            .as_any_mut()
            .downcast_mut::<DepolarizingNoise>()
            .unwrap();
        downcast_boxed.set_probability(0.05);

        // Verify that we can't downcast to a different type
        assert!(boxed_noise.as_any_mut().downcast_mut::<String>().is_none());
    }
}
