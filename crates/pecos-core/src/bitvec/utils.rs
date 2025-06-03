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

//! Utility operations for `BitVec` values

use bitvec::prelude::*;

/// Resize two `BitVecs` to the same width with appropriate sign extension
///
/// # Arguments
/// * `a` - First `BitVec` (will be modified in place)
/// * `b` - Second `BitVec` (will be modified in place)
/// * `default_width` - Minimum target width
///
/// # Note
/// Single-bit values are not sign-extended regardless of their value.
/// Multi-bit values are sign-extended based on their MSB.
pub fn resize_to_same_width(
    a: &mut BitVec<u8, Lsb0>,
    b: &mut BitVec<u8, Lsb0>,
    default_width: usize,
) {
    // Determine target width
    let target_width = a.len().max(b.len()).max(default_width);

    // Resize with sign extension only for negative numbers
    // For positive values and single-bit booleans, extend with zeros
    let a_sign = if a.len() > 1 {
        a.last().as_deref().copied().unwrap_or(false)
    } else {
        false // Don't sign-extend single bits
    };
    a.resize(target_width, a_sign);

    let b_sign = if b.len() > 1 {
        b.last().as_deref().copied().unwrap_or(false)
    } else {
        false // Don't sign-extend single bits
    };
    b.resize(target_width, b_sign);
}
