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

use crate::sims_rngs::choices::Choices;
use core::fmt::Debug;
use rand::distr::Bernoulli;
use rand::prelude::*;

// TODO: Update this to use the new standard rng interface

/// Represents the minimal interface needed for simulations.
/// This trait also allows the blanket implementation provided by Rng to be overridden in favor of
/// potentially more efficient implementations.
pub trait SimRng: RngCore + SeedableRng + Debug {
    /// Generate a single bool where true has a probability of `p`.
    #[inline]
    fn gen_bool(&mut self, p: f64) -> bool {
        Bernoulli::new(p)
            .expect("Failed to create Bernoulli distribution due to invalid probability")
            .sample(self)
    }

    /// Generates a vector of bools, where true has an independent probability of `p`.
    #[inline]
    fn gen_bools(&mut self, p: f64, n: usize) -> Vec<bool> {
        let bernoulli = Bernoulli::new(p)
            .expect("Failed to create Bernoulli distribution due to invalid probability");
        (0..n).map(|_| bernoulli.sample(self)).collect()
    }

    /// Gives true and false each with probability of 50%
    #[inline]
    #[allow(clippy::cast_possible_wrap, clippy::as_conversions)]
    fn coin_flip(&mut self) -> bool {
        (self.next_u32() as i32) < 0
    }

    /// Choose between options given a weighted probabilities.
    #[inline]
    fn choose_weighted<'a, T>(&mut self, choices: &'a Choices<T>) -> &'a T {
        choices.sample(self)
    }

    #[inline]
    #[must_use]
    fn from_entropy() -> Self {
        Self::try_from_os_rng().expect("Failed to create RNG from OS entropy")
    }

    /// Create a new RNG instance from a 64-bit seed value
    ///
    /// This is a convenience method that wraps the `seed_from_u64` method from the `SeedableRng` trait.
    /// It allows for deterministic random number generation by using a specific seed.
    ///
    /// # Arguments
    /// * `seed` - A 64-bit seed value
    ///
    /// # Returns
    /// A new instance of the RNG initialized with the given seed
    ///
    /// # Examples
    /// ```rust
    /// use pecos_core::SimRng;
    /// use rand_chacha::ChaCha8Rng;
    ///
    /// let rng = ChaCha8Rng::from_seed(42);
    /// ```
    #[inline]
    #[must_use]
    fn from_seed(seed: u64) -> Self {
        Self::seed_from_u64(seed)
    }
}
