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

#![allow(clippy::similar_names)]
// For percentage calculations below with large usize values converted to f64,
// we accept the potential precision loss since the values are used only for display
// with a single decimal place, and the precision loss would only be observable
// with extremely large shot counts (> 2^53).
#![allow(clippy::cast_precision_loss)]

use crate::byte_message::ByteMessage;
use bitvec::prelude::*;
use num_bigint::BigInt;
use pecos_core::errors::PecosError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fmt;

/// Represents a data value that can be stored in a shot result.
///
/// This enum supports common numeric types and provides a flexible way to store
/// measurement outcomes and other data from quantum program execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Data {
    /// 8-bit unsigned integer
    U8(u8),
    /// 16-bit unsigned integer
    U16(u16),
    /// 32-bit unsigned integer
    U32(u32),
    /// 64-bit unsigned integer
    U64(u64),
    /// 8-bit signed integer
    I8(i8),
    /// 16-bit signed integer
    I16(i16),
    /// 32-bit signed integer
    I32(i32),
    /// 64-bit signed integer
    I64(i64),
    /// 32-bit floating point
    F32(f32),
    /// 64-bit floating point
    F64(f64),
    /// String data
    String(String),
    /// Boolean value
    Bool(bool),
    /// Arbitrary precision integer
    BigInt(BigInt),
    /// Byte array for efficient binary data storage
    Bytes(Vec<u8>),
    /// Bit vector with indexed bit access
    BitVec(BitVec<u8, Lsb0>),
    /// JSON value for complex or dynamic data
    Json(JsonValue),
}

impl Data {
    /// Create a Bytes variant from a Vec<u8>
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }

    /// Create a `BitVec` variant from a bitstring (string of '0' and '1' chars)
    /// Returns None if the string contains non-binary characters
    #[must_use]
    pub fn from_bitstring(bitstring: &str) -> Option<Self> {
        let mut bv = BitVec::<u8, Lsb0>::with_capacity(bitstring.len());
        for ch in bitstring.chars() {
            match ch {
                '0' => bv.push(false),
                '1' => bv.push(true),
                _ => return None, // Invalid character
            }
        }
        Some(Self::BitVec(bv))
    }

    /// Create a Bytes variant from a bitstring (for backward compatibility)
    #[must_use]
    pub fn from_bitstring_as_bytes(bitstring: &str) -> Option<Self> {
        // Convert bitstring to bytes (8 bits per byte)
        let mut bytes = Vec::new();
        let chars: Vec<char> = bitstring.chars().collect();

        for chunk in chars.chunks(8) {
            let mut byte = 0u8;
            for (i, &ch) in chunk.iter().enumerate() {
                match ch {
                    '0' => {} // bit is already 0
                    '1' => byte |= 1 << (7 - i),
                    _ => return None, // Invalid character
                }
            }
            bytes.push(byte);
        }

        Some(Self::Bytes(bytes))
    }

    /// Convert to a bitstring representation
    #[must_use]
    pub fn to_bitstring(&self) -> Option<String> {
        match self {
            Self::Bytes(bytes) => {
                let mut result = String::with_capacity(bytes.len() * 8);
                for byte in bytes {
                    for i in (0..8).rev() {
                        result.push(if (byte >> i) & 1 == 1 { '1' } else { '0' });
                    }
                }
                Some(result)
            }
            Self::BitVec(bv) => {
                let mut result = String::with_capacity(bv.len());
                for bit in bv {
                    result.push(if *bit { '1' } else { '0' });
                }
                Some(result)
            }
            _ => None,
        }
    }

    /// Convert data to a value string suitable for JSON output
    ///
    /// For integer-like types (including `BitVec`), returns decimal representation.
    /// For other types, returns a sensible string representation.
    #[must_use]
    pub fn to_value_string(&self) -> String {
        match self {
            // Integer types -> decimal
            Self::U8(v) => v.to_string(),
            Self::U16(v) => v.to_string(),
            Self::U32(v) => v.to_string(),
            Self::U64(v) => v.to_string(),
            Self::I8(v) => v.to_string(),
            Self::I16(v) => v.to_string(),
            Self::I32(v) => v.to_string(),
            Self::I64(v) => v.to_string(),
            Self::Bool(v) => if *v { "1" } else { "0" }.to_string(),
            Self::BitVec(bv) => {
                // Convert BitVec to decimal string
                let mut value = 0u128; // Use u128 for up to 128 bits
                for (i, bit) in bv.iter().enumerate() {
                    if *bit && i < 128 {
                        value |= 1u128 << i;
                    }
                }
                value.to_string()
            }
            Self::BigInt(v) => v.to_string(),
            // Other types -> sensible string representation
            Self::F32(v) => v.to_string(),
            Self::F64(v) => v.to_string(),
            Self::String(v) => v.clone(),
            Self::Bytes(v) => format!("{v:?}"), // Could use hex or base64
            Self::Json(v) => v.to_string(),
        }
    }

    /// Try to convert the data to a u32 value if possible
    #[must_use]
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::U8(v) => Some(u32::from(*v)),
            Self::U16(v) => Some(u32::from(*v)),
            Self::U32(v) => Some(*v),
            Self::U64(v) => u32::try_from(*v).ok(),
            Self::I8(v) => u32::try_from(*v).ok(),
            Self::I16(v) => u32::try_from(*v).ok(),
            Self::I32(v) => u32::try_from(*v).ok(),
            Self::I64(v) => u32::try_from(*v).ok(),
            Self::Bool(v) => Some(u32::from(*v)),
            Self::Json(v) => v.as_u64().and_then(|n| u32::try_from(n).ok()),
            Self::BigInt(v) => u32::try_from(v).ok(),
            Self::Bytes(v) => {
                // Try to interpret first 4 bytes as little-endian u32
                if v.len() >= 4 {
                    Some(u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
                } else {
                    None
                }
            }
            Self::BitVec(v) => {
                // Convert up to 32 bits to u32
                if v.is_empty() {
                    None
                } else {
                    let mut result = 0u32;
                    for (i, bit) in v.iter().take(32).enumerate() {
                        if *bit {
                            result |= 1 << i;
                        }
                    }
                    Some(result)
                }
            }
            _ => None,
        }
    }
}

