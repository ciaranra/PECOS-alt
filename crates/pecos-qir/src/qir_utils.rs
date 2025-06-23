use pecos_core::errors::PecosError;
use std::path::Path;
use std::fs;
use log::{debug, warn};
use crate::error_handling::validate_qir_for_runtime_issues;

/// Validate QIR format for compatibility with PECOS runtime
///
/// This function checks for common incompatibilities that could cause runtime aborts
/// and also performs runtime issue detection.
///
/// # Arguments
///
/// * `content` - The QIR file content as a string
///
/// # Returns
///
/// * `Ok(())` - If the QIR format is compatible
/// * `Err(error)` - If there are format incompatibilities
pub fn validate_qir_format(content: &str) -> Result<(), PecosError> {
    // Check if this is HUGR format - if so, it's compatible with PECOS runtime
    let is_hugr_format = content.contains("@__quantum__qis__h__body(i64") || 
                         content.contains("@__quantum__qis__m__body(i64");
    
    if is_hugr_format {
        // HUGR format is the native format for PECOS runtime
        debug!("QIR uses HUGR format - compatible with PECOS runtime");
        return Ok(());
    }
    let mut issues = Vec::new();
    
    // Check for non-standard measurement signatures
    if content.contains("call i32 @__quantum__qis__m__body") {
        issues.push("Non-standard measurement signature: '__quantum__qis__m__body' should return void, not i32");
    }
    
    // Check for integer-based qubit operations (should use pointers in standard QIR)
    if content.contains("@__quantum__qis__h__body(i64") || content.contains("@__quantum__qis__m__body(i64") {
        issues.push("Integer-based qubit operations found - standard QIR should use opaque pointer types (%Qubit*, %Result*)");
    }
    
    // Check for missing opaque types (standard QIR should declare these)
    let has_qubit_type = content.contains("%Qubit = type opaque");
    let has_result_type = content.contains("%Result = type opaque");
    
    if !has_qubit_type || !has_result_type {
        if content.contains("__quantum__qis__") {
            issues.push("Missing standard QIR opaque types (%Qubit and %Result should be declared as opaque)");
        }
    }
    
    // Check for function return type compatibility
    if content.contains("define i1 @") && content.contains("EntryPoint") {
        issues.push("Entry point function returns 'i1' but should return 'void' for standard QIR compatibility");
    }
    
    // Perform additional runtime issue detection
    match validate_qir_for_runtime_issues(content) {
        Ok(warnings) => {
            if !warnings.is_empty() {
                warn!("QIR runtime warnings detected:");
                for warning in &warnings {
                    warn!("  - {}", warning);
                }
                // Add warnings to issues for reporting (as non-fatal)
                for warning in warnings {
                    debug!("Runtime Warning: {}", warning);
                }
            }
        }
        Err(e) => {
            warn!("Failed to perform runtime issue detection: {}", e);
        }
    }

    if !issues.is_empty() {
        let error_msg = format!(
            "QIR format compatibility issues detected:\n{}\n\nThis QIR appears to be generated with HUGR-specific conventions that are incompatible with standard QIR runtime. \
            The runtime expects standard QIR format with opaque pointer types and void-returning entry functions.",
            issues.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n")
        );
        return Err(PecosError::Input(error_msg));
    }
    
    Ok(())
}

/// Find the entry point function in a QIR/LLVM file
///
/// Parses the LLVM IR to find functions with the "EntryPoint" attribute.
///
/// # Errors
/// Returns an error if the LLVM file cannot be parsed or accessed.
pub fn find_entry_point(llvm_file: &Path) -> Result<Option<String>, PecosError> {
    // Read the LLVM IR file
    let content = fs::read_to_string(llvm_file)
        .map_err(|e| PecosError::IO(e))?;
    
    // Skip QIR format validation for entry point detection - just find the function
    // The validation will be done separately if needed
    
    // Look for attribute definitions like: attributes #0 = { "EntryPoint" }
    let mut entry_point_attrs = Vec::new();
    for line in content.lines() {
        if line.starts_with("attributes #") && line.contains("\"EntryPoint\"") {
            // Extract attribute number
            if let Some(attr_num) = line.split('#').nth(1).and_then(|s| s.split(' ').next()) {
                entry_point_attrs.push(format!("#{}", attr_num));
                debug!("Found EntryPoint attribute: #{}", attr_num);
            }
        }
    }
    
    // Now look for functions that use these attributes
    for line in content.lines() {
        if line.starts_with("define ") {
            // Check if this function has any of the EntryPoint attributes
            for attr in &entry_point_attrs {
                if line.contains(attr) {
                    // Extract function name
                    if let Some(func_start) = line.find('@') {
                        if let Some(func_end) = line[func_start+1..].find('(') {
                            let func_name = &line[func_start+1..func_start+1+func_end];
                            debug!("Found entry point function: {}", func_name);
                            return Ok(Some(func_name.to_string()));
                        }
                    }
                }
            }
        }
    }
    
    // No function with EntryPoint attribute found
    debug!("No function with EntryPoint attribute found in {}", llvm_file.display());
    Ok(None)
}
