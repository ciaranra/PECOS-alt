// A wrapper struct for QASM-style formatting of ShotVec results

use pecos_engines::shot_results::{Data, Shot, ShotVec};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// A wrapper around `ShotVec` that provides QASM-style formatting methods.
///
/// This struct provides binary and decimal formatting for quantum measurement results,
/// optimized for classical register representations where data is stored as `U32` or `BitVec`.
///
/// # Example
/// ```no_run
/// # use pecos_engines::ShotVec;
/// # use pecos_qasm::QASMResults;
/// # let shot_vec = ShotVec::new();
/// let results = QASMResults::new(shot_vec);
/// println!("{}", results.to_compact_json());
/// ```
#[derive(Debug, Clone)]
pub struct QASMResults {
    shot_vec: ShotVec,
}

impl QASMResults {
    /// Create a new `QASMResults` wrapper around a `ShotVec`
    #[must_use]
    pub fn new(shot_vec: ShotVec) -> Self {
        Self { shot_vec }
    }

    /// Get a reference to the underlying `ShotVec`
    #[must_use]
    pub fn shot_vec(&self) -> &ShotVec {
        &self.shot_vec
    }

    /// Consume this wrapper and return the underlying `ShotVec`
    #[must_use]
    pub fn into_shot_vec(self) -> ShotVec {
        self.shot_vec
    }

    /// Get the number of shots
    #[must_use]
    pub fn len(&self) -> usize {
        self.shot_vec.len()
    }

    /// Check if there are no shots
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shot_vec.is_empty()
    }

    /// Get results as binary strings in JSON format
    ///
    /// Returns a JSON value with each register mapped to an array of binary strings.
    /// Each binary string is zero-padded to the register's bit width.
    ///
    /// # Example Output
    /// ```json
    /// {"c": ["00", "11", "00", ...], "d": ["000", "011", "000", ...]}
    /// ```
    pub fn to_binary_json(&self) -> Value {
        let binary_strings = self.shot_vec.format_as_binary_strings();
        let mut map = Map::new();

        for (name, strings) in binary_strings {
            let values: Vec<Value> = strings.into_iter().map(Value::String).collect();
            map.insert(name, Value::Array(values));
        }

        Value::Object(map)
    }

    /// Get results as compact JSON string (shot-based format)
    ///
    /// Returns a compact JSON string with each shot as an object: `[{"c":3},{"c":0}]`
    #[must_use]
    pub fn to_compact_json(&self) -> String {
        self.shot_vec.to_compact_json()
    }

    /// Get outcome counts for each register
    ///
    /// Returns a map where each register name maps to another map of outcome values and their counts.
    #[must_use]
    pub fn outcome_counts(&self) -> HashMap<String, HashMap<u64, usize>> {
        let mut result = HashMap::new();
        let register_names = self.shot_vec.get_register_names();

        for name in register_names {
            let mut counts = HashMap::new();

            for shot in &self.shot_vec.shots {
                if let Some(value) = shot.data.get(&name).and_then(|d| match d {
                    Data::U32(v) => Some(u64::from(*v)),
                    Data::BitVec(bv) => {
                        let mut value = 0u64;
                        for (i, bit) in bv.iter().enumerate() {
                            if *bit && i < 64 {
                                value |= 1u64 << i;
                            }
                        }
                        Some(value)
                    }
                    _ => None,
                }) {
                    *counts.entry(value).or_insert(0) += 1;
                }
            }

            result.insert(name, counts);
        }

        result
    }
}

// Implement Display for convenient printing
impl std::fmt::Display for QASMResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_compact_json())
    }
}

// Allow dereferencing to the underlying ShotVec for direct access
impl std::ops::Deref for QASMResults {
    type Target = ShotVec;

    fn deref(&self) -> &Self::Target {
        &self.shot_vec
    }
}

// Allow conversion from ShotVec
impl From<ShotVec> for QASMResults {
    fn from(shot_vec: ShotVec) -> Self {
        Self::new(shot_vec)
    }
}

// Allow conversion back to ShotVec
impl From<QASMResults> for ShotVec {
    fn from(results: QASMResults) -> Self {
        results.into_shot_vec()
    }
}

// Custom serialization to output binary format by default
impl Serialize for QASMResults {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as the binary JSON representation
        self.to_binary_json().serialize(serializer)
    }
}

// Custom deserialization from a map of register arrays
impl<'de> Deserialize<'de> for QASMResults {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        // Deserialize as a JSON object with register arrays
        let value = Value::deserialize(deserializer)?;

        let obj = value
            .as_object()
            .ok_or_else(|| de::Error::custom("Expected object with register arrays"))?;

        let mut shot_vec = ShotVec::new();

        // Get the length from the first array
        let shot_count = obj
            .values()
            .find_map(|v| v.as_array().map(std::vec::Vec::len))
            .unwrap_or(0);

        // Create shots
        for i in 0..shot_count {
            let mut shot = Shot::default();

            for (reg_name, values) in obj {
                if let Some(array) = values.as_array() {
                    if let Some(val) = array.get(i).and_then(serde_json::Value::as_u64) {
                        if let Ok(val_u32) = u32::try_from(val) {
                            shot.data.insert(reg_name.clone(), Data::U32(val_u32));
                        }
                    }
                }
            }

            shot_vec.shots.push(shot);
        }

        Ok(QASMResults::new(shot_vec))
    }
}
