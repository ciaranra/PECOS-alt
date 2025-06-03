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

//! Display and string conversion operations for `BitVec` values

use bitvec::prelude::*;

/// Convert a `BitVec` to a decimal string representation
///
/// # Arguments
/// * `bitvec` - The `BitVec` to convert
///
/// # Returns
/// A string representation of the decimal value
#[must_use]
pub fn to_decimal_string(bitvec: &BitVec<u8, Lsb0>) -> String {
    if bitvec.is_empty() {
        return "0".to_string();
    }

    // For now, we'll handle up to 128 bits efficiently
    // For larger values, we'd need a more sophisticated algorithm
    if bitvec.len() <= 128 {
        let mut value = 0u128;
        for (i, bit) in bitvec.iter().enumerate() {
            if i < 128 && *bit {
                value |= 1u128 << i;
            }
        }
        value.to_string()
    } else {
        // For very large numbers, show in hex with bit count
        format!(
            "0x{:x}...[{} bits]",
            bitvec
                .iter()
                .take(64)
                .fold(0u64, |acc, bit| (acc << 1) | u64::from(*bit)),
            bitvec.len()
        )
    }
}

/// Convert a `BitVec` to a binary string representation (MSB first, prefixed with "0b")
///
/// # Arguments
/// * `bitvec` - The `BitVec` to convert
///
/// # Returns
/// A string like "0b1010" with MSB first (conventional binary representation)
#[must_use]
pub fn to_binary_string(bitvec: &BitVec<u8, Lsb0>) -> String {
    if bitvec.is_empty() {
        return "0b0".to_string();
    }

    let mut result = String::with_capacity(bitvec.len() + 2);
    result.push_str("0b");

    // Reverse iteration to get MSB first
    for bit in bitvec.iter().rev() {
        result.push(if *bit { '1' } else { '0' });
    }

    result
}

/// Convert a `BitVec` to a hexadecimal string representation (prefixed with "0x")
///
/// # Arguments
/// * `bitvec` - The `BitVec` to convert
///
/// # Returns
/// A string like "0x1a2b" representing the hexadecimal value
#[must_use]
pub fn to_hex_string(bitvec: &BitVec<u8, Lsb0>) -> String {
    use std::fmt::Write;

    if bitvec.is_empty() {
        return "0x0".to_string();
    }

    // Group bits into nibbles (4 bits each) and convert to hex
    let mut result = String::from("0x");
    let mut nibbles = Vec::new();

    // Process bits in groups of 4, starting from LSB
    for chunk in bitvec.chunks(4) {
        let mut nibble_value = 0u8;
        for (i, bit) in chunk.iter().enumerate() {
            if *bit {
                nibble_value |= 1 << i;
            }
        }
        nibbles.push(nibble_value);
    }

    // Convert nibbles to hex, MSB first
    for &nibble in nibbles.iter().rev() {
        let _ = write!(result, "{nibble:x}");
    }

    result
}

/// Convert a `BitVec` to a bitstring representation (e.g., "1010")
///
/// # Arguments
/// * `bitvec` - The `BitVec` to convert
///
/// # Returns
/// A string of '0' and '1' characters representing the bits (LSB first)
#[must_use]
pub fn to_bitstring(bitvec: &BitVec<u8, Lsb0>) -> String {
    let mut result = String::with_capacity(bitvec.len());
    for bit in bitvec {
        result.push(if *bit { '1' } else { '0' });
    }
    result
}

/// Convert a `BitVec` to a boolean array representation (e.g., "[true, false, true]")
///
/// # Arguments
/// * `bitvec` - The `BitVec` to convert
///
/// # Returns
/// A string like "[true, false, true]" showing the boolean values (LSB first)
#[must_use]
pub fn to_bool_array(bitvec: &BitVec<u8, Lsb0>) -> String {
    if bitvec.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::from("[");
    for (i, bit) in bitvec.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(if *bit { "true" } else { "false" });
    }
    result.push(']');
    result
}

/// Create a `BitVec` from a bitstring (e.g., "1010")
///
/// # Arguments
/// * `bitstring` - String of '0' and '1' characters
///
/// # Returns
/// `Some(BitVec)` if parsing succeeds, `None` if invalid characters found
#[must_use]
pub fn from_bitstring(bitstring: &str) -> Option<BitVec<u8, Lsb0>> {
    let mut bv = BitVec::<u8, Lsb0>::with_capacity(bitstring.len());
    for ch in bitstring.chars() {
        match ch {
            '0' => bv.push(false),
            '1' => bv.push(true),
            _ => return None, // Invalid character
        }
    }
    Some(bv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_decimal_string() {
        let mut bv = BitVec::<u8, Lsb0>::new();
        bv.extend([true, false, true]); // 5 in binary (LSB first)

        assert_eq!(to_decimal_string(&bv), "5");
    }

    #[test]
    fn test_from_bitstring() {
        let bv = from_bitstring("101").unwrap(); // LSB first
        assert_eq!(bv.len(), 3);
        assert!(bv[0]); // LSB
        assert!(!bv[1]);
        assert!(bv[2]); // MSB
    }

    #[test]
    fn test_display_formats() {
        let mut bv = BitVec::<u8, Lsb0>::new();
        bv.extend([true, false, true]); // 5 in binary (LSB first)

        // Test binary string (MSB first)
        assert_eq!(to_binary_string(&bv), "0b101");

        // Test hex string
        assert_eq!(to_hex_string(&bv), "0x5");

        // Test bool array
        assert_eq!(to_bool_array(&bv), "[true, false, true]");

        // Test bitstring (LSB first)
        assert_eq!(to_bitstring(&bv), "101");
    }
}
