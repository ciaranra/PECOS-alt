//! Common test utilities for LLVM simulation tests

use pecos_engines::shot_results::ShotMap;

/// Helper to get register values as i64 from `ShotMap`
///
/// # Errors
///
/// Returns an error if the register is not found or values cannot be converted
pub fn get_register_i64(
    shot_map: &ShotMap,
    register: &str,
) -> Result<Vec<i64>, Box<dyn std::error::Error>> {
    // Try to get as BitVec first (most common for quantum registers)
    if let Ok(values) = shot_map.try_bits_as_u64(register) {
        Ok(values
            .into_iter()
            .map(|v| i64::try_from(v).unwrap_or(v as i64))
            .collect())
    }
    // Try as i64 directly
    else if let Ok(values) = shot_map.try_i64s(register) {
        Ok(values)
    }
    // Try as u32 and convert
    else if let Ok(values) = shot_map.try_u32s(register) {
        Ok(values.into_iter().map(i64::from).collect())
    } else {
        Err(format!("Cannot get register '{register}' as i64 values").into())
    }
}