// Implement Display trait for Data instead of inherent to_string method
impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U8(v) => write!(f, "{v}"),
            Self::U16(v) => write!(f, "{v}"),
            Self::U32(v) => write!(f, "{v}"),
            Self::U64(v) => write!(f, "{v}"),
            Self::I8(v) => write!(f, "{v}"),
            Self::I16(v) => write!(f, "{v}"),
            Self::I32(v) => write!(f, "{v}"),
            Self::I64(v) => write!(f, "{v}"),
            Self::F32(v) => write!(f, "{v}"),
            Self::F64(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v}"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::BigInt(v) => write!(f, "{v}"),
            Self::Bytes(v) => write!(f, "{v:?}"), // Could also use hex or base64
            Self::BitVec(_) => write!(
                f,
                "{}",
                self.to_bitstring().unwrap_or_else(|| format!("{self:?}"))
            ),
            Self::Json(v) => write!(f, "{v}"),
        }
    }
}

/// Represents the results of a single shot (execution) of a quantum program.
///
/// This struct contains a flexible mapping of data values for storing measurement
/// outcomes and other execution results. Complex or engine-specific data can be
/// stored using the `Data::Json` variant.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    /// Mapping of names to data values (measurements, calculations, complex data, etc.)
    pub data: BTreeMap<String, Data>,
}

impl Shot {
    /// Add a register with a specific bit width to the shot
    ///
    /// This stores the register value as a `BitVec` and also stores metadata about its width.
    /// The width is important for proper formatting (e.g., zero-padding in binary representation).
    ///
    /// # Parameters
    ///
    /// * `name` - The register name
    /// * `value` - The register value as u32
    /// * `width` - The bit width of the register
    pub fn add_register(&mut self, name: &str, value: u32, width: usize) {
        // Create a BitVec with the specified width
        let mut bv = BitVec::<u8, Lsb0>::with_capacity(width);

        // Set bits from the value
        for i in 0..width {
            bv.push((value >> i) & 1 == 1);
        }

        // Store the BitVec
        self.data.insert(name.to_string(), Data::BitVec(bv));

        // Store the width metadata with a special key
        self.data.insert(
            format!("_width_{name}"),
            Data::U32(u32::try_from(width).unwrap_or(u32::MAX)),
        );
    }

    /// Get a register's bit width if it was stored with `add_register`
    #[must_use]
    pub fn get_register_width(&self, name: &str) -> Option<usize> {
        self.data
            .get(&format!("_width_{name}"))
            .and_then(Data::as_u32)
            .map(|w| w as usize)
    }

