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
use rand::distr::Bernoulli;
use rand::prelude::*;

/// Choose between options given weighted probabilities.
#[inline]
pub fn choose_weighted<'a, T, R: Rng>(rng: &mut R, choices: &'a Choices<T>) -> &'a T {
    choices.sample(rng)
}

/// Gives true and false each with probability of 50%
#[inline]
#[allow(clippy::cast_possible_wrap, clippy::as_conversions)]
pub fn coin_flip<R: Rng>(rng: &mut R) -> bool {
    (rng.next_u32() as i32) < 0
}

/// Generates a vector of bools, where true has an independent probability of `p`.
///
/// # Panics
///
/// This function will panic if `p` is not a valid probability (not between 0.0 and 1.0, inclusive).
#[inline]
pub fn gen_bools<R: Rng>(rng: &mut R, p: f64, n: usize) -> Vec<bool> {
    let bernoulli = Bernoulli::new(p)
        .expect("Failed to create Bernoulli distribution due to invalid probability");
    (0..n).map(|_| bernoulli.sample(rng)).collect()
}
