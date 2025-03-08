use crate::channels::ByteMessage;
use crate::engines::noise::{DepolarizingNoise, NoiseModel, PassThroughNoise};
use crate::engines::quantum::new_quantum_engine_arbitrary_qgate;
use crate::engines::{EngineSystem, QuantumEngine};
use dyn_clone;
use pecos_qsim::StateVec;

/// A system that combines a noise model with a quantum engine
///
/// This system implements the `EngineSystem` trait to provide a standardized
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

impl EngineSystem for QuantumSystem {
    type Controller = Box<dyn NoiseModel>;
    type ControlledEngine = Box<dyn QuantumEngine>;
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn controller(&self) -> &Self::Controller {
        &self.noise_model
    }

    fn controller_mut(&mut self) -> &mut Self::Controller {
        &mut self.noise_model
    }

    fn engine(&self) -> &Self::ControlledEngine {
        &self.quantum_engine
    }

    fn engine_mut(&mut self) -> &mut Self::ControlledEngine {
        &mut self.quantum_engine
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
    // Create a state vector simulator with the specified number of qubits
    let state_vec = StateVec::new(num_qubits);

    // Create a quantum engine using the state vector simulator
    let quantum_engine = new_quantum_engine_arbitrary_qgate(state_vec);

    // Create a QuantumSystem with depolarizing noise
    QuantumSystem::new_with_depolarizing_noise(quantum_engine, probability)
}
