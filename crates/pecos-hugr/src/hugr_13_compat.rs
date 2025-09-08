//! HUGR 0.13 compatibility layer
//!
//! This module provides compatibility for HUGR 0.13 types when loading
//! HUGR packages from older versions (e.g., from guppylang).

use pecos_core::errors::PecosError;
use serde_json::Value;

/// Convert HUGR 0.13 types to HUGR 0.20 equivalents
pub fn convert_hugr_13_types(hugr_json: &mut Value) -> Result<(), PecosError> {
    // Convert List types to Array types
    convert_list_to_array(hugr_json)?;

    // Add other type conversions as needed

    Ok(())
}

/// Convert HUGR 0.13 List types to HUGR 0.20 Array types
fn convert_list_to_array(value: &mut Value) -> Result<(), PecosError> {
    match value {
        Value::Object(map) => {
            // Check if this is a type argument with "tya":"List"
            if let Some(Value::String(tya)) = map.get("tya")
                && tya == "List"
            {
                // This is a List type argument, convert it to Array
                map.insert("tya".to_string(), Value::String("Array".to_string()));
                log::debug!("Converted List to Array in tya field");

                // Also update the elems field name to values if present
                if let Some(elems) = map.remove("elems") {
                    map.insert("values".to_string(), elems);
                    log::debug!("Renamed 'elems' to 'values' for Array type");
                }
            }

            // Convert various fields that might contain "List"
            let fields_to_check = vec!["t", "variant", "tp", "type", "tag"];

            for field in fields_to_check {
                if let Some(Value::String(s)) = map.get(field)
                    && s == "List"
                {
                    map.insert(field.to_string(), Value::String("Array".to_string()));
                    log::debug!("Converted List to Array in field: {field}");
                }
            }

            // Also check for any string value that contains List
            for (key, val) in &map.clone() {
                if let Value::String(s) = val
                    && s.contains("List")
                {
                    let new_val = s.replace("List", "Array");
                    map.insert(key.clone(), Value::String(new_val));
                    log::debug!(
                        "Replaced List with Array in field {}: {} -> {}",
                        key,
                        s,
                        map[key]
                    );
                }
            }

            // Update extension references
            if let Some(Value::String(ext)) = map.get_mut("extension")
                && ext.contains("list")
            {
                *ext = ext.replace("list", "array");
                log::debug!("Updated extension: {ext}");
            }

            // If we converted a List variant, ensure it has the right extension
            if let Some(Value::String(variant)) = map.get("variant")
                && variant == "Array"
                && !map.contains_key("extension")
            {
                map.insert(
                    "extension".to_string(),
                    Value::String("collections.array".to_string()),
                );
            }

            // Recursively process all values in the object
            for (_, v) in map.iter_mut() {
                convert_list_to_array(v)?;
            }
        }
        Value::Array(arr) => {
            // Recursively process all values in the array
            for v in arr.iter_mut() {
                convert_list_to_array(v)?;
            }
        }
        _ => {
            // Other value types don't need processing
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_to_array_conversion() {
        let mut json = json!({
            "type": {
                "variant": "List",
                "extension": "collections.list",
                "args": [{
                    "t": "Type",
                    "value": "Int"
                }]
            }
        });

        convert_list_to_array(&mut json).unwrap();

        assert_eq!(json["type"]["variant"], "Array");
        assert_eq!(json["type"]["extension"], "collections.array");
    }
}
