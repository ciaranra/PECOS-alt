use crate::channels::ByteMessage;
use crate::engines::noise::{NoiseModel, PassThroughNoise};
use crate::engines::{Engine, QuantumEngine};
use crate::errors::QueueError;
use dyn_clone;

/// A system that combines a noise model with a quantum engine
///
/// This system implements the `Engine` trait to provide a standardized
/// way of applying noise models to quantum engines.
pub struct QuantumSystem {
    noise_model: Box<dyn NoiseModel>,
    quantum_engine: Box<dyn QuantumEngine>,
}

impl QuantumSystem {
    /// Creates a new `QuantumSystem` with the specified noise model and quantum engine
    #[must_use]
    pub fn new(noise_model: Box<dyn NoiseModel>, quantum_engine: Box<dyn QuantumEngine>) -> Self {
        Self {
            noise_model,
            quantum_engine,
        }
    }

    /// Creates a new `QuantumSystem` with a custom quantum engine and no noise
    #[must_use]
    pub fn new_without_noise(quantum_engine: Box<dyn QuantumEngine>) -> Self {
        Self::new(Box::new(PassThroughNoise), quantum_engine)
    }

    /// Set a specific seed for both the quantum engine and noise model
    ///
    /// This method sets different but deterministic seeds for both the quantum engine
    /// and the noise model (if it supports seeding). The noise model's seed is derived
    /// from the base seed using a standard seed derivation protocol to ensure they don't
    /// produce correlated random sequences.
    ///
    /// This is the preferred method for users who need deterministic behavior from
    /// the entire quantum system. It handles the complexity of seeding multiple
    /// components with different but related seeds to avoid correlation issues.
    ///
    /// # Arguments
    /// * `seed` - Base seed value for the random number generators
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns a `QueueError` if setting the seed fails for either component
    ///
    /// # Implementation Note
    /// This method uses the `derive_seed` function to create a different seed for
    /// the noise model, ensuring that the quantum engine and noise model have
    /// uncorrelated random sequences even when using the same base seed.
    pub fn set_seed(&mut self, seed: u64) -> Result<(), QueueError> {
        // Derive a different seed for the noise model using the standard protocol
        let noise_seed = pecos_core::sims_rngs::rng_manageable::derive_seed(seed, "noise_model");

        // Set the seed for the quantum engine
        self.quantum_engine.set_seed(seed)?;

        // Set the seed for the noise model
        self.noise_model.set_seed(noise_seed)?;

        Ok(())
    }

    /// Returns a reference to the noise model
    #[must_use]
    pub fn noise_model(&self) -> &dyn NoiseModel {
        &*self.noise_model
    }

    /// Returns a mutable reference to the noise model
    #[must_use]
    pub fn noise_model_mut(&mut self) -> &mut dyn NoiseModel {
        &mut *self.noise_model
    }

    /// Returns a reference to the quantum engine
    #[must_use]
    pub fn quantum_engine(&self) -> &dyn QuantumEngine {
        &*self.quantum_engine
    }

    /// Returns a mutable reference to the quantum engine
    #[must_use]
    pub fn quantum_engine_mut(&mut self) -> &mut dyn QuantumEngine {
        &mut *self.quantum_engine
    }
}

impl Engine for QuantumSystem {
    type Input = ByteMessage;
    type Output = ByteMessage;

    fn process(&mut self, input: Self::Input) -> Result<Self::Output, QueueError> {
        // Apply noise to the input
        let noisy_input = self.noise_model.apply_noise(input)?;

        // Process the noisy input through the quantum engine
        self.quantum_engine.process(noisy_input)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        self.noise_model.reset()?;
        self.quantum_engine.reset()
    }
}

