use super::{ByteMessage, NoiseModel, PassThroughNoise};
use crate::errors::QueueError;
use parking_lot::Mutex;
use pecos_core::types::{GateType, QuantumCommand};
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
    // Existing method implementations stay the same
    #[must_use]
    pub fn builder() -> DepolarizingNoiseBuilder {
        DepolarizingNoiseBuilder::new()
    }

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

    // Apply noise to commands (internal implementation)
    // Updated to work directly with Vec<QuantumCommand>
    fn apply_noise_to_commands(&self, commands: Vec<QuantumCommand>) -> Vec<QuantumCommand> {
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

        noisy_commands
    }
}

impl NoiseModel for DepolarizingNoise {
    fn apply_noise(&self, message: ByteMessage) -> Result<ByteMessage, QueueError> {
        // Parse commands from the message
        let commands = message.parse_quantum_operations()?;

        // Extract commands as Vec
        let commands_vec: Vec<QuantumCommand> = commands;

        // Apply noise to the commands
        let noisy_commands = self.apply_noise_to_commands(commands_vec);

        // REPLACE code like this:
        // ByteMessage::from_commands(noisy_commands)

        // WITH this builder pattern:
        Ok(ByteMessage::builder()
            .add_quantum_commands(&noisy_commands)
            .build())
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

// The rest of the code remains unchanged
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