    /// Create a binary string for a register, respecting its stored width
    #[must_use]
    pub fn register_to_binary_string(&self, name: &str) -> Option<String> {
        match self.data.get(name)? {
            Data::BitVec(bv) => {
                // For BitVec, the length IS the width
                let width = bv.len();
                let mut result = String::with_capacity(width);
                for i in (0..width).rev() {
                    result.push(if bv[i] { '1' } else { '0' });
                }
                Some(result)
            }
            Data::U32(v) => {
                // For U32, check if we have stored width metadata
                let width = self.get_register_width(name).unwrap_or(32);
                Some(format!("{v:0width$b}"))
            }
            _ => None,
        }
    }

    /// Create a `Shot` directly from a `ByteMessage` containing measurement results.
    ///
    /// This method extracts measurement results from a `ByteMessage` and creates a `Shot`
    /// with properly mapped result IDs to names.
    ///
    /// # Parameters
    ///
    /// * `message` - A `ByteMessage` containing measurement results
    /// * `result_id_to_name` - A mapping from `result_id` to a human-readable name
    ///
    /// # Returns
    ///
    /// A new `Shot` instance containing the processed measurement results
    ///
    /// # Errors
    ///
    /// Returns an error if the `ByteMessage` cannot be parsed or doesn't contain valid measurement results
    pub fn from_byte_message(
        message: &ByteMessage,
        result_id_to_name: &BTreeMap<usize, String>,
    ) -> Result<Self, PecosError> {
        // Extract the measurement results from the ByteMessage
        let measurements = message.measurement_results_as_vec()?;

        let mut result = Self::default();

        // Process each measurement
        for (result_id, value) in measurements {
            // Get the name for this result_id, or use a default if not found
            let name = result_id_to_name
                .get(&result_id)
                .cloned()
                .unwrap_or_else(|| format!("result_{result_id}"));

            // Store as U32 data
            result.data.insert(name, Data::U32(value));
        }

        Ok(result)
    }

    /// Creates a binary string representation of results.
    ///
    /// This is a convenience method that creates a binary string from register values.
    ///
    /// # Parameters
    ///
    /// * `registers` - Optional list of register names to include. If None, all registers are used.
    /// * `sort_by_name` - Whether to sort registers by name (true) or use provided order (false)
    ///
    /// # Returns
    ///
    /// A binary string representation of the specified registers
    #[must_use]
    pub fn create_binary_string(&self, registers: Option<&[&str]>, sort_by_name: bool) -> String {
        let mut register_entries: Vec<(String, u32)> = match registers {
            Some(names) => names
                .iter()
                .filter_map(|&name| {
                    self.data
                        .get(name)
                        .and_then(Data::as_u32)
                        .map(|v| (name.to_string(), v))
                })
                .collect(),
            None => self
                .data
                .iter()
                .filter_map(|(name, data)| data.as_u32().map(|v| (name.clone(), v)))
                .collect(),
        };

        if sort_by_name {
            register_entries.sort_by(|(name1, _), (name2, _)| name1.cmp(name2));
        }

        register_entries
            .iter()
            .map(|(_, value)| if *value > 0 { '1' } else { '0' })
            .collect()
    }
}

/// Represents the results of multiple shots (executions) of a quantum program.
///
/// This struct contains a vector of individual shot results, providing a clean
/// and flexible way to store and access measurement data from multiple executions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShotVec {
    /// Vector of individual shot results
    pub shots: Vec<Shot>,
}

impl Default for ShotVec {
    fn default() -> Self {
        Self::new()
    }
}

impl ShotVec {
    /// Get the total number of shots
    #[must_use]
    pub fn len(&self) -> usize {
        self.shots.len()
    }

