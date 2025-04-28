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

//! Quantum noise models for realistic quantum computation simulation
//!
//! This module provides various noise models that can be used to simulate
//! realistic quantum computation with errors. Each noise model implements
//! the `NoiseModel` trait and can be used with the quantum engines.

pub mod biased_depolarizing;
pub mod biased_measurement;
pub mod depolarizing;
pub mod general;
pub mod pass_through;
pub mod sampler;
pub mod type_cache_samplers;
pub mod utils;

pub use self::biased_depolarizing::BiasedDepolarizingNoiseModel;
pub use self::biased_measurement::BiasedMeasurementNoiseModel;
pub use self::depolarizing::DepolarizingNoiseModel;
pub use self::general::GeneralNoiseModel;
pub use self::pass_through::PassThroughNoiseModel;
pub use self::utils::{NoiseRng, NoiseUtils, ProbabilityValidator};
pub use sampler::{CachedSampler, PrecisionLevel, Sampler, SamplingMethod};

// Re-export the generic sampler types
pub use type_cache_samplers::{
    TypeCachedTableSampler8Bit, TypeCachedTableSampler16Bit, TypeCachedTableSampler32Bit,
    TypeCachedTableSampler64Bit, TypeCachedTwoQubitTableSampler8Bit,
    TypeCachedTwoQubitTableSampler16Bit, TypeCachedTwoQubitTableSampler32Bit,
    TypeCachedTwoQubitTableSampler64Bit,
};

use crate::byte_message::ByteMessage;
use crate::engines::{ControlEngine, EngineStage};
use crate::errors::QueueError;
use dyn_clone::DynClone;
use pecos_core::RngManageable;
use rand_chacha::ChaCha8Rng;
use std::any::Any;

/// Trait defining interface for quantum noise models
///
/// Noise models are a special kind of controller that transform
/// quantum operations before they're executed and potentially transform measurement
/// results after they're produced.
pub trait NoiseModel:
    ControlEngine<
        Input = ByteMessage,
        Output = ByteMessage,
        EngineInput = ByteMessage,
        EngineOutput = ByteMessage,
    > + DynClone
    + Send
    + Sync
    + Any
    + RngManageable<Rng = ChaCha8Rng>
{
    /// Returns a reference to self as Any
    ///
    /// This allows for type-checking and downcasting without requiring
    /// experimental trait upcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns a mutable reference to self as Any
    ///
    /// This allows for type-checking and downcasting without requiring
    /// experimental trait upcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Register the NoiseModel trait with dyn_clone
dyn_clone::clone_trait_object!(NoiseModel);

/// Base implementation for noise models
///
/// This struct provides common functionality for all noise models,
/// reducing code duplication and improving maintainability.
pub struct BaseNoiseModel {
    /// The random number generator for the noise model
    rng: NoiseRng,
}

impl BaseNoiseModel {
    /// Create a new `BaseNoiseModel` with a random seed
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: NoiseRng::new(),
        }
    }

    /// Create a new `BaseNoiseModel` with a specific seed
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: NoiseRng::with_seed(seed),
        }
    }

    /// Get a reference to the random number generator
    #[must_use]
    pub fn rng(&self) -> &NoiseRng {
        &self.rng
    }

    /// Check if a message contains measurement results
    ///
    /// # Arguments
    /// * `message` - The `ByteMessage` to check
    ///
    /// # Returns
    /// true if the message contains measurement results, false otherwise
    #[must_use]
    pub fn has_measurements(&self, message: &ByteMessage) -> bool {
        NoiseUtils::has_measurements(message)
    }
}

impl Default for BaseNoiseModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for BaseNoiseModel {
    fn clone(&self) -> Self {
        Self {
            rng: self.rng.clone(),
        }
    }
}

impl RngManageable for BaseNoiseModel {
    type Rng = ChaCha8Rng;

    fn set_rng(&mut self, rng: ChaCha8Rng) -> Result<(), Box<dyn std::error::Error>> {
        self.rng.set_rng(rng)
    }

    fn rng(&self) -> &Self::Rng {
        self.rng.rng()
    }

