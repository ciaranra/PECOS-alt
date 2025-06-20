/*!
HUGR Type Translation Layer

This module creates a translation layer between old HUGR (from Guppy) and new HUGR (with LLVM support).

Key differences between versions:
1. OLD: Types identified by simple names like "int", "bool"
2. NEW: Types identified by extension-qualified keys like ("arithmetic.int.types", "int")
3. OLD: Limited extension support in hugr-llvm 0.20.1
4. NEW: Full extension-qualified type system with custom type callbacks

Transformation approach:
1. Parse the HUGR JSON from Guppy (old format)
2. Transform type representations to be compatible with hugr-llvm 0.20.1
3. Handle both arithmetic.int.types and bool types
4. Transform operation signatures that reference these types
5. Reconstruct HUGR with compatible types

This is a temporary solution until Guppy updates to newer HUGR with LLVM support.
*/

use log::info;
use pecos_core::errors::PecosError;
use serde_json::{Map, Value};

/// Transform HUGR bytes to make arithmetic types compatible with hugr-llvm
///
/// # Errors
/// Returns `PecosError` if:
/// - The HUGR bytes don't contain valid JSON data
/// - The JSON cannot be parsed
/// - Serialization of the transformed HUGR fails
pub fn transform_hugr_types(hugr_bytes: &[u8]) -> Result<Vec<u8>, PecosError> {
    info!("Starting HUGR type transformation for hugr-llvm compatibility");
    info!("Input HUGR size: {} bytes", hugr_bytes.len());
    eprintln!(
        "DEBUG: Starting HUGR type transformation, input size: {} bytes",
        hugr_bytes.len()
    );

    // HUGR format: header + JSON
    // Find where JSON starts (after the header)
    let json_start = hugr_bytes
        .iter()
        .position(|&b| b == b'{')
        .ok_or_else(|| PecosError::Processing("HUGR doesn't contain JSON data".to_string()))?;

    let header = &hugr_bytes[..json_start];
    let json_bytes = &hugr_bytes[json_start..];

    // Parse JSON
    let json_str = std::str::from_utf8(json_bytes)
        .map_err(|e| PecosError::with_context(e, "Invalid UTF-8 in HUGR JSON"))?;

    let mut json: Value = serde_json::from_str(json_str)
        .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR JSON"))?;

    // Transform the types
    let transform_count = transform_types_recursive(&mut json);
    info!("Transformed {} type instances", transform_count);

    // Also handle extension definitions to prevent conflicts
    handle_extension_conflicts(&mut json);

    // Serialize back to JSON
    let transformed_json = serde_json::to_string(&json)
        .map_err(|e| PecosError::with_context(e, "Failed to serialize transformed HUGR"))?;

    // Reconstruct HUGR with header + transformed JSON
    let mut result = header.to_vec();
    result.extend_from_slice(transformed_json.as_bytes());

    Ok(result)
}

