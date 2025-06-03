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

//! Helper functions for formatting QASM simulation results.

use pecos_engines::shot_results::{Data, ShotVec};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Format shot results as binary strings for each register
///
/// This function formats the results where each register is represented as an array
/// of binary strings, one per shot. For example:
/// ```json
/// {
///   "c": ["00", "11", "00", "11"],
///   "d": ["0", "1", "0", "1"]
/// }
/// ```
///
/// # Parameters
///
/// * `results` - The shot results to format
/// * `register_sizes` - A map of register names to their bit widths
///
/// # Returns
///
/// A JSON value containing the formatted results
///
/// # Examples
///
/// ```
/// use pecos_qasm::result_formatter::format_as_binary_strings;
/// use pecos_engines::shot_results::{Data, Shot, ShotVec};
/// use std::collections::BTreeMap;
///
/// // Create some example results
/// let mut shot1 = Shot::default();
/// shot1.data.insert("c".to_string(), Data::U32(0));  // "00"
/// shot1.data.insert("d".to_string(), Data::U32(1));  // "001"
///
/// let mut shot2 = Shot::default();
/// shot2.data.insert("c".to_string(), Data::U32(3));  // "11"
/// shot2.data.insert("d".to_string(), Data::U32(5));  // "101"
///
/// let results = ShotVec { shots: vec![shot1, shot2] };
///
/// // Define register sizes
/// let mut register_sizes = BTreeMap::new();
/// register_sizes.insert("c".to_string(), 2);  // 2-bit register
/// register_sizes.insert("d".to_string(), 3);  // 3-bit register
///
/// let formatted = format_as_binary_strings(&results, &register_sizes);
/// // Result: {"c": ["00", "11"], "d": ["001", "101"]}
/// ```
#[must_use]
pub fn format_as_binary_strings(
    results: &ShotVec,
    register_sizes: &BTreeMap<String, usize>,
) -> Value {
    if results.shots.is_empty() {
        return Value::Object(Map::new());
    }

    let mut result = Map::new();

    // For each register, collect binary strings from all shots
    for (reg_name, &bit_width) in register_sizes {
        let binary_strings: Vec<String> = results
            .shots
            .iter()
            .map(|shot| {
                shot.data
                    .get(reg_name)
                    .and_then(pecos_engines::prelude::Data::as_u32)
                    .map_or_else(
                        || "0".repeat(bit_width),
                        |value| {
                            if bit_width == 0 {
                                String::new()
                            } else {
                                format!("{value:0bit_width$b}")
                            }
                        },
                    )
            })
            .collect();

        result.insert(
            reg_name.clone(),
            Value::Array(binary_strings.into_iter().map(Value::String).collect()),
        );
    }

    Value::Object(result)
}

/// Format shot results as decimal values for each register
///
/// Similar to `format_as_binary_strings` but returns decimal values instead.
///
/// # Parameters
///
/// * `results` - The shot results to format
/// * `register_names` - Optional list of register names to include (None = all registers)
///
/// # Returns
///
/// A JSON value containing the formatted results
#[must_use]
pub fn format_as_decimal_arrays(results: &ShotVec, register_names: Option<&[&str]>) -> Value {
    if results.shots.is_empty() {
        return Value::Object(Map::new());
    }

    // Collect all unique register names if not specified
    let all_keys: Vec<String> = if let Some(names) = register_names {
        names.iter().map(|&s| s.to_string()).collect()
    } else {
        let mut keys = std::collections::BTreeSet::new();
        for shot in &results.shots {
            for key in shot.data.keys() {
                keys.insert(key.clone());
            }
        }
        keys.into_iter().collect()
    };

    let mut result = Map::new();

    // For each register, collect decimal values from all shots
    for reg_name in all_keys {
        let values: Vec<Value> = results
            .shots
            .iter()
            .map(|shot| {
                shot.data
                    .get(&reg_name)
                    .map_or(Value::Null, |data| match data {
                        Data::U8(v) => Value::Number((*v).into()),
                        Data::U16(v) => Value::Number((*v).into()),
                        Data::U32(v) => Value::Number((*v).into()),
                        Data::U64(v) => Value::Number((*v).into()),
                        Data::I8(v) => Value::Number((*v).into()),
                        Data::I16(v) => Value::Number((*v).into()),
                        Data::I32(v) => Value::Number((*v).into()),
                        Data::I64(v) => Value::Number((*v).into()),
                        _ => Value::Null,
                    })
            })
            .collect();

        result.insert(reg_name, Value::Array(values));
    }

    Value::Object(result)
}

/// Convenience extension trait for `QASMEngine` to format results
pub trait QASMResultFormatter {
    /// Get the results formatted as binary strings
    fn get_binary_string_results(&self, results: &ShotVec) -> Option<Value>;
}

impl QASMResultFormatter for crate::engine::QASMEngine {
    fn get_binary_string_results(&self, results: &ShotVec) -> Option<Value> {
        self.classical_register_sizes()
            .map(|sizes| format_as_binary_strings(results, sizes))
    }
}
