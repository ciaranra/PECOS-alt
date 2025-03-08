use crate::channels::ByteMessage;
use crate::engines::noise::{DepolarizingNoise, NoiseModel, PassThroughNoise};
use crate::engines::quantum::StateVecEngine;
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

    /// Creates a new `QuantumSystem` with a custom quantum engine and depolarizing noise
    #[must_use]
    pub fn new_with_depolarizing_noise(
        quantum_engine: Box<dyn QuantumEngine>,
        probability: f64,
    ) -> Self {
        Self::new(
            Box::new(DepolarizingNoise::new_with_options(probability, None)),
            quantum_engine,
        )
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
    QuantumSystem::new_with_depolarizing_noise(quantum_engine, probability)
}