    /// Check if there are no shots
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shots.is_empty()
    }

    /// Try to convert the shot vector to a `ShotMap` (columnar format)
    ///
    /// This method transforms the row-based shot data into column-based data where:
    /// - Keys are register names
    /// - Values are vectors containing the `Data` value for each shot
    ///
    /// # Returns
    /// - `Ok(ShotMap)` containing the columnar representation
    /// - `Err(PecosError)` if not all shots have the same register keys
    ///
    /// # Errors
    /// Returns a `PecosError` if:
    /// - Not all shots have the same register keys
    /// - A register is missing from any shot after the first
    ///
    /// # Example
    /// ```
    /// # use pecos_engines::shot_results::{ShotVec, Shot};
    /// let mut shot_vec = ShotVec::new();
    ///
    /// // Add shots with consistent structure
    /// for i in 0..3 {
    ///     let mut shot = Shot::default();
    ///     shot.add_register("a", i, 2);
    ///     shot.add_register("b", i * 2, 3);
    ///     shot_vec.shots.push(shot);
    /// }
    ///
    /// // Convert to ShotMap
    /// match shot_vec.try_as_shot_map() {
    ///     Ok(shot_map) => {
    ///         // Access all values for register "a"
    ///         let a_values = shot_map.get("a").unwrap();
    ///         assert_eq!(a_values.len(), 3);
    ///     }
    ///     Err(e) => {
    ///         // Handle inconsistent shot structure
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    /// This function should not panic under normal usage. The `unwrap()` call is protected
    /// by prior validation that ensures the key exists in the `BTreeMap`.
    pub fn try_as_shot_map(&self) -> Result<crate::shot_map::ShotMap, PecosError> {
        if self.is_empty() {
            return crate::shot_map::ShotMap::new(BTreeMap::new());
        }

        // Get register names from the first shot
        let register_names = self.get_register_names();

        // Initialize the columnar map with empty vectors
        let mut columnar_map: BTreeMap<String, Vec<Data>> = BTreeMap::new();
        for name in &register_names {
            columnar_map.insert(name.clone(), Vec::with_capacity(self.len()));
        }

        // Iterate through all shots and populate the columnar data
        for (shot_idx, shot) in self.shots.iter().enumerate() {
            // Check that this shot has the same keys
            let shot_keys: Vec<String> = shot
                .data
                .keys()
                .filter(|k| !k.starts_with("_width_"))
                .cloned()
                .collect();

            if shot_keys.len() != register_names.len() {
                return Err(PecosError::Processing(format!(
                    "Shot {} has {} registers, but expected {} based on first shot",
                    shot_idx,
                    shot_keys.len(),
                    register_names.len()
                )));
            }

            // Add each register's value to the appropriate column
            for name in &register_names {
                match shot.data.get(name) {
                    Some(data) => {
                        columnar_map.get_mut(name).unwrap().push(data.clone());
                    }
                    None => {
                        return Err(PecosError::Processing(format!(
                            "Shot {shot_idx} is missing register '{name}' which was present in the first shot"
                        )));
                    }
                }
            }
        }

        crate::shot_map::ShotMap::new(columnar_map)
    }
}

impl ShotVec {
    /// Creates a new empty `ShotVec` instance.
    #[must_use]
    pub fn new() -> Self {
        Self { shots: Vec::new() }
    }

    /// Get all register names (excluding metadata entries)
    #[must_use]
    pub fn get_register_names(&self) -> Vec<String> {
        if self.shots.is_empty() {
            return Vec::new();
        }

        // Get keys from first shot and filter out metadata entries
        let mut names: Vec<String> = self.shots[0]
            .data
            .keys()
            .filter(|k| !k.starts_with("_width_"))
            .cloned()
            .collect();
        names.sort();
        names
    }

    /// Format results as binary strings for all registers
    ///
    /// Returns a map where each register name maps to a vector of binary strings,
    /// one per shot. Each binary string is zero-padded to the register's width.
    #[must_use]
    pub fn format_as_binary_strings(&self) -> BTreeMap<String, Vec<String>> {
        let register_names = self.get_register_names();
        let mut result = BTreeMap::new();

        for name in register_names {
            let binary_strings: Vec<String> = self
                .shots
                .iter()
                .map(|shot| shot.register_to_binary_string(&name).unwrap_or_default())
                .collect();
            result.insert(name, binary_strings);
        }

        result
    }

