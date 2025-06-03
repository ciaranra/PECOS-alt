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

//! Comparison operations for `BitVec` values

use bitvec::prelude::*;
use std::cmp::Ordering;

/// Compare two `BitVecs` as signed integers (two's complement)
///
/// # Arguments
/// * `a` - First value to compare
/// * `b` - Second value to compare
///
/// # Returns
/// `Ordering::Less` if `a < b`, `Ordering::Greater` if `a > b`, `Ordering::Equal` if `a == b`
#[must_use]
pub fn compare(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> Ordering {
    // Check if either is empty
    if a.is_empty() || b.is_empty() {
        return a.len().cmp(&b.len());
    }

    // Get sign bits (MSB)
    let a_sign = a.last().as_deref().copied().unwrap_or(false);
    let b_sign = b.last().as_deref().copied().unwrap_or(false);

    match (a_sign, b_sign) {
        (true, false) => Ordering::Less,    // a is negative, b is positive
        (false, true) => Ordering::Greater, // a is positive, b is negative
        _ => {
            // Both have same sign, compare magnitude
            // For positive numbers: larger magnitude = larger number
            // For negative numbers: larger magnitude = smaller number (more negative)
            for i in (0..a.len()).rev() {
                let a_bit = a.get(i).as_deref().copied().unwrap_or(false);
                let b_bit = b.get(i).as_deref().copied().unwrap_or(false);

                match (a_bit, b_bit) {
                    (true, false) => return Ordering::Greater,
                    (false, true) => return Ordering::Less,
                    _ => {}
                }
            }
            Ordering::Equal
        }
    }
}

/// Compare two `BitVecs` as unsigned integers
///
/// # Arguments
/// * `a` - First value to compare
/// * `b` - Second value to compare
///
/// # Returns
/// `Ordering::Less` if `a < b`, `Ordering::Greater` if `a > b`, `Ordering::Equal` if `a == b`
#[must_use]
pub fn compare_unsigned(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> Ordering {
    // Compare from MSB to LSB
    for i in (0..a.len().max(b.len())).rev() {
        let a_bit = a.get(i).as_deref().copied().unwrap_or(false);
        let b_bit = b.get(i).as_deref().copied().unwrap_or(false);

        match (a_bit, b_bit) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            _ => {}
        }
    }

    Ordering::Equal
}