impl Clone for QuantumSystem {
    fn clone(&self) -> Self {
        QuantumSystem {
            noise_model: dyn_clone::clone_box(&*self.noise_model),
            quantum_engine: dyn_clone::clone_box(&*self.quantum_engine),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::byte::builder::ByteMessageBuilder;
    use crate::engines::noise::{DepolarizingNoise, PassThroughNoise};
    use crate::engines::quantum::StateVecEngine;
    /// Creates a new `QuantumSystem` with a state vector quantum engine and depolarizing noise
    ///
    /// # Parameters
    /// - `num_qubits`: Number of qubits for the quantum engine
    /// - `probability`: Probability parameter for the depolarizing noise model (between 0.0 and 1.0)
    ///
    /// # Returns
    /// A new `QuantumSystem` configured with the specified parameters
    #[must_use]
    pub fn create_quantume_system_with_state_vec_and_depolarizing_noise(
        num_qubits: usize,
        probability: f64,
    ) -> QuantumSystem {
        // Create a quantum engine using a state vector simulator
        let quantum_engine = Box::new(StateVecEngine::new(num_qubits));

        // Create a QuantumSystem with depolarizing noise
        QuantumSystem::new(
            Box::new(DepolarizingNoise::new_with_options(probability)),
            quantum_engine,
        )
    }

    /// Creates a new `QuantumSystem` with a state vector quantum engine and depolarizing noise with a specific seed
    ///
    /// This function first creates a quantum system with the specified number of qubits and
    /// depolarizing noise probability, then sets the seed using the `set_seed` method, which
    /// handles the derivation of component-specific seeds.
    ///
    /// # Parameters
    /// - `num_qubits`: Number of qubits for the quantum engine
    /// - `probability`: Probability parameter for the depolarizing noise model (between 0.0 and 1.0)
    /// - `seed`: Seed value for the random number generators
    ///
    /// # Returns
    /// A new `QuantumSystem` configured with the specified parameters and seeded randomness
    #[must_use]
    pub fn create_quantume_system_with_state_vec_and_depolarizing_noise_with_seed(
        num_qubits: usize,
        probability: f64,
        seed: u64,
    ) -> QuantumSystem {
        // Create a quantum engine
        let quantum_engine = Box::new(StateVecEngine::new(num_qubits));

        let mut system = // Create a QuantumSystem with depolarizing noise
        QuantumSystem::new(
            Box::new(DepolarizingNoise::new_with_options(probability)),
            quantum_engine,
        );

        system
            .set_seed(seed)
            .expect("Failed to set seed for system");

        system
    }

    /// Test that verifies the ability to update the probability of a depolarizing noise model
    #[test]
    fn test_access_and_update_noise_model() {
        // Create a quantum system with 2 qubits and 1% depolarizing noise
        let mut system = create_quantume_system_with_state_vec_and_depolarizing_noise(2, 0.01);

        // Get a reference to the noise model and verify it's a DepolarizingNoise
        let noise_model = system.noise_model();
        assert!(noise_model.as_any().is::<DepolarizingNoise>());

        // Create a simple quantum circuit with an X gate on qubit 0
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let input = builder.build();

        // Process the input with 1% noise
        let _result1 = system
            .process(input.clone())
            .expect("Failed to process input with initial noise");

        // Get a mutable reference to the noise model and update the probability
        if let Some(depolarizing_noise) = system
            .noise_model_mut()
            .as_any_mut()
            .downcast_mut::<DepolarizingNoise>()
        {
            depolarizing_noise.set_probability(0.05);
        } else {
            panic!("Failed to downcast noise model to DepolarizingNoise");
        }

        // Process the same input with 5% noise
        let _result2 = system
            .process(input)
            .expect("Failed to process input with updated noise");

        // Verify that a system with PassThroughNoise cannot be downcast to DepolarizingNoise
        let mut system_without_noise =
            QuantumSystem::new_without_noise(Box::new(StateVecEngine::new(2)));

        // Verify the noise model is not a DepolarizingNoise
        assert!(
            system_without_noise
                .noise_model()
                .as_any()
                .is::<PassThroughNoise>()
        );
        assert!(
            !system_without_noise
                .noise_model()
                .as_any()
                .is::<DepolarizingNoise>()
        );

        // Attempt to downcast to DepolarizingNoise should fail
        assert!(
            system_without_noise
                .noise_model_mut()
                .as_any_mut()
                .downcast_mut::<DepolarizingNoise>()
                .is_none()
        );
    }

    /// Test that verifies the seed management functionality
    #[test]
    fn test_seed_management() {
        // Create two quantum systems with the same seed
        let seed = 42u64;
        let mut system1 =
            create_quantume_system_with_state_vec_and_depolarizing_noise_with_seed(2, 0.5, seed);
        let mut system2 =
            create_quantume_system_with_state_vec_and_depolarizing_noise_with_seed(2, 0.5, seed);

        // Create a simple quantum circuit with a Hadamard gate and measurement
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_h(&[0]);
        builder.add_measurements(&[0], &[0]);
        let input = builder.build();

        // Process the input with both systems - they should produce the same results
        let result1 = system1
            .process(input.clone())
            .expect("Failed to process input with system1");
        let result2 = system2
            .process(input.clone())
            .expect("Failed to process input with system2");

        // Extract and compare measurement results
        let meas1 = result1
            .parse_measurements()
            .expect("Failed to parse measurement results from system1");
        let meas2 = result2
            .parse_measurements()
            .expect("Failed to parse measurement results from system2");

        assert_eq!(
            meas1, meas2,
            "Systems with the same seed should produce the same results"
        );

        // Now create a system with a different seed
        let different_seed = 43u64;
        let mut system3 = create_quantume_system_with_state_vec_and_depolarizing_noise_with_seed(
            2,
            0.5,
            different_seed,
        );

        // Reset system1 and set it to use the different seed
        system1.reset().expect("Failed to reset system1");
        system1
            .set_seed(different_seed)
            .expect("Failed to set seed for system1");

        // Process the input again with system1 and system3
        let result1 = system1
            .process(input.clone())
            .expect("Failed to process input with system1 after seed change");
        let result3 = system3
            .process(input)
            .expect("Failed to process input with system3");

        // Extract and compare measurement results
        let meas1 = result1
            .parse_measurements()
            .expect("Failed to parse measurement results from system1");
        let meas3 = result3
            .parse_measurements()
            .expect("Failed to parse measurement results from system3");

        assert_eq!(
            meas1, meas3,
            "System1 with updated seed should match system3"
        );
    }
}
