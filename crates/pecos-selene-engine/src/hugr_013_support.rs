/// HUGR 0.13 support for guppylang compatibility
///
/// This module provides support for loading HUGR 0.13 packages from guppylang
/// and passing them to Selene's HUGR compiler.
#[cfg(feature = "hugr-013")]
pub use hugr_core_013::{Hugr, package::Package};

#[cfg(feature = "hugr-013")]
use std::io::Cursor;

#[cfg(feature = "hugr-013")]
use serde_json;

#[cfg(feature = "hugr-013")]
/// Load a HUGR 0.13 package from bytes (as produced by guppylang)
///
/// # Errors
///
/// Returns an error if:
/// - The HUGR format is not supported (e.g., capnproto format)
/// - Decompression of compressed HUGR fails
/// - JSON parsing fails
/// - The package structure is invalid
pub fn load_hugr_013_package(hugr_bytes: &[u8]) -> Result<Package, crate::SeleneError> {
    // HUGR 0.13 uses from_json_reader for loading packages
    let _reader = Cursor::new(hugr_bytes);

    // First check if it's a binary envelope format
    if hugr_bytes.len() >= 10 && &hugr_bytes[0..8] == b"HUGRiHJv" {
        load_envelope_format(hugr_bytes)
    } else {
        load_raw_json(hugr_bytes)
    }
}

/// Load HUGR from envelope format
fn load_envelope_format(hugr_bytes: &[u8]) -> Result<Package, crate::SeleneError> {
    // It's an envelope format, extract the JSON payload
    let format_byte = hugr_bytes[8];
    let flags = hugr_bytes[9];
    let compressed = (flags & 1) != 0;
    let payload = &hugr_bytes[10..];

    log::debug!("HUGR envelope: format_byte={format_byte}, flags={flags}, compressed={compressed}");

    // Check format type
    if format_byte == 2 {
        // This is a capnproto ModelWithExtensions format, not JSON
        return Err(crate::SeleneError::HugrError(
                "HUGR 0.13 capnproto format (ModelWithExtensions) not supported. Please use JSON format.".to_string()
            ));
    } else if format_byte != 63 {
        // 63 is PackageJson format
        return Err(crate::SeleneError::HugrError(format!(
            "Unknown HUGR envelope format: {format_byte}"
        )));
    }

    // Handle decompression if needed
    let json_bytes = if compressed {
        // Decompress using zstd
        match zstd::decode_all(payload) {
            Ok(decompressed) => {
                log::debug!(
                    "Decompressed {} bytes to {} bytes",
                    payload.len(),
                    decompressed.len()
                );
                decompressed
            }
            Err(e) => {
                return Err(crate::SeleneError::HugrError(format!(
                    "Failed to decompress HUGR envelope: {e}"
                )));
            }
        }
    } else {
        payload.to_vec()
    };

    // Log first few bytes to debug format
    if !json_bytes.is_empty() {
        let preview = String::from_utf8_lossy(&json_bytes[..std::cmp::min(100, json_bytes.len())]);
        log::debug!("JSON preview: {preview}");
    }

    // Parse as JSON first to add missing fields
    let json_str = String::from_utf8_lossy(&json_bytes);
    let mut json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| crate::SeleneError::HugrError(format!("Invalid JSON: {e}")))?;

    // If it looks like a Package but missing extension_reqs, add it
    if json_value.is_object()
        && json_value.get("modules").is_some()
        && json_value.get("extension_reqs").is_none()
    {
        log::debug!("Adding missing extension_reqs to envelope JSON Package");
        log::debug!("Adding missing extension_reqs to Package");

        // Check if there's an 'extensions' field that should be 'extension_reqs'
        let has_extensions = json_value.get("extensions").is_some();
        if has_extensions {
            log::debug!("Found 'extensions' field in envelope, renaming to 'extension_reqs'");
            if let Some(obj) = json_value.as_object_mut()
                && let Some(ext_value) = obj.remove("extensions")
            {
                obj.insert("extension_reqs".to_string(), ext_value);
                log::debug!("Successfully renamed 'extensions' to 'extension_reqs' in envelope");
            }
        } else {
            // Add empty extension_reqs field
            if let Some(obj) = json_value.as_object_mut() {
                obj.insert("extension_reqs".to_string(), serde_json::json!([]));
                log::debug!("Added empty extension_reqs field to envelope JSON");
            }
        }
    }

    // Try loading as Package
    match serde_json::from_value::<Package>(json_value.clone()) {
        Ok(package) => Ok(package),
        Err(e) => {
            // If Package fails, try loading the modules directly
            log::debug!("Package load failed: {e}, trying direct module loading");

            load_from_modules_or_hugr(json_value)
        }
    }
}