    fn rng_mut(&mut self) -> &mut Self::Rng {
        self.rng.rng_mut()
    }
}

impl ControlEngine for Box<dyn NoiseModel> {
    type Input = ByteMessage;
    type Output = ByteMessage;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        input: Self::Input,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        (**self).start(input)
    }

    fn continue_processing(
        &mut self,
        result: Self::EngineOutput,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, QueueError> {
        (**self).continue_processing(result)
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        (**self).reset()
    }
}

// Add tests for the BaseNoiseModel
#[cfg(test)]
mod base_tests {
    use super::*;
    use crate::byte_message::ByteMessageBuilder;

    #[test]
    fn test_base_noise_model_construction() {
        // Create a noise model with default seed
        let model = BaseNoiseModel::new();
        assert!(model.rng().random_float() >= 0.0);

        // Create a noise model with specific seed
        let model = BaseNoiseModel::with_seed(42);
        assert!(model.rng().random_float() >= 0.0);
    }

    #[test]
    fn test_base_noise_model_has_measurements() {
        let model = BaseNoiseModel::new();

        // Create a message with measurements
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_measurement_results();
        builder.add_measurement_results(&[0], &[0]);
        let message = builder.build();
        assert!(model.has_measurements(&message));

        // Create a message without measurements
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let message = builder.build();
        assert!(!model.has_measurements(&message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_message::ByteMessageBuilder;
    use crate::engines::noise::biased_measurement::BiasedMeasurementNoiseModel;

    #[test]
    fn test_noise_model_biased_measurement() {
        // Create a biased measurement noise model
        let mut noise_model = BiasedMeasurementNoiseModel::new(0.1, 0.2);

        // Create a quantum operation message
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let quantum_message = builder.build();

        // Create a measurement result message
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_measurement_results();
        builder.add_measurement_results(&[0], &[0]);
        let measurement_message = builder.build();

        // Operation should pass through unchanged
        let operation_result = noise_model.start(quantum_message.clone()).unwrap();
        if let EngineStage::NeedsProcessing(output) = operation_result {
            assert_eq!(
                output.as_bytes(),
                quantum_message.as_bytes(),
                "Quantum operations should pass through biased measurement noise unchanged"
            );
        } else {
            panic!("Expected NeedsProcessing stage");
        }

        // Measurements should be potentially modified
        let measurement_result = noise_model
            .continue_processing(measurement_message.clone())
            .unwrap();
        if let EngineStage::Complete(output) = measurement_result {
            // We can't check for equality because the noise is random,
            // but we can at least verify the output is a valid measurement result
            let measurements = output.parse_measurements().unwrap();
            assert!(
                !measurements.is_empty(),
                "Output should contain at least one measurement"
            );
        } else {
            panic!("Expected Complete stage");
        }
    }

    #[test]
    fn test_noise_model_depolarizing() {
        // Create a depolarizing noise model
        let mut noise_model = DepolarizingNoiseModel::new_uniform(0.1);

        // Create a quantum operation message
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_x(&[0]);
        let quantum_message = builder.build();

        // Create a measurement result message
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_measurement_results();
        builder.add_measurement_results(&[0], &[0]);
        let measurement_message = builder.build();

        // Operations should be modified
        let operation_result = noise_model.start(quantum_message.clone()).unwrap();
        if let EngineStage::NeedsProcessing(output) = operation_result {
            // Can't check for exact output due to randomness
            let gates = output.parse_quantum_operations().unwrap();
            assert!(!gates.is_empty(), "Output should contain at least one gate");
        } else {
            panic!("Expected NeedsProcessing stage");
        }

        // Measurements should pass through unchanged
        let measurement_result = noise_model
            .continue_processing(measurement_message.clone())
            .unwrap();
        if let EngineStage::Complete(output) = measurement_result {
            assert_eq!(
                output.as_bytes(),
                measurement_message.as_bytes(),
                "Measurement results should pass through depolarizing noise unchanged"
            );
        } else {
            panic!("Expected Complete stage");
        }
    }
}
