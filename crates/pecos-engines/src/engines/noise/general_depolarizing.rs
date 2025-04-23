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
use pecos_core::RngManageable;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct GeneralDepolarizingNoise {
    /// Probability of applying a random Pauli error
    p_prep: f64,
    p_meas: f64,
    p1: f64,
    p2: f64,
    /// Shared random number generator
    rng: Arc<Mutex<ChaCha8Rng>>,
}

impl GeneralDepolarizingNoise {
    #[must_use]
    pub fn new(p_prep: f64, p_meas: f64, p1: f64, p2: f64) -> Self {
        Self::new_with_options(p_prep, p_meas, p1, p2)
    }

    #[must_use]
    pub fn new_with_options(p_prep: f64, p_meas: f64, p1: f64, p2: f64) -> Self {
        let rng = ChaCha8Rng::from_os_rng();

        Self {
            p_prep,
            p_meas,
            p1,
            p2,
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
    pub fn set_probabilities(&mut self, p_prep: f64, p_meas: f64, p1: f64, p2: f64) {
        assert!(
            (0.0..=1.0).contains(&p1),
            "Probability must be between 0.0 and 1.0"
        );
        assert!(
            (0.0..=1.0).contains(&p2),
            "Probabiliity must be between 0.0 and 1.0"
        );

        self.p_prep = p_prep;
        self.p_meas = p_meas;
        self.p1 = p1;
        self.p2 = p2;
    }

    /// Get the current probability of applying a random Pauli error
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
                    self.apply_sq_faults(&mut builder, gate);
                }
                GateType::CX | GateType::RZZ | GateType::SZZ => {
                    self.apply_tq_faults(&mut builder, gate);
                }
                GateType::RZ => {
                    builder.add_quantum_gate(gate);
                }
                GateType::Measure => {
                    self.apply_meas_faults(&mut builder, gate);
                }
                GateType::Prep => {
                    self.apply_prep_faults(&mut builder, gate);
                }
            }
        }

        builder.build()
    }

    fn apply_prep_faults(&self, builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        builder.add_quantum_gate(gate);

        let mut rng = self.rng.lock().unwrap();

        if rng.random::<f64>() < self.p1 {
            builder.add_x(&gate.qubits);
        }
    }

    fn apply_meas_faults(&self, builder: &mut ByteMessageBuilder, gate: &QuantumGate) {
        let mut rng = self.rng.lock().unwrap();

        if rng.random::<f64>() < self.p1 {
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
                    builder.add_x(&gate.qubits);
                }
                1 => {
                    builder.add_y(&gate.qubits);
                }
                _ => {
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
                    builder.add_x(&[gate.qubits[1]]);
                }
                // IY
                1 => {
                    builder.add_y(&[gate.qubits[1]]);
                }
                // IZ
                2 => {
                    builder.add_z(&[gate.qubits[1]]);
                }
                // XI
                3 => {
                    builder.add_x(&[gate.qubits[0]]);
                }
                // XX
                4 => {
                    builder.add_x(&[gate.qubits[0]]);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // XY
                5 => {
                    builder.add_x(&[gate.qubits[0]]);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // XZ
                6 => {
                    builder.add_x(&[gate.qubits[0]]);
                    builder.add_z(&[gate.qubits[1]]);
                }
                // YI
                7 => {
                    builder.add_y(&[gate.qubits[0]]);
                }
                // YX
                8 => {
                    builder.add_y(&[gate.qubits[0]]);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // YY
                9 => {
                    builder.add_y(&[gate.qubits[0]]);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // YZ
                10 => {
                    builder.add_y(&[gate.qubits[0]]);
                    builder.add_z(&[gate.qubits[1]]);
                }
                // ZI
                11 => {
                    builder.add_z(&[gate.qubits[0]]);
                }
                // ZX
                12 => {
                    builder.add_z(&[gate.qubits[0]]);
                    builder.add_x(&[gate.qubits[1]]);
                }
                // ZY
                13 => {
                    builder.add_z(&[gate.qubits[0]]);
                    builder.add_y(&[gate.qubits[1]]);
                }
                // ZZ
                _ => {
                    builder.add_z(&[gate.qubits[0]]);
                    builder.add_z(&[gate.qubits[1]]);
                }
            }
        }
    }
}

impl NoiseModel for GeneralDepolarizingNoise {
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

    fn set_seed(&mut self, seed: u64) -> Result<(), QueueError> {
        // Use the RngManageable trait's set_rng method directly with a seeded RNG
        // to avoid infinite recursion with set_seed
        RngManageable::set_rng(self, ChaCha8Rng::seed_from_u64(seed))
            .map_err(|e| QueueError::OperationError(e.to_string()))
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
            "GeneralNoise stores its RNG behind an Arc<Mutex> and cannot return a direct reference"
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
            "GeneralNoise stores its RNG behind an Arc<Mutex> and cannot return a direct mutable reference"
        )
    }
}