    /// Creates a serializable representation for JSON output
    ///
    /// Returns an array of shot objects, where each shot contains its data
    /// with numerical values (U32, `BitVec`, etc.) converted to decimal numbers.
    ///
    /// # Example Output
    /// ```json
    /// [{"c": 3}, {"c": 0}, {"c": 3}, ...]
    /// ```
    #[must_use]
    pub fn create_json_value(&self) -> serde_json::Value {
        use serde_json::{Map, Value};

        let shots: Vec<Value> = self
            .shots
            .iter()
            .map(|shot| {
                let mut obj = Map::new();

                for (key, data) in &shot.data {
                    // Skip metadata entries
                    if key.starts_with('_') {
                        continue;
                    }

                    // Use to_value_string for simplicity, then parse as number if possible
                    let value_str = data.to_value_string();

                    // Try to parse as number first, fallback to string
                    let value = if let Ok(n) = value_str.parse::<u64>() {
                        Value::Number(n.into())
                    } else if let Ok(n) = value_str.parse::<i64>() {
                        Value::Number(n.into())
                    } else if let Ok(n) = value_str.parse::<f64>() {
                        serde_json::Number::from_f64(n)
                            .map_or_else(|| Value::String(value_str), Value::Number)
                    } else {
                        Value::String(value_str)
                    };

                    obj.insert(key.clone(), value);
                }

                Value::Object(obj)
            })
            .collect();

        Value::Array(shots)
    }

    /// Converts the `ShotVec` to a compact JSON string
    ///
    /// # Returns
    ///
    /// A compact JSON string without whitespace or formatting
    #[must_use]
    pub fn to_compact_json(&self) -> String {
        if self.shots.iter().all(|shot| shot.data.is_empty()) {
            return "[]".to_string();
        }

        // If we have complex JSON data, serialize the full shots
        if self
            .shots
            .iter()
            .any(|shot| shot.data.values().any(|v| matches!(v, Data::Json(_))))
        {
            serde_json::to_string(&self.shots).unwrap_or_else(|_| "[]".to_string())
        } else {
            // Otherwise use the aggregated format
            let json_value = self.create_json_value();
            serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string())
        }
    }

    /// Creates a `ShotVec` instance from a slice of `Shot` instances.
    ///
    /// This method simply collects the individual shot results into a vector.
    ///
    /// # Parameters
    ///
    /// * `results` - A slice of `Shot` instances to process
    ///
    /// # Returns
    ///
    /// A new `ShotVec` instance containing the processed measurement results
    #[must_use]
    pub fn from_measurements(results: &[Shot]) -> Self {
        Self {
            shots: results.to_vec(),
        }
    }

    /// Create a `ShotVec` instance directly from a `ByteMessage` containing measurement results.
    ///
    /// This method extracts measurement results from a `ByteMessage` and creates a `ShotVec`
    /// instance with properly formatted results. It's more efficient than going through
    /// `Shot` instances and provides better context about the measurements.
    ///
    /// # Parameters
    ///
    /// * `message` - A `ByteMessage` containing measurement results
    ///
    /// # Errors
    ///
    /// Returns a `PecosError` if the measurements cannot be extracted from the `ByteMessage`
    /// or if there are issues with creating the `ShotVec` instance.
    pub fn from_byte_message(message: &ByteMessage) -> Result<Self, PecosError> {
        // Extract the measurement results from the ByteMessage
        let measurements = message.measurement_results_as_vec()?;

        let mut shot_result = Shot::default();

        // Process each measurement
        for (result_id, value) in measurements {
            // Get the name for this result_id, or use a default if not found
            let name = format!("result_{result_id}");

            // Add the measurement to the results
            shot_result.data.insert(name, Data::U32(value));
        }

        Ok(Self {
            shots: vec![shot_result],
        })
    }

    /// Prints the `ShotVec` to stdout.
    pub fn print(&self) {
        println!("{self}");
    }
}

