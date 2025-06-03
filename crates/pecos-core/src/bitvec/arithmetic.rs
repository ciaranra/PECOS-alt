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

//! Arithmetic operations for `BitVec` values

use bitvec::prelude::*;
use std::cmp::Ordering;

use super::bitwise::shift_left;
use super::comparison::compare_unsigned;

/// Add two `BitVecs` (two's complement addition with wraparound)
///
/// # Arguments
/// * `a` - First operand
/// * `b` - Second operand
///
/// # Returns
/// A new `BitVec` containing `a + b` with the same length as `a`
#[must_use]
pub fn add(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> BitVec<u8, Lsb0> {
    let mut result = BitVec::with_capacity(a.len());
    let mut carry = false;

    for i in 0..a.len() {
        let a_bit = a.get(i).as_deref().copied().unwrap_or(false);
        let b_bit = b.get(i).as_deref().copied().unwrap_or(false);

        let sum = u8::from(a_bit) + u8::from(b_bit) + u8::from(carry);
        result.push((sum & 1) != 0);
        carry = sum > 1;
    }

    result
}

/// Subtract two `BitVecs` (two's complement subtraction with wraparound)
///
/// # Arguments
/// * `a` - Minuend (value to subtract from)
/// * `b` - Subtrahend (value to subtract)
///
/// # Returns
/// A new `BitVec` containing `a - b` with the same length as `a`
#[must_use]
pub fn subtract(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> BitVec<u8, Lsb0> {
    // a - b = a + (~b + 1) (two's complement)
    let mut b_inv = b.clone();
    b_inv = !b_inv;

    // Add 1 to inverted b
    let mut one = BitVec::with_capacity(b.len());
    one.push(true);
    one.resize(b.len(), false);
    let b_neg = add(&b_inv, &one);

    // Add a + (-b)
    add(a, &b_neg)
}

/// Multiply two `BitVecs` (signed multiplication)
///
/// # Arguments
/// * `a` - First factor
/// * `b` - Second factor
///
/// # Returns
/// A new `BitVec` containing `a * b` with the same length as `a`
#[must_use]
pub fn multiply(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> BitVec<u8, Lsb0> {
    // Check signs
    let a_negative = a.last().as_deref().copied().unwrap_or(false);
    let b_negative = b.last().as_deref().copied().unwrap_or(false);

    // Get absolute values
    let abs_a = if a_negative { negate(a) } else { a.clone() };
    let abs_b = if b_negative { negate(b) } else { b.clone() };

    // Perform unsigned multiplication on absolute values
    let mut result = BitVec::repeat(false, a.len());

    for (i, bit) in abs_b.iter().enumerate() {
        if *bit {
            // Shift a left by i positions
            let shifted = shift_left(&abs_a, i);
            // Add to result
            result = add(&result, &shifted);
        }
    }

    // Truncate to original width
    result.truncate(a.len());

    // Apply sign to result (negative if signs differ)
    if a_negative != b_negative {
        result = negate(&result);
    }

    result
}

/// Divide two `BitVecs` (signed division)
///
/// # Arguments
/// * `a` - Dividend (value to be divided)
/// * `b` - Divisor (value to divide by)
///
/// # Returns
/// A new `BitVec` containing `a / b` with the same length as `a`
/// Returns zero if `b` is zero (division by zero)
#[must_use]
pub fn divide(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> BitVec<u8, Lsb0> {
    // Check for division by zero
    if b.not_any() {
        return BitVec::repeat(false, a.len()); // Return 0 on division by zero
    }

    // Check signs
    let a_negative = a.last().as_deref().copied().unwrap_or(false);
    let b_negative = b.last().as_deref().copied().unwrap_or(false);

    // Get absolute values
    let abs_a = if a_negative { negate(a) } else { a.clone() };
    let abs_b = if b_negative { negate(b) } else { b.clone() };

    // Perform unsigned division on absolute values
    let mut quotient = BitVec::repeat(false, a.len());
    let mut remainder = abs_a;

    // Find highest set bit in divisor
    let divisor_bits = abs_b.len() - abs_b.trailing_zeros();
    if divisor_bits == 0 {
        return quotient; // b is zero
    }

    // Perform long division
    for i in (0..a.len()).rev() {
        if i + 1 >= divisor_bits {
            let shift_amount = i + 1 - divisor_bits;
            let shifted_b = shift_left(&abs_b, shift_amount);

            // Use unsigned comparison for absolute values
            if compare_unsigned(&remainder, &shifted_b) != Ordering::Less {
                remainder = subtract(&remainder, &shifted_b);
                quotient.set(shift_amount, true);
            }
        }
    }

    // Apply sign to result (negative if signs differ)
    if a_negative != b_negative {
        quotient = negate(&quotient);
    }

    quotient
}

/// Negate a `BitVec` (two's complement)
///
/// # Arguments
/// * `bv` - Value to negate
///
/// # Returns
/// A new `BitVec` containing `-bv`
#[must_use]
pub fn negate(bv: &BitVec<u8, Lsb0>) -> BitVec<u8, Lsb0> {
    let mut inv = bv.clone();
    inv = !inv;

    let mut one = BitVec::with_capacity(bv.len());
    one.push(true);
    one.resize(bv.len(), false);

    add(&inv, &one)
}

/// Add two `BitVecs` with extension (used for parsing - allows result to grow)
///
/// # Arguments
/// * `a` - First operand
/// * `b` - Second operand
///
/// # Returns
/// A new `BitVec` containing `a + b` with extended length if needed
pub(crate) fn add_extend(a: &BitVec<u8, Lsb0>, b: &BitVec<u8, Lsb0>) -> BitVec<u8, Lsb0> {
    let max_len = a.len().max(b.len());
    let mut result = BitVec::with_capacity(max_len + 1);
    let mut carry = false;

    for i in 0..max_len {
        let a_bit = a.get(i).as_deref().copied().unwrap_or(false);
        let b_bit = b.get(i).as_deref().copied().unwrap_or(false);

        let sum = u8::from(a_bit) + u8::from(b_bit) + u8::from(carry);
        result.push((sum & 1) != 0);
        carry = sum > 1;
    }

    if carry {
        result.push(true);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let mut a = BitVec::<u8, Lsb0>::new();
        a.extend([true, false, true]); // 5 in binary (LSB first)

        let mut b = BitVec::<u8, Lsb0>::new();
        b.extend([true, true, false]); // 3 in binary (LSB first)

        let result = add(&a, &b);

        // 5 + 3 = 8, but in 3-bit arithmetic with wraparound: 8 = [false, false, false] (LSB first)
        // This is because 8 = 1000 in 4-bit, but we only keep the lower 3 bits = 000
        assert_eq!(result.len(), 3);
        assert!(!result[0]); // LSB
        assert!(!result[1]);
        assert!(!result[2]); // MSB
    }

    #[test]
    fn test_subtract() {
        let mut a = BitVec::<u8, Lsb0>::new();
        a.extend([false, false, true]); // 4 in binary (LSB first)

        let mut b = BitVec::<u8, Lsb0>::new();
        b.extend([true, false, false]); // 1 in binary (LSB first)

        let result = subtract(&a, &b);
        // 4 - 1 = 3 = [true, true, false] (LSB first)
        assert_eq!(result.len(), 3);
        assert!(result[0]); // LSB
        assert!(result[1]);
        assert!(!result[2]); // MSB
    }
}
