// Example of a simpler approach to ShotVec JSON conversion

use serde_json::{Map, Value};
use crate::shot_results::{Data, ShotVec};

impl ShotVec {
    /// Simpler JSON conversion - just convert everything to its natural JSON representation
    pub fn to_simple_json(&self) -> Value {
        let shots: Vec<Value> = self.shots
            .iter()
            .map(|shot| {
                let mut obj = Map::new();
                
                for (key, data) in &shot.data {
                    // Skip metadata
                    if key.starts_with('_') {
                        continue;
                    }
                    
                    // Use the natural serde serialization for each type
                    if let Ok(value) = serde_json::to_value(data) {
                        obj.insert(key.clone(), value);
                    }
                }
                
                Value::Object(obj)
            })
            .collect();
        
        Value::Array(shots)
    }
    
    /// Alternative: Add a method to Data for "measurement value" conversion
    /// This separates the concern of "what is a measurement result" from JSON serialization
    pub fn to_measurement_json(&self) -> Value {
        let shots: Vec<Value> = self.shots
            .iter()
            .map(|shot| {
                let mut obj = Map::new();
                
                for (key, data) in &shot.data {
                    if key.starts_with('_') {
                        continue;
                    }
                    
                    // Only include data that can be interpreted as measurement results
                    if let Some(value) = data.as_measurement_value() {
                        obj.insert(key.clone(), Value::Number(value.into()));
                    }
                }
                
                Value::Object(obj)
            })
            .collect();
        
        Value::Array(shots)
    }
}

impl Data {
    /// Convert to a measurement value (decimal number) if this data type 
    /// represents a quantum measurement result
    pub fn as_measurement_value(&self) -> Option<u64> {
        match self {
            Data::U8(v) => Some(*v as u64),
            Data::U16(v) => Some(*v as u64),
            Data::U32(v) => Some(*v as u64),
            Data::U64(v) => Some(*v),
            Data::Bool(v) => Some(if *v { 1 } else { 0 }),
            Data::BitVec(bv) => {
                // Convert BitVec to decimal (up to 64 bits)
                let mut value = 0u64;
                for (i, bit) in bv.iter().take(64).enumerate() {
                    if *bit {
                        value |= 1u64 << i;
                    }
                }
                Some(value)
            }
            Data::BigInt(v) => u64::try_from(v).ok(),
            _ => None, // Other types aren't measurement results
        }
    }
}