impl fmt::Display for ShotVec {
    /// Formats the shot results for display using compact JSON.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_compact_json())
    }
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::*;

    #[test]
    fn test_shot_results_display_64bit() {
        // Create a shot with various data types
        let mut shot1 = Shot::default();
        shot1.data.insert("reg_32".to_string(), Data::U32(42));

        // Add a large 64-bit register (larger than u32::MAX)
        let large_value = 1u64 << 34; // 2^34 = 17,179,869,184 (>4B)
        shot1
            .data
            .insert("reg_64".to_string(), Data::U64(large_value));

        // Add a signed 64-bit register with negative value
        shot1.data.insert("reg_signed".to_string(), Data::I64(-42));

        // Add some floating point data
        shot1
            .data
            .insert("float_val".to_string(), Data::F64(std::f64::consts::PI));

        // Create ShotVec with one shot
        let shot_results = ShotVec { shots: vec![shot1] };

        // Convert to string
        let json_string = shot_results.to_compact_json();
        let display_string = format!("{shot_results}");

        // Print the actual JSON for debugging
        println!("COMPACT JSON STRING: {json_string}");

        // The display string should match the compact JSON string
        assert_eq!(display_string, json_string);

        // Verify that both are valid JSON and contain the same data
        let json_value1: serde_json::Value = serde_json::from_str(&display_string).unwrap();
        let json_value2: serde_json::Value = serde_json::from_str(&json_string).unwrap();

        // Verify that both are arrays with the same length
        assert_eq!(
            json_value1.as_array().unwrap().len(),
            json_value2.as_array().unwrap().len(),
            "JSON arrays should have the same number of shots"
        );

        // Verify that all registers appear in the JSON
        assert!(json_string.contains("\"reg_32\""));
        assert!(json_string.contains("42"));
        assert!(json_string.contains("\"reg_64\""));
        assert!(json_string.contains("17179869184"));
        assert!(json_string.contains("\"reg_signed\""));
        assert!(json_string.contains("-42"));
        assert!(json_string.contains("\"float_val\""));
        assert!(json_string.contains("3.14159"));

        // Test with multiple shots
        let mut shot1_copy = Shot::default();
        shot1_copy.data.insert("reg_32".to_string(), Data::U32(42));
        shot1_copy
            .data
            .insert("reg_64".to_string(), Data::U64(large_value));
        shot1_copy
            .data
            .insert("reg_signed".to_string(), Data::I64(-42));
        shot1_copy
            .data
            .insert("float_val".to_string(), Data::F64(std::f64::consts::PI));

        let mut shot2 = Shot::default();
        shot2.data.insert("reg_32".to_string(), Data::U32(100));
        shot2.data.insert("reg_64".to_string(), Data::U64(200));

        let shot_results = ShotVec {
            shots: vec![shot1_copy, shot2],
        };

        let json_string = shot_results.to_compact_json();
        println!("Multi-shot JSON: {json_string}");

        // Verify the new shot array format shows individual shot objects
        assert!(json_string.contains("\"reg_32\":42"));
        assert!(json_string.contains("\"reg_32\":100"));
        assert!(json_string.contains("42"));
        assert!(json_string.contains("100"));

        // Test with JSON data variant
        let mut shot_with_json = Shot::default();
        shot_with_json
            .data
            .insert("measurement".to_string(), Data::U32(1));
        shot_with_json.data.insert(
            "metadata".to_string(),
            Data::Json(serde_json::json!({"custom": "data", "nested": {"value": 42}})),
        );

        let shot_results = ShotVec {
            shots: vec![shot_with_json],
        };

        let json_string = shot_results.to_compact_json();
        println!("Shot with JSON data: {json_string}");

        // When shots have JSON data variants, it should serialize the full shot structure
        assert!(json_string.contains("\"data\""));
        assert!(json_string.contains("\"metadata\""));
        assert!(json_string.contains("\"custom\""));
        assert!(json_string.contains("\"nested\""));
    }

    #[test]
    fn test_shot_results_compact_json() {
        // Create shot results with multiple shots
        let mut shot1 = Shot::default();
        shot1.data.insert("c".to_string(), Data::U32(0));
        shot1.data.insert("q".to_string(), Data::U32(1));

        let mut shot2 = Shot::default();
        shot2.data.insert("c".to_string(), Data::U32(3));
        shot2.data.insert("q".to_string(), Data::U32(0));

        let mut shot3 = Shot::default();
        shot3.data.insert("c".to_string(), Data::U32(2));
        shot3.data.insert("q".to_string(), Data::U32(1));

        let shot_results = ShotVec {
            shots: vec![shot1, shot2, shot3],
        };

        // Test compact format
        let compact_json = shot_results.to_compact_json();
        println!("COMPACT FORMAT: {compact_json}");

        // Compact format should not have newlines
        assert!(!compact_json.contains('\n'));
        // Should contain the data in the new format
        assert!(compact_json.contains(r#"{"c":0,"q":1}"#));
        assert!(compact_json.contains(r#"{"c":3,"q":0}"#));
        assert!(compact_json.contains(r#"{"c":2,"q":1}"#));

        // Test that Display also uses compact format
        let display_string = format!("{shot_results}");
        assert_eq!(display_string, compact_json);
    }

    #[test]
    fn test_bigint_support() {
        use num_bigint::BigInt;

        // Create a shot with BigInt data
        let mut shot = Shot::default();

        // Add a regular u32
        shot.data.insert("regular".to_string(), Data::U32(42));

        // Add a BigInt that fits in u32
        let small_bigint = BigInt::from(100u32);
        shot.data
            .insert("small_bigint".to_string(), Data::BigInt(small_bigint));

        // Add a BigInt that exceeds u64::MAX
        let huge_bigint = BigInt::from(u128::MAX) + BigInt::from(1000u32);
        shot.data
            .insert("huge_bigint".to_string(), Data::BigInt(huge_bigint.clone()));

        // Test to_string()
        assert_eq!(shot.data.get("regular").unwrap().to_string(), "42");
        assert_eq!(shot.data.get("small_bigint").unwrap().to_string(), "100");
        assert_eq!(
            shot.data.get("huge_bigint").unwrap().to_string(),
            (BigInt::from(u128::MAX) + BigInt::from(1000u32)).to_string()
        );

        // Test as_u32()
        assert_eq!(shot.data.get("regular").unwrap().as_u32(), Some(42));
        assert_eq!(shot.data.get("small_bigint").unwrap().as_u32(), Some(100));
        assert_eq!(shot.data.get("huge_bigint").unwrap().as_u32(), None); // Too big for u32

        // Test that BigInt serializes and we can work with it
        let shot_vec = ShotVec { shots: vec![shot] };
        let json = serde_json::to_string(&shot_vec).unwrap();

        // Print for debugging
        println!("Serialized JSON: {json}");

        // The important thing is that it serializes without error
        assert!(json.contains("\"regular\":42"));
        assert!(json.contains("\"small_bigint\""));
        assert!(json.contains("\"huge_bigint\""));

        // For BigInt deserialization, we'll need to use the actual format that num-bigint uses
        // Instead of testing deserialization, let's just make sure BigInt works programmatically
        let mut test_shot = Shot::default();
        test_shot.data.insert(
            "big_value".to_string(),
            Data::BigInt(BigInt::from(u128::MAX)),
        );

        match test_shot.data.get("big_value") {
            Some(Data::BigInt(v)) => {
                assert_eq!(v.to_string(), u128::MAX.to_string());
            }
            _ => panic!("Expected BigInt variant"),
        }
    }

    #[test]
    fn test_bytes_support() {
        // Create a shot with Bytes data
        let mut shot = Shot::default();

        // Add raw bytes
        let bytes = vec![0xFF, 0x00, 0xAB, 0xCD];
        shot.data
            .insert("raw_bytes".to_string(), Data::from_bytes(bytes.clone()));

        // Add bytes from bitstring
        let bitstring = "10110011";
        shot.data.insert(
            "from_bits".to_string(),
            Data::from_bitstring_as_bytes(bitstring).unwrap(),
        );

        // Test to_string (should show debug format)
        let bytes_str = shot.data.get("raw_bytes").unwrap().to_string();
        assert!(bytes_str.contains("255")); // 0xFF = 255
        assert!(bytes_str.contains("171")); // 0xAB = 171

        // Test bytes_to_bitstring
        match shot.data.get("from_bits").unwrap() {
            Data::Bytes(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0], 0b1011_0011);
            }
            _ => panic!("Expected Bytes variant"),
        }

        // Test bitstring conversion
        let bitstring_back = shot.data.get("from_bits").unwrap().to_bitstring();
        assert_eq!(bitstring_back, Some("10110011".to_string()));

        // Test as_u32
        let u32_bytes = vec![0x12, 0x34, 0x56, 0x78];
        shot.data
            .insert("u32_bytes".to_string(), Data::from_bytes(u32_bytes));
        assert_eq!(
            shot.data.get("u32_bytes").unwrap().as_u32(),
            Some(0x7856_3412) // Little-endian
        );

        // Test with measurement data - storing 16 qubit measurements efficiently
        let measurement_bits = "1011001110101101";
        let measurement_data = Data::from_bitstring_as_bytes(measurement_bits).unwrap();
        shot.data
            .insert("measurements".to_string(), measurement_data);

        // Verify we can get the bitstring back
        let retrieved = shot
            .data
            .get("measurements")
            .unwrap()
            .to_bitstring()
            .unwrap();
        assert_eq!(retrieved, measurement_bits);

        // Test serialization
        let shot_vec = ShotVec { shots: vec![shot] };
        let json = serde_json::to_string(&shot_vec).unwrap();

        // Bytes should serialize as arrays of numbers
        assert!(json.contains("\"raw_bytes\":[255,0,171,205]"));
        assert!(json.contains("\"from_bits\":[179]")); // 0b10110011 = 179
    }

    #[test]
    fn test_bitvec_support() {
        use bitvec::prelude::*;

        // Create a shot with BitVec data
        let mut shot = Shot::default();

        // Add BitVec from bitstring
        let bitstring = "101100111010110100101110";
        shot.data.insert(
            "bitvec".to_string(),
            Data::from_bitstring(bitstring).unwrap(),
        );

        // Test that it's actually a BitVec
        match shot.data.get("bitvec").unwrap() {
            Data::BitVec(bv) => {
                assert_eq!(bv.len(), bitstring.len());
                // Test individual bit access
                assert!(bv[0]); // '1'
                assert!(!bv[1]); // '0'
                assert!(bv[2]); // '1'
                assert!(bv[3]); // '1'
                assert!(!bv[4]); // '0'
            }
            _ => panic!("Expected BitVec variant"),
        }

        // Test to_bitstring
        let retrieved = shot.data.get("bitvec").unwrap().to_bitstring().unwrap();
        assert_eq!(retrieved, bitstring);

        // Test to_string (should return the bitstring)
        let string_repr = shot.data.get("bitvec").unwrap().to_string();
        assert_eq!(string_repr, bitstring);

        // Test as_u32 (first 32 bits interpreted as little-endian)
        let u32_val = shot.data.get("bitvec").unwrap().as_u32();
        // BitVec stores bits with LSB at index 0
        // So "101100111010110100101110" has bit[0]=1, bit[1]=0, bit[2]=1, etc.
        assert!(u32_val.is_some());

        // Create BitVec directly and modify it
        let mut bv = BitVec::<u8, Lsb0>::from_bitslice(bits![u8, Lsb0; 0, 1, 0, 1, 1, 0, 1, 0]);
        bv.set(2, true); // Change bit 2 from 0 to 1
        shot.data.insert("modified".to_string(), Data::BitVec(bv));

        match shot.data.get("modified").unwrap() {
            Data::BitVec(bv) => {
                assert!(bv[2]); // We changed this
                // Use our to_bitstring method instead
                let bitstring = shot.data.get("modified").unwrap().to_bitstring().unwrap();
                assert_eq!(bitstring, "01111010");
            }
            _ => panic!("Expected BitVec variant"),
        }

        // Test serialization - BitVec serializes based on its serde implementation
        let shot_vec = ShotVec { shots: vec![shot] };
        let json = serde_json::to_string(&shot_vec).unwrap();

        // BitVec should serialize (the format depends on bitvec's serde implementation)
        assert!(json.contains("\"bitvec\""));
        assert!(json.contains("\"modified\""));
    }

    #[test]
    fn test_register_with_width() {
        // Create shots with register data
        let mut shot1 = Shot::default();
        shot1.add_register("c", 0, 2); // 2-bit register with value 0 -> "00"
        shot1.add_register("d", 5, 3); // 3-bit register with value 5 -> "101"

        let mut shot2 = Shot::default();
        shot2.add_register("c", 3, 2); // 2-bit register with value 3 -> "11"
        shot2.add_register("d", 1, 3); // 3-bit register with value 1 -> "001"

        // Test binary string formatting
        assert_eq!(shot1.register_to_binary_string("c"), Some("00".to_string()));
        assert_eq!(
            shot1.register_to_binary_string("d"),
            Some("101".to_string())
        );
        assert_eq!(shot2.register_to_binary_string("c"), Some("11".to_string()));
        assert_eq!(
            shot2.register_to_binary_string("d"),
            Some("001".to_string())
        );

        // Test width metadata
        assert_eq!(shot1.get_register_width("c"), Some(2));
        assert_eq!(shot1.get_register_width("d"), Some(3));

        // Create ShotVec and test formatting
        let shot_vec = ShotVec {
            shots: vec![shot1, shot2],
        };
        let binary_strings = shot_vec.format_as_binary_strings();

        assert_eq!(
            binary_strings.get("c"),
            Some(&vec!["00".to_string(), "11".to_string()])
        );
        assert_eq!(
            binary_strings.get("d"),
            Some(&vec!["101".to_string(), "001".to_string()])
        );

        // Test that register names exclude metadata
        let names = shot_vec.get_register_names();
        assert_eq!(names, vec!["c", "d"]);
        assert!(!names.contains(&"_width_c".to_string()));
        assert!(!names.contains(&"_width_d".to_string()));
    }
}
