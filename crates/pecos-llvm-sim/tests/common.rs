//! Common test utilities for LLVM simulation tests

use pecos_engines::shot_results::{ShotVec, ShotMap};
use std::collections::HashMap;

/// Convert a ShotVec to a HashMap<String, Vec<i64>> for tests that expect the old format
pub fn shot_vec_to_hashmap(shot_vec: ShotVec) -> Result<HashMap<String, Vec<i64>>, Box<dyn std::error::Error>> {
    let shot_map = shot_vec.try_as_shot_map()?;
    let mut result = HashMap::new();
    
    for register in shot_map.register_names() {
        // Try to get as BitVec first (most common for quantum registers)
        if let Ok(values) = shot_map.try_bits_as_u64(&register) {
            let i64_values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
            result.insert(register.to_string(), i64_values);
        }
        // Try as i64 directly
        else if let Ok(values) = shot_map.try_i64s(&register) {
            result.insert(register.to_string(), values);
        }
        // Try as u32 and convert
        else if let Ok(values) = shot_map.try_u32s(&register) {
            let i64_values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
            result.insert(register.to_string(), i64_values);
        }
        // Default to zeros if we can't convert
        else {
            let zeros = vec![0i64; shot_map.num_shots()];
            result.insert(register.to_string(), zeros);
        }
    }
    
    Ok(result)
}

/// Helper to get register values as i64 from ShotMap
pub fn get_register_i64(shot_map: &ShotMap, register: &str) -> Result<Vec<i64>, Box<dyn std::error::Error>> {
    // Try to get as BitVec first (most common for quantum registers)
    if let Ok(values) = shot_map.try_bits_as_u64(register) {
        Ok(values.into_iter().map(|v| v as i64).collect())
    }
    // Try as i64 directly
    else if let Ok(values) = shot_map.try_i64s(register) {
        Ok(values)
    }
    // Try as u32 and convert
    else if let Ok(values) = shot_map.try_u32s(register) {
        Ok(values.into_iter().map(|v| v as i64).collect())
    }
    else {
        Err(format!("Cannot get register '{}' as i64 values", register).into())
    }
}