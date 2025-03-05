use super::{ByteMessage, NoiseModel, PassThroughNoise};
use crate::errors::QueueError;
use parking_lot::Mutex;
use pecos_core::types::{CommandBatch, GateType, QuantumCommand};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

/// Simple depolarizing noise model that applies random Pauli errors
pub struct DepolarizingNoise {
    /// Probability of applying a noise operation after each gate
    probability: f64,
    /// Shared random number generator
    rng: Arc<Mutex<StdRng>>,
}

impl DepolarizingNoise {
    /// Creates a new instance of `DepolarizingNoise` with the specified noise probability.
    ///
    /// # Parameters
    /// - `probability`: The probability of applying a noise operation after each gate.
    ///   Must be a value between 0 and 1 (inclusive).
    ///
    /// # Panics
    /// - Panics if `probability` is not within the range [0.0, 1.0].
    #[must_use]
    pub fn builder() -> DepolarizingNoiseBuilder {
        DepolarizingNoiseBuilder::new()
    }

    /// Creates a new instance of `DepolarizingNoise` with the given noise probability and optional seed.
    ///
    /// # Parameters
    /// - `probability`: The probability of applying noise, between 0.0 and 1.0 (inclusive).
    /// - `seed`: An optional seed for the random number generator.
    ///
    /// # Panics
    /// - Panics if `probability` is not in the range [0.0, 1.0].
    #[must_use]
    pub fn new_with_options(probability: f64, seed: Option<u64>) -> Self {
        assert!(
            (0.0..=1.0).contains(&probability),
            "Probability must be between 0 and 1"
        );
        // Create RNG with seed if provided
        let rng = match seed {
            Some(s) => Arc::new(Mutex::new(StdRng::seed_from_u64(s))),
            None => Arc::new(Mutex::new(StdRng::from_os_rng())),
        };

        Self { probability, rng }
    }

    /// Helper to create sequence of gates for Pauli X
    fn x_gates(qubit: usize) -> Vec<QuantumCommand> {
        vec![QuantumCommand {
            gate: GateType::X {},
            qubits: vec![qubit],
        }]
    }

    /// Helper to create sequence of gates for Pauli Y
    fn y_gates(qubit: usize) -> Vec<QuantumCommand> {
        vec![QuantumCommand {
            gate: GateType::Y {},
            qubits: vec![qubit],
        }]
    }

    /// Helper to create Pauli Z gate
    fn z_gate(qubit: usize) -> QuantumCommand {
        QuantumCommand {
            gate: GateType::Z {},
            qubits: vec![qubit],
        }
    }

    // Apply noise to a command batch (internal implementation)
    fn apply_noise_to_batch(&self, commands: CommandBatch) -> CommandBatch {
        let mut noisy_commands = Vec::new();
        let mut rng = self.rng.lock();

        for cmd in commands {
            // Add the original command
            noisy_commands.push(cmd.clone());

            // For each qubit in the command, maybe apply noise
            for &qubit in &cmd.qubits {
                if rng.random::<f64>() < self.probability {
                    // Randomly choose X, Y, or Z error
                    match rng.random::<f64>() * 3.0 {
                        x if x < 1.0 => noisy_commands.extend(Self::x_gates(qubit)),
                        x if x < 2.0 => noisy_commands.extend(Self::y_gates(qubit)),
                        _ => noisy_commands.push(Self::z_gate(qubit)),
                    }
                }
            }
        }

        // Convert Vec back to CommandBatch
        noisy_commands.into()
    }
}

impl NoiseModel for DepolarizingNoise {
    fn apply_noise(&self, message: ByteMessage) -> Result<ByteMessage, QueueError> {
        // Parse commands from the message
        let commands = message.parse_quantum_operations()?;

        // Apply noise to the commands
        let noisy_commands = self.apply_noise_to_batch(commands);

        // Create a new message with the noisy commands
        ByteMessage::create_quantum_operations(&noisy_commands)
    }

    fn clone_box(&self) -> Box<dyn NoiseModel> {
        Box::new(DepolarizingNoise {
            probability: self.probability,
            rng: Arc::new(Mutex::new(StdRng::from_os_rng())),
        })
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        Ok(())
    }
}

pub struct DepolarizingNoiseBuilder {
    probability: f64,
    seed: Option<u64>,
}

impl Default for DepolarizingNoiseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DepolarizingNoiseBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            probability: 0.0,
            seed: None,
        }
    }

    #[must_use]
    pub fn with_probability(mut self, p: f64) -> Self {
        self.probability = p;
        self
    }

    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    #[must_use]
    pub fn build(self) -> Box<dyn NoiseModel> {
        let seed = self.seed;

        if self.probability == 0.0 {
            Box::new(PassThroughNoise)
        } else {
            Box::new(DepolarizingNoise::new_with_options(self.probability, seed))
        }
    }
}
