//! Shared helpers for noise model parsing and validation

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::BTreeMap;

/// Maximum safe f64 value that can be exactly converted to u64
pub const MAX_SAFE_U64: f64 = 9_007_199_254_740_992.0; // 2^53

/// Extract an optional f64 value from a Python object attribute
pub fn get_optional_f64(obj: &Bound<'_, PyAny>, attr: &str) -> PyResult<Option<f64>> {
    match obj.getattr(attr) {
        Ok(val) => {
            if val.is_none() {
                Ok(None)
            } else {
                Ok(Some(val.extract()?))
            }
        }
        Err(_) => Ok(None),
    }
}

/// Extract an optional bool value from a Python object attribute
pub fn get_optional_bool(obj: &Bound<'_, PyAny>, attr: &str) -> PyResult<Option<bool>> {
    match obj.getattr(attr) {
        Ok(val) => {
            if val.is_none() {
                Ok(None)
            } else {
                Ok(Some(val.extract()?))
            }
        }
        Err(_) => Ok(None),
    }
}

/// Extract an optional dictionary from a Python object attribute
pub fn get_optional_dict(
    obj: &Bound<'_, PyAny>,
    attr: &str,
) -> PyResult<Option<BTreeMap<String, f64>>> {
    match obj.getattr(attr) {
        Ok(val) => {
            if val.is_none() {
                Ok(None)
            } else {
                let dict: &Bound<'_, PyDict> = val.downcast()?;
                let mut map = BTreeMap::new();
                for (key, value) in dict.iter() {
                    let key_str: String = key.extract()?;
                    let val_f64: f64 = value.extract()?;
                    map.insert(key_str, val_f64);
                }
                Ok(Some(map))
            }
        }
        Err(_) => Ok(None),
    }
}

/// Validate and convert f64 to u64 for seed values
///
/// Uses `MAX_SAFE_U64` (2^53) as the upper bound since f64 can only represent
/// integers exactly up to that value. Beyond that, precision is lost.
pub fn validate_and_convert_seed(seed: f64) -> PyResult<u64> {
    // Check for NaN and infinity
    if !seed.is_finite() {
        return Err(PyValueError::new_err("Seed must be a finite number"));
    }

    // Check for negative values (also handles -0.0)
    if seed < 0.0 {
        return Err(PyValueError::new_err("Seed must be non-negative"));
    }

    // Check if the value has a fractional part
    if seed.fract() != 0.0 {
        return Err(PyValueError::new_err("Seed must be a whole number"));
    }

    // Use `MAX_SAFE_U64` to ensure exact representation in f64
    // This avoids precision loss since we're staying within f64's exact range
    if seed >= MAX_SAFE_U64 {
        return Err(PyValueError::new_err(
            "Seed value too large (must be less than 2^53 for exact representation)",
        ));
    }

    // Since we've validated all constraints, the cast is safe
    // but clippy doesn't know this. In this specific case, using allow
    // is justified because we've done comprehensive validation.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = seed as u64;
    Ok(result)
}
