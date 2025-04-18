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

use rand::{RngCore, SeedableRng};

// Define a temporary Error module to match the rand crate's API
// This will be removed once we update to a newer version of rand
mod error {
    use core::fmt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Error;

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("CyclicRng error")
        }
    }

    impl std::error::Error for Error {}
}

/// Seed for the cyclic RNG
///
/// This is a simple container for the seed value and cycle length.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct CyclicSeed {
    /// The random seed value
    pub seed: u64,
    /// The length of the cycle
    pub cycle_length: usize,
}

impl Default for CyclicSeed {
    fn default() -> Self {
        Self {
            seed: 0,
            cycle_length: 1024,
        }
    }
}

/// A cyclic random number generator that repeats after a configurable number of iterations
///
/// This RNG is useful for testing scenarios where you want deterministic behavior
/// that repeats after a certain number of iterations.
///
/// Not suitable for production use - only for specific testing scenarios.
#[derive(Debug, Clone)]
pub struct CyclicRng {
    /// The current state of the RNG
    state: u64,
    /// The length of the cycle
    cycle_length: usize,
    /// The current position in the cycle
    cycle_pos: usize,
}

impl CyclicRng {
    /// Create a new cyclic RNG with the given seed and cycle length
    #[must_use]
    pub fn new(seed: u64, cycle_length: usize) -> Self {
        Self {
            state: seed,
            cycle_length,
            cycle_pos: 0,
        }
    }

    /// Create a new cyclic RNG with the given seed
    #[must_use]
    pub fn from_seed_struct(seed: &CyclicSeed) -> Self {
        Self::new(seed.seed, seed.cycle_length)
    }
}

impl RngCore for CyclicRng {
    fn next_u32(&mut self) -> u32 {
        // Generate a value based on the current state
        #[allow(clippy::cast_possible_truncation)]
        let result =
            ((self.state.wrapping_add(self.cycle_pos as u64)) % u64::from(u32::MAX)) as u32;

        // Update position in the cycle
        self.cycle_pos = (self.cycle_pos + 1) % self.cycle_length;

        result
    }

    fn next_u64(&mut self) -> u64 {
        // Generate a value based on the current state
        let result = self.state.wrapping_add(self.cycle_pos as u64);

        // Update position in the cycle
        self.cycle_pos = (self.cycle_pos + 1) % self.cycle_length;

        result
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // Use the method from rand crate to fill bytes using next_u32
        let mut i = 0;
        while i < dest.len() {
            let random_val = self.next_u32();
            let bytes = random_val.to_le_bytes();
            let remaining = dest.len() - i;
            let len = std::cmp::min(bytes.len(), remaining);
            dest[i..i + len].copy_from_slice(&bytes[..len]);
            i += len;
        }
    }
}

impl SeedableRng for CyclicRng {
    type Seed = [u8; 8];

    fn from_seed(seed: Self::Seed) -> Self {
        let seed_val = u64::from_le_bytes(seed);
        Self::new(seed_val, 1024)
    }

    fn seed_from_u64(seed: u64) -> Self {
        Self::new(seed, 1024)
    }
}

// The SimRng implementation has been removed.
// CyclicRng now directly uses RngCore + SeedableRng traits from the rand crate.
