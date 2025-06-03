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

//! Bitwise operations for `BitVec` values

use bitvec::prelude::*;

/// Left shift a `BitVec` (maintains same length, fills with zeros on right)
///
/// # Arguments
/// * `bv` - Value to shift
/// * `amount` - Number of bit positions to shift left
///
/// # Returns
/// A new `BitVec` containing `bv << amount`
#[must_use]
pub fn shift_left(bv: &BitVec<u8, Lsb0>, amount: usize) -> BitVec<u8, Lsb0> {
    if amount >= bv.len() {
        return BitVec::repeat(false, bv.len());
    }

    let mut result = BitVec::with_capacity(bv.len());
    // Fill with zeros for the shifted-in bits
    result.resize(amount, false);
    // Append the bits that weren't shifted out
    result.extend_from_bitslice(&bv[..bv.len() - amount]);
    result
}

/// Right shift a `BitVec` (maintains same length, fills with zeros on left)
///
/// # Arguments
/// * `bv` - Value to shift
/// * `amount` - Number of bit positions to shift right
///
/// # Returns
/// A new `BitVec` containing `bv >> amount`
#[must_use]
pub fn shift_right(bv: &BitVec<u8, Lsb0>, amount: usize) -> BitVec<u8, Lsb0> {
    if amount >= bv.len() {
        return BitVec::repeat(false, bv.len());
    }

    let mut result = BitVec::with_capacity(bv.len());
    // In LSB0, right shift moves bits towards lower indices
    // Take all bits except the last 'amount' ones
    result.extend_from_bitslice(&bv[amount..]);
    // Fill the rest with zeros
    result.resize(bv.len(), false);
    result
}

/// Left shift a `BitVec` with extension (used for parsing - grows the `BitVec` as needed)
///
/// # Arguments
/// * `bv` - Value to shift
/// * `amount` - Number of bit positions to shift left
///
/// # Returns
/// A new `BitVec` containing `bv << amount` with extended length
#[must_use]
pub fn shift_left_extend(bv: &BitVec<u8, Lsb0>, amount: usize) -> BitVec<u8, Lsb0> {
    if amount == 0 || bv.is_empty() {
        return bv.clone();
    }

    let mut result = BitVec::with_capacity(bv.len() + amount);
    result.resize(amount, false); // Add zeros at the beginning
    result.extend_from_bitslice(bv); // Append the original bits
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shift_left() {
        let mut bv = BitVec::<u8, Lsb0>::new();
        bv.extend([true, false, true, false]); // 5 in 4-bit (LSB first)

        let result = shift_left(&bv, 1);
        // Left shift by 1: [false, true, false, true] = 10 in 4-bit
        assert_eq!(result.len(), 4);
        assert!(!result[0]); // LSB
        assert!(result[1]);
        assert!(!result[2]);
        assert!(result[3]); // MSB
    }

    #[test]
    fn test_shift_right() {
        let mut bv = BitVec::<u8, Lsb0>::new();
        bv.extend([false, true, false, true]); // 10 in 4-bit (LSB first)

        let result = shift_right(&bv, 1);
        // Right shift by 1: [true, false, true, false] = 5 in 4-bit
        assert_eq!(result.len(), 4);
        assert!(result[0]); // LSB
        assert!(!result[1]);
        assert!(result[2]);
        assert!(!result[3]); // MSB
    }

    #[test]
    fn test_shift_left_extend() {
        let mut bv = BitVec::<u8, Lsb0>::new();
        bv.extend([true, false, true]); // 5 in binary (LSB first)

        let result = shift_left_extend(&bv, 2);
        // Left shift by 2 with extension: [false, false, true, false, true] = 20
        assert_eq!(result.len(), 5);
        assert!(!result[0]); // LSB
        assert!(!result[1]);
        assert!(result[2]);
        assert!(!result[3]);
        assert!(result[4]); // MSB
    }
}
