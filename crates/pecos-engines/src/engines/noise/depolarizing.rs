use crate::channels::ByteMessage;
use crate::channels::byte::builder::MessageBuilder;
use crate::channels::byte::gate_type::{GateTypeId, QuantumGate};
use crate::engines::noise::NoiseModel;
use crate::errors::QueueError;
use log::trace;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// Depolarizing noise model
///
/// This noise model applies random Pauli errors (X, Y, Z) to qubits
/// with a specified probability.
#[derive(Clone)]
pub struct DepolarizingNoise {
    /// Probability of applying a random Pauli error
    probability: f64,
    /// Shared random number generator
    rng: Arc<Mutex<StdRng>>,
}

impl DepolarizingNoise {
    /// Create a new depolarizing noise model with the given probability
    /// of applying a random Pauli error.
    #[must_use]
    pub fn new(probability: f64) -> Self {
        Self {
            probability,
            rng: Arc::new(Mutex::new(StdRng::from_os_rng())),
        }
    }

    /// Create a new depolarizing noise model with custom options
    ///
    /// # Arguments
    ///
    /// * `probability` - Probability of applying a random Pauli error (between 0.0 and 1.0)
    /// * `seed` - Optional seed for the random number generator
    #[must_use]
    pub fn new_with_options(probability: f64, seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        };

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
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Process each gate
        for gate in gates {
            // First, add the original gate to the message
            match gate.gate_type {
                GateTypeId::X => {
                    builder.add_x(&gate.qubits);
                }
                GateTypeId::Y => {
                    builder.add_y(&gate.qubits);
                }
                GateTypeId::Z => {
                    builder.add_z(&gate.qubits);
                }
                GateTypeId::H => {
                    builder.add_h(&gate.qubits);
                }
                GateTypeId::CX => {
                    if gate.qubits.len() >= 2 {
                        builder.add_cx(&[gate.qubits[0]], &[gate.qubits[1]]);
                    }
                }
                GateTypeId::RZZ => {
                    if gate.qubits.len() >= 2 {
                        builder.add_rzz(gate.params[0], &[gate.qubits[1]], &[gate.qubits[2]]);
                    }
                }
                GateTypeId::SZZ => {
                    if gate.qubits.len() >= 2 {
                        builder.add_szz(&[gate.qubits[0]], &[gate.qubits[1]]);
                    }
                }
                GateTypeId::RZ => {
                    if !gate.params.is_empty() {
                        builder.add_rz(gate.params[0], &gate.qubits);
                    }
                }
                GateTypeId::R1XY => {
                    if gate.params.len() >= 2 {
                        builder.add_r1xy(gate.params[0], gate.params[1], &gate.qubits);
                    }
                }
                GateTypeId::Measure => {
                    if !gate.qubits.is_empty() && gate.result_id.is_some() {
                        builder.add_measurements(&gate.qubits, &[gate.result_id.unwrap()]);
                    }
                }
                GateTypeId::Prep => {
                    builder.add_prep(&gate.qubits);
                }
            }

            // Apply random noise to each qubit with probability p
            let mut rng = self.rng.lock().unwrap();
            for &qubit in &gate.qubits {
                if rng.random::<f64>() < self.probability {
                    // Choose a random Pauli error (X, Y, or Z)
                    let error_type = rng.random_range(0..3);
                    match error_type {
                        0 => {
                            trace!("Applying X noise to qubit {}", qubit);
                            builder.add_x(&[qubit]);
                        }
                        1 => {
                            trace!("Applying Y noise to qubit {}", qubit);
                            builder.add_y(&[qubit]);
                        }
                        _ => {
                            trace!("Applying Z noise to qubit {}", qubit);
                            builder.add_z(&[qubit]);
                        }
                    }
                }
            }
        }

        builder.build()
    }
}

impl NoiseModel for DepolarizingNoise {
    fn apply_noise(&self, message: ByteMessage) -> Result<ByteMessage, QueueError> {
        // Parse the commands from the message
        let gates = message.parse_quantum_operations()?;

        // Apply noise to the commands
        Ok(self.apply_noise_to_gates(&gates))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // No state to reset
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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

        Box::new(DepolarizingNoise::new_with_options(probability, self.seed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probability_getter_and_setter() {
        // Create a noise model with initial probability
        let mut noise = DepolarizingNoise::new(0.01);

        // Check initial probability
        assert_eq!(noise.probability(), 0.01);

        // Update probability and check it was updated
        noise.set_probability(0.05);
        assert_eq!(noise.probability(), 0.05);

        // Update to boundary values
        noise.set_probability(0.0);
        assert_eq!(noise.probability(), 0.0);

        noise.set_probability(1.0);
        assert_eq!(noise.probability(), 1.0);
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
        let noise = DepolarizingNoise::builder().with_probability(0.3).build();

        // Create a direct instance with the same probability
        let direct_noise = DepolarizingNoise::new(0.3);

        // Apply noise to a simple message and verify both produce similar results
        // (We can't check exact equality due to randomness, but we can verify the builder works)
        let mut builder = MessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let input = builder.build();

        // Just verify that both can process the input without errors
        let _result1 = noise
            .apply_noise(input.clone())
            .expect("Builder-created noise model failed");
        let _result2 = direct_noise
            .apply_noise(input)
            .expect("Directly created noise model failed");
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
        assert_eq!(noise.probability(), 0.05);

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
