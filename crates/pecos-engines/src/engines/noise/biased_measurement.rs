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
use crate::engines::noise::{BaseNoiseModel, NoiseModel};
use crate::engines::{ControlEngine, EngineStage};
use crate::errors::QueueError;
use pecos_core::RngManageable;
use rand_chacha::ChaCha8Rng;
use std::any::Any;
use std::ops::RangeInclusive;

/// A noise model that biases qubit measurements
///
/// This noise model introduces bias to measurement results, changing
/// the probability of measuring 0 vs 1. It leaves quantum operations
/// unchanged.
#[derive(Clone)]
pub struct BiasedMeasurementNoise {
    /// The probability of flipping a 0 measurement to 1
    prob_flip_from_0: f64,
    /// The probability of flipping a 1 measurement to 0
    prob_flip_from_1: f64,
    /// Base noise model implementation
    base: BaseNoiseModel,
}

impl BiasedMeasurementNoise {
    /// Creates a new biased measurement noise model
    ///
    /// # Arguments
    /// * `prob_flip_from_0` - Probability of flipping a 0 measurement to 1
    /// * `prob_flip_from_1` - Probability of flipping a 1 measurement to 0
    ///
    /// # Panics
    /// Panics if either probability is not in the range [0, 1]
    #[must_use]
    pub fn new(prob_flip_from_0: f64, prob_flip_from_1: f64) -> Self {
        // Validate the probabilities
        Self::validate_probability(prob_flip_from_0, "prob_flip_from_0");
        Self::validate_probability(prob_flip_from_1, "prob_flip_from_1");

        Self {
            prob_flip_from_0,
            prob_flip_from_1,
            base: BaseNoiseModel::new(),
        }
    }

    /// Creates a new biased measurement noise model with a specific seed
    ///
    /// # Arguments
    /// * `prob_flip_from_0` - Probability of flipping a 0 measurement to 1
    /// * `prob_flip_from_1` - Probability of flipping a 1 measurement to 0
    /// * `seed` - Seed for the random number generator
    ///
    /// # Panics
    /// Panics if either probability is not in the range [0, 1]
    #[must_use]
    pub fn with_seed(prob_flip_from_0: f64, prob_flip_from_1: f64, seed: u64) -> Self {
        // Validate the probabilities
        Self::validate_probability(prob_flip_from_0, "prob_flip_from_0");
        Self::validate_probability(prob_flip_from_1, "prob_flip_from_1");

        Self {
            prob_flip_from_0,
            prob_flip_from_1,
            base: BaseNoiseModel::with_seed(seed),
        }
    }

    /// Validate that a probability is between 0.0 and 1.0
    ///
    /// # Arguments
    /// * `probability` - The probability value to validate
    /// * `name` - Name of the probability for error reporting
    ///
    /// # Panics
    /// Panics if the probability is not between 0.0 and 1.0
    fn validate_probability(probability: f64, name: &str) {
        let valid_range: RangeInclusive<f64> = 0.0..=1.0;
        assert!(
            valid_range.contains(&probability),
            "Probability {name} must be between 0.0 and 1.0, but was {probability}"
        );
    }

    /// Get the probability of flipping a 0 measurement to 1
    #[must_use]
    pub fn prob_flip_from_0(&self) -> f64 {
        self.prob_flip_from_0
    }

    /// Get the probability of flipping a 1 measurement to 0
    #[must_use]
    pub fn prob_flip_from_1(&self) -> f64 {
        self.prob_flip_from_1
    }

    /// Apply bias to a single measurement result
    ///
    /// # Arguments
    /// * `result_id` - The result ID of the measurement
    /// * `outcome` - The outcome of the measurement (0 or 1)
    ///
    /// # Returns
    /// The potentially biased measurement outcome
    fn apply_bias_to_measurement(&self, result_id: u32, outcome: u32) -> (u32, u32) {
        // Generate a random number to determine if we should flip
        let should_flip = if outcome == 0 {
            // Flip from 0 to 1 with probability prob_flip_from_0
            self.base.rng().random_float() < self.prob_flip_from_0
        } else {
            // Flip from 1 to 0 with probability prob_flip_from_1
            self.base.rng().random_float() < self.prob_flip_from_1
        };

        if should_flip {
            // Flip the measurement outcome
            (result_id, 1 - outcome)
        } else {
            // Keep the original measurement
            (result_id, outcome)
        }
    }

    /// Apply bias to a `ByteMessage` containing measurement results
    ///
    /// # Arguments
    /// * `message` - The `ByteMessage` containing measurement results
    ///
    /// # Returns
    /// A new `ByteMessage` with biased measurement results
    ///
    /// # Errors
    /// Returns a `QueueError` if applying bias fails
    fn apply_bias_to_message(&self, message: ByteMessage) -> Result<ByteMessage, QueueError> {
        // Parse the message to extract the measurement results
        let measurements = message.parse_measurements()?;

        // If the message doesn't contain measurements, return it unchanged
        if measurements.is_empty() {
            return Ok(message);
        }

        // Apply bias to each measurement
        let biased_measurements: Vec<(usize, u32)> = measurements
            .iter()
            .map(|(result_id, outcome)| {
                let (biased_result_id, biased_outcome) =
                    self.apply_bias_to_measurement(*result_id, *outcome);
                (biased_result_id as usize, biased_outcome)
            })
            .collect();

        // Create a new ByteMessage with the biased measurements
        Ok(ByteMessage::record_measurement_results(
            &biased_measurements,
        ))
    }
}

impl ControlEngine for BiasedMeasurementNoise {
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        input: Self::Input,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // Quantum operations pass through unchanged
        Ok(EngineStage::NeedsProcessing(input))
    }

    fn continue_processing(
        &mut self,
        result: Self::EngineOutput,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        // Apply bias to measurement results
        let biased_result = self.apply_bias_to_message(result)?;
        Ok(EngineStage::Complete(biased_result))
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        // Nothing to reset
        Ok(())
    }
}

impl NoiseModel for BiasedMeasurementNoise {
    fn set_seed(&mut self, seed: u64) -> Result<(), QueueError> {
        // Get a mutable reference to the NoiseRng to call set_seed
        let mut rng = self.base.rng().clone();
        rng.set_seed(seed)?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl RngManageable for BiasedMeasurementNoise {
    type Rng = ChaCha8Rng;

    fn set_rng(&mut self, rng: Self::Rng) -> Result<(), Box<dyn std::error::Error>> {
        self.base.set_rng(rng)
    }

    fn rng(&self) -> &Self::Rng {
        // Call BaseNoiseModel's RngManageable::rng() method
        <BaseNoiseModel as RngManageable>::rng(&self.base)
    }

    fn rng_mut(&mut self) -> &mut Self::Rng {
        // Call BaseNoiseModel's RngManageable::rng_mut() method
        <BaseNoiseModel as RngManageable>::rng_mut(&mut self.base)
    }
}