/// Recursively transform types in the JSON structure
fn transform_types_recursive(value: &mut Value) -> usize {
    let mut count = 0;

    match value {
        Value::Object(map) => {
            // Check if this is an opaque arithmetic type
            if is_arithmetic_int_type(map) {
                if transform_arithmetic_int_type(map) {
                    count += 1;
                }
            } else if is_bool_type(map) {
                if transform_bool_type(map) {
                    count += 1;
                }
            } else if is_operation_signature(map) {
                // Transform operation signatures that reference old types
                if transform_operation_signature(map) {
                    count += 1;
                }
            } else if is_constant_value(map) {
                // Transform constant values
                if transform_constant_value(map) {
                    count += 1;
                }
            } else if is_operation_definition(map) {
                // Transform operation definitions to handle type mismatches
                if transform_operation_definition(map) {
                    count += 1;
                }
            }

            // Recurse into all values
            for (_, v) in map.iter_mut() {
                count += transform_types_recursive(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                count += transform_types_recursive(v);
            }
        }
        _ => {}
    }

    count
}

/// Check if this is an arithmetic integer type
fn is_arithmetic_int_type(map: &Map<String, Value>) -> bool {
    map.get("t").and_then(|t| t.as_str()) == Some("Opaque")
        && map.get("extension").and_then(|e| e.as_str()) == Some("arithmetic.int.types")
        && map.get("id").and_then(|id| id.as_str()) == Some("int")
}

/// Check if this is a boolean type
fn is_bool_type(map: &Map<String, Value>) -> bool {
    map.get("t").and_then(|t| t.as_str()) == Some("Opaque")
        && (map.get("extension").and_then(|e| e.as_str()) == Some("tket2.bool")
            || map.get("extension").and_then(|e| e.as_str()) == Some("prelude")
                && map.get("id").and_then(|id| id.as_str()) == Some("bool"))
}

/// Transform arithmetic integer type to be compatible with hugr-llvm 0.20.1
///
/// The key insight: newer hugr-llvm uses extension-qualified type keys like
/// ("arithmetic.int.types", "int") but version 0.20.1 has hardcoded concrete
/// implementations that expect specific type formats.
///
/// Solution: Transform to a type format that hugr-llvm 0.20.1 can handle while
/// preserving the semantic intent of integer operations.
fn transform_arithmetic_int_type(map: &mut Map<String, Value>) -> bool {
    // Extract the bit width from args to preserve semantic information
    let bit_width = map
        .get("args")
        .and_then(|args| args.as_array())
        .and_then(|arr| arr.first())
        .and_then(|arg| arg.as_object())
        .and_then(|obj| obj.get("n"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(6); // Default to 64-bit (log_width=6) if we can't determine

    info!(
        "Transforming arithmetic.int.types with {} bit width (log_width={})",
        1 << bit_width,
        bit_width
    );
    eprintln!("DEBUG: Creating hugr-llvm 0.20.1 compatible int type for bit_width={bit_width}");

    // Strategy: Map to a concrete integer type that hugr-llvm 0.20.1 understands.
    // We'll use an approach similar to newer hugr-llvm but compatible with 0.20.1

    // Map different bit widths to appropriate LLVM integer types
    // This follows the pattern from newer hugr-llvm/src/extension/int.rs:1100-1110
    let llvm_type_width = match bit_width {
        0..=3 => 8, // i8 for small integers
        4 => 16,    // i16 for log_width=4 (16 bits)
        5 => 32,    // i32 for log_width=5 (32 bits)
        _ => 64,    // i64 for log_width=6 (64 bits) and anything larger
    };

    // Clear the old arithmetic.int.types format
    map.clear();

    // Transform to a basic integer type that hugr-llvm 0.20.1 can handle
    // Using "I" (Integer) type from the valid type variants we discovered
    map.insert("t".to_string(), Value::String("I".to_string()));

    // Store the bit width as metadata in case we need it later
    map.insert("width".to_string(), Value::Number(llvm_type_width.into()));

    info!(
        "Transformed int({}) to basic integer type with width {}",
        1 << bit_width,
        llvm_type_width
    );
    true
}

/// Transform boolean type to be compatible with hugr-llvm 0.20.1
///
/// The newer hugr-llvm handles booleans as sum types with two unit variants.
/// hugr-llvm 0.20.1 should be able to handle this representation.
fn transform_bool_type(map: &mut Map<String, Value>) -> bool {
    info!("Transforming boolean type to sum type representation");
    eprintln!("DEBUG: Converting bool to Sum type with 2 unit variants");

    // Clear the old opaque bool format
    map.clear();

    // Convert to a sum type (unit sum with 2 variants) which is the standard
    // representation for booleans in HUGR that hugr-llvm can handle
    map.insert("t".to_string(), Value::String("Sum".to_string()));
    map.insert("s".to_string(), Value::String("Unit".to_string()));
    map.insert("size".to_string(), Value::Number(2.into()));

    info!("Successfully transformed bool to Sum type");
    true
}

/// Check if this is an operation signature that might reference old types
fn is_operation_signature(map: &Map<String, Value>) -> bool {
    // Look for signatures in extensions that might contain arithmetic.int.types
    if let Some(signature) = map.get("signature") {
        if let Some(sig_obj) = signature.as_object() {
            if let Some(body) = sig_obj.get("body") {
                // Check if the signature body contains arithmetic.int.types
                return contains_arithmetic_int_types(body);
            }
        }
    }
    false
}

/// Check if a value contains arithmetic.int.types references
fn contains_arithmetic_int_types(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            if is_arithmetic_int_type(map) {
                return true;
            }
            for v in map.values() {
                if contains_arithmetic_int_types(v) {
                    return true;
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                if contains_arithmetic_int_types(v) {
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}

/// Transform operation signature types
fn transform_operation_signature(map: &mut Map<String, Value>) -> bool {
    eprintln!("DEBUG: Found potential operation signature");

    if let Some(signature) = map.get_mut("signature") {
        eprintln!("DEBUG: Processing signature");
        if let Some(sig_obj) = signature.as_object_mut() {
            if let Some(body) = sig_obj.get_mut("body") {
                eprintln!("DEBUG: Transforming signature body");
                let count = transform_types_recursive(body);
                if count > 0 {
                    eprintln!("DEBUG: Transformed {count} types in operation signature");
                    return true;
                }
            }
        }
    }

    false
}

/// Check if this is a constant value that needs transformation
fn is_constant_value(map: &Map<String, Value>) -> bool {
    // Check for const nodes with arithmetic extension
    if let Some(op) = map.get("op") {
        if let Some(op_str) = op.as_str() {
            if op_str == "Const" || op_str == "LoadConstant" {
                return true;
            }
        }
    }

    // Check for values with extension metadata
    if map.contains_key("v") && (map.contains_key("extension") || map.contains_key("typ")) {
        return true;
    }

    false
}

/// Transform constant values to be compatible with hugr-llvm
fn transform_constant_value(map: &mut Map<String, Value>) -> bool {
    eprintln!("DEBUG: Transforming constant value");

    // If this is a constant with an extension, we need to handle it
    if let Some(extension) = map.get("extension").and_then(|e| e.as_str()) {
        if extension == "arithmetic.int.types" {
            // Transform the constant to a format hugr-llvm can handle
            eprintln!("DEBUG: Found arithmetic constant");

            // Remove extension metadata that hugr-llvm doesn't understand
            map.remove("extension");

            // If there's a type field, transform it
            if let Some(typ) = map.get_mut("typ") {
                transform_types_recursive(typ);
            }

            return true;
        }
    }

    // Handle Const operations
    if let Some(op) = map.get("op").and_then(|o| o.as_str()) {
        if op == "Const" || op == "LoadConstant" {
            // Transform any type references in the const
            if let Some(v) = map.get_mut("v") {
                transform_types_recursive(v);
            }
            if let Some(typ) = map.get_mut("typ") {
                transform_types_recursive(typ);
            }
            return true;
        }
    }

    false
}

/// Check if this is an operation definition
fn is_operation_definition(map: &Map<String, Value>) -> bool {
    // Look for operations in arithmetic extension
    if let Some(op) = map.get("op") {
        if let Some(op_obj) = op.as_object() {
            if let Some(extension) = op_obj.get("extension").and_then(|e| e.as_str()) {
                return extension == "arithmetic.int.types" || extension == "arithmetic.int.ops";
            }
        }
    }

    // Also check for op_name patterns
    if map.contains_key("op_name") || map.contains_key("op_type") {
        return true;
    }

    false
}

/// Transform operation definitions to handle signature mismatches
fn transform_operation_definition(map: &mut Map<String, Value>) -> bool {
    eprintln!("DEBUG: Transforming operation definition");

    // Handle operations with extension metadata
    if let Some(op) = map.get_mut("op") {
        if let Some(op_obj) = op.as_object_mut() {
            // Transform the signature if present
            if let Some(sig) = op_obj.get_mut("signature") {
                transform_types_recursive(sig);
            }

            // Handle extension-specific operations
            if let Some(extension) = op_obj.get("extension").and_then(|e| e.as_str()) {
                if extension == "arithmetic.int.types" || extension == "arithmetic.int.ops" {
                    eprintln!("DEBUG: Found arithmetic operation");

                    // Transform any type references in the operation
                    if let Some(args) = op_obj.get_mut("args") {
                        transform_types_recursive(args);
                    }

                    // Special handling for iadd and other arithmetic ops
                    if let Some(op_name) = op_obj.get("op_name").and_then(|n| n.as_str()) {
                        eprintln!("DEBUG: Operation name: {op_name}");

                        // For operations like iadd, we need to ensure the signature matches
                        // what hugr-llvm expects
                        if op_name == "iadd" || op_name == "isub" || op_name == "imul" {
                            // These operations should have int(6) types, not usize
                            if let Some(sig) = op_obj.get_mut("signature") {
                                fix_arithmetic_op_signature(sig);
                            }
                        }
                    }

                    return true;
                }
            }
        }
    }

    false
}

/// Fix arithmetic operation signatures to use int(6) instead of usize
fn fix_arithmetic_op_signature(sig: &mut Value) {
    if let Some(sig_obj) = sig.as_object_mut() {
        if let Some(body) = sig_obj.get_mut("body") {
            if let Some(body_obj) = body.as_object_mut() {
                // Fix input types
                if let Some(input) = body_obj.get_mut("input") {
                    if let Some(input_arr) = input.as_array_mut() {
                        for typ in input_arr.iter_mut() {
                            if let Some(typ_obj) = typ.as_object_mut() {
                                // Replace usize with int(6)
                                if typ_obj.get("t").and_then(|t| t.as_str()) == Some("usize") {
                                    typ_obj.clear();
                                    typ_obj.insert("t".to_string(), Value::String("I".to_string()));
                                    typ_obj.insert("width".to_string(), Value::Number(64.into()));
                                }
                            }
                        }
                    }
                }

                // Fix output types
                if let Some(output) = body_obj.get_mut("output") {
                    if let Some(output_arr) = output.as_array_mut() {
                        for typ in output_arr.iter_mut() {
                            if let Some(typ_obj) = typ.as_object_mut() {
                                // Replace usize with int(6)
                                if typ_obj.get("t").and_then(|t| t.as_str()) == Some("usize") {
                                    typ_obj.clear();
                                    typ_obj.insert("t".to_string(), Value::String("I".to_string()));
                                    typ_obj.insert("width".to_string(), Value::Number(64.into()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Handle extension conflicts by removing problematic extension operations
fn handle_extension_conflicts(json: &mut Value) {
    eprintln!("DEBUG: Handling extension conflicts");

    // If there's an extensions array, process it
    if let Some(extensions) = json
        .as_object_mut()
        .and_then(|obj| obj.get_mut("extensions"))
    {
        if let Some(extensions_arr) = extensions.as_array_mut() {
            for ext in extensions_arr.iter_mut() {
                if let Some(ext_obj) = ext.as_object_mut() {
                    // Check if this is the arithmetic.int extension
                    if ext_obj.get("name").and_then(|n| n.as_str()) == Some("arithmetic.int") {
                        eprintln!("DEBUG: Found arithmetic.int extension, checking operations");

                        // Remove or fix operations that might conflict
                        if let Some(operations) = ext_obj.get_mut("operations") {
                            if let Some(ops_obj) = operations.as_object_mut() {
                                // Remove the iadd operation definition if it exists
                                if ops_obj.contains_key("iadd") {
                                    eprintln!(
                                        "DEBUG: Removing iadd operation definition from extension"
                                    );
                                    ops_obj.remove("iadd");
                                }

                                // Remove other arithmetic operations that might conflict
                                let ops_to_remove = vec!["isub", "imul", "idiv"];
                                for op in ops_to_remove {
                                    if ops_obj.contains_key(op) {
                                        eprintln!(
                                            "DEBUG: Removing {op} operation definition from extension"
                                        );
                                        ops_obj.remove(op);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Also check modules for Extension nodes that define operations
    if let Some(modules) = json.as_object_mut().and_then(|obj| obj.get_mut("modules")) {
        if let Some(modules_arr) = modules.as_array_mut() {
            for module in modules_arr.iter_mut() {
                if let Some(module_obj) = module.as_object_mut() {
                    if let Some(nodes) = module_obj.get_mut("nodes") {
                        if let Some(nodes_arr) = nodes.as_array_mut() {
                            // Process nodes to handle Extension operations
                            for node in nodes_arr.iter_mut() {
                                if let Some(node_obj) = node.as_object_mut() {
                                    if let Some(op) = node_obj.get("op").and_then(|o| o.as_str()) {
                                        if op == "Extension" {
                                            // Check if this is an arithmetic extension operation
                                            if let Some(extension) =
                                                node_obj.get("extension").and_then(|e| e.as_str())
                                            {
                                                if extension == "arithmetic.int"
                                                    || extension == "arithmetic.int.ops"
                                                {
                                                    eprintln!(
                                                        "DEBUG: Found Extension node for arithmetic operations"
                                                    );

                                                    // Transform the signature to avoid conflicts
                                                    if let Some(sig) = node_obj.get_mut("signature")
                                                    {
                                                        transform_types_recursive(sig);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_arithmetic_int() {
        let mut map = serde_json::json!({
            "t": "Opaque",
            "extension": "arithmetic.int.types",
            "id": "int",
            "args": [{"tya": "BoundedNat", "n": 6}],
            "bound": "C"
        })
        .as_object_mut()
        .unwrap()
        .clone();

        assert!(is_arithmetic_int_type(&map));
        assert!(transform_arithmetic_int_type(&mut map));

        // Should transform to basic integer type
        assert_eq!(map.get("t").unwrap(), "I");
        assert_eq!(map.get("width").unwrap(), 64); // log_width=6 -> 64-bit integer
        assert!(!map.contains_key("extension"));
    }

    #[test]
    fn test_transform_bool() {
        let mut map = serde_json::json!({
            "t": "Opaque",
            "extension": "tket2.bool",
            "id": "bool",
            "args": [],
            "bound": "C"
        })
        .as_object_mut()
        .unwrap()
        .clone();

        assert!(is_bool_type(&map));
        assert!(transform_bool_type(&mut map));

        assert_eq!(map.get("t").unwrap(), "Sum");
        assert_eq!(map.get("s").unwrap(), "Unit");
        assert_eq!(map.get("size").unwrap(), 2);
    }
}