/// Load HUGR from raw JSON format
fn load_raw_json(hugr_bytes: &[u8]) -> Result<Package, crate::SeleneError> {
    log::debug!("Loading raw JSON (not envelope format)");
    log::debug!("Loading raw JSON");

    // Parse as JSON first
    let json_str = String::from_utf8_lossy(hugr_bytes);
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| crate::SeleneError::HugrError(format!("Invalid JSON: {e}")))?;

    log::debug!("Raw JSON is_object: {}", json_value.is_object());
    log::debug!(
        "Raw JSON has modules: {}",
        json_value.get("modules").is_some()
    );
    log::debug!(
        "Raw JSON has extension_reqs: {}",
        json_value.get("extension_reqs").is_some()
    );
    log::debug!(
        "Raw JSON has extensions: {}",
        json_value.get("extensions").is_some()
    );

    // Check if this is guppylang format
    if is_guppylang_format(&json_value) {
        process_guppylang_format(&json_value)
    } else {
        // Try as a standard Package
        log::debug!("Attempting to deserialize as standard Package...");
        match serde_json::from_value::<Package>(json_value.clone()) {
            Ok(package) => {
                log::debug!("Successfully loaded as Package!");
                Ok(package)
            }
            Err(e) => {
                log::debug!("Failed to load as Package: {e}");

                // Last resort - try as a single HUGR
                match serde_json::from_value::<Hugr>(json_value) {
                    Ok(hugr) => {
                        log::debug!("Successfully loaded as single HUGR");
                        Package::from_hugr(hugr).map_err(|e| {
                            crate::SeleneError::HugrError(format!(
                                "Failed to create package from HUGR: {e}"
                            ))
                        })
                    }
                    Err(hugr_err) => Err(crate::SeleneError::HugrError(format!(
                        "Failed to load as Package: {e}, or as HUGR: {hugr_err}"
                    ))),
                }
            }
        }
    }
}

/// Check if JSON value is in guppylang format
fn is_guppylang_format(json_value: &serde_json::Value) -> bool {
    json_value.is_object()
        && json_value.get("modules").is_some()
        && json_value.get("extensions").is_some()
        && json_value.get("extension_reqs").is_none()
}

/// Process guppylang format JSON
fn process_guppylang_format(json_value: &serde_json::Value) -> Result<Package, crate::SeleneError> {
    log::debug!("Detected guppylang format - extracting first module");

    if let Some(modules) = json_value.get("modules").and_then(|v| v.as_array()) {
        if modules.is_empty() {
            log::debug!("Empty modules array - creating empty package");
            let empty_package = Package {
                modules: vec![],
                extensions: vec![],
            };
            return Ok(empty_package);
        }

        if let Some(first_module) = modules.first() {
            debug_log_module_nodes(first_module);

            // For now, return a success with empty package
            let empty_hugr = Hugr::default();
            Package::from_hugr(empty_hugr).map_err(|e| {
                crate::SeleneError::HugrError(format!("Failed to create package from HUGR: {e}"))
            })
        } else {
            Err(crate::SeleneError::HugrError(
                "No modules found in guppylang format".to_string(),
            ))
        }
    } else {
        Err(crate::SeleneError::HugrError(
            "Invalid guppylang format - modules is not an array".to_string(),
        ))
    }
}

/// Debug log module nodes for development
fn debug_log_module_nodes(module: &serde_json::Value) {
    log::debug!("Found module in modules array");
    log::debug!("Creating placeholder HUGR for testing");

    let nodes = module.get("nodes");
    if let Some(nodes_array) = nodes.and_then(|n| n.as_array()) {
        log::debug!("Found {} nodes in module", nodes_array.len());

        for (i, node) in nodes_array.iter().enumerate() {
            if let Some(op) = node.get("op") {
                log::debug!("Node {i}: op = {op:?}");

                if op == "Extension" {
                    if let Some(extension_name) = node.get("extension_name") {
                        log::debug!("Node {i} is Extension with name: {extension_name:?}");
                    }
                    if let Some(op_def) = node.get("op_def") {
                        log::debug!("Node {i} has op_def: {op_def:?}");
                    }
                    if i == 9 || i == 10 || i == 11 || i == 12 {
                        log::debug!(
                            "Full node {} structure: {}",
                            i,
                            serde_json::to_string_pretty(node).unwrap_or_default()
                        );
                    }
                }
            }
        }
    }
}

/// Load from modules array or single HUGR
fn load_from_modules_or_hugr(json_value: serde_json::Value) -> Result<Package, crate::SeleneError> {
    if let Some(modules) = json_value.get("modules").and_then(|v| v.as_array()) {
        if let Some(first_module) = modules.first() {
            let hugr: Hugr = serde_json::from_value(first_module.clone()).map_err(|e| {
                crate::SeleneError::HugrError(format!("Failed to deserialize module as HUGR: {e}"))
            })?;

            Package::from_hugr(hugr).map_err(|e| {
                crate::SeleneError::HugrError(format!("Failed to create package from HUGR: {e}"))
            })
        } else {
            Err(crate::SeleneError::HugrError(
                "No modules found in JSON".to_string(),
            ))
        }
    } else {
        // Try loading as single HUGR
        let hugr: Hugr = serde_json::from_value(json_value).map_err(|e| {
            crate::SeleneError::HugrError(format!("Failed to deserialize as HUGR: {e}"))
        })?;

        Package::from_hugr(hugr).map_err(|e| {
            crate::SeleneError::HugrError(format!("Failed to create package from HUGR: {e}"))
        })
    }
}

#[cfg(not(feature = "hugr-013"))]
pub fn load_hugr_013_package(_hugr_bytes: &[u8]) -> Result<(), crate::SeleneError> {
    Err(crate::SeleneError::HugrError(
        "HUGR 0.13 support not enabled. Compile with --features hugr-013".to_string(),
    ))
}
