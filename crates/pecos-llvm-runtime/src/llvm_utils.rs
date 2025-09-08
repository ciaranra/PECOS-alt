use log::debug;
use pecos_core::errors::PecosError;
use std::fs;
use std::path::Path;

/// Find the entry point function in a QIR/LLVM file
///
/// Parses the LLVM IR to find functions with the "`EntryPoint`" attribute.
///
/// # Errors
/// Returns an error if the LLVM file cannot be parsed or accessed.
pub(crate) fn find_entry_point(llvm_file: &Path) -> Result<Option<String>, PecosError> {
    // Read the LLVM IR file
    let content = fs::read_to_string(llvm_file).map_err(PecosError::IO)?;

    // Skip QIR format validation for entry point detection - just find the function
    // The validation will be done separately if needed

    // Look for attribute definitions like: attributes #0 = { "EntryPoint" }
    let mut entry_point_attrs = Vec::new();
    for line in content.lines() {
        if line.starts_with("attributes #") && line.contains("\"EntryPoint\"") {
            // Extract attribute number
            if let Some(attr_num) = line.split('#').nth(1).and_then(|s| s.split(' ').next()) {
                entry_point_attrs.push(format!("#{attr_num}"));
                debug!("Found EntryPoint attribute: #{attr_num}");
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
                    if let Some(func_start) = line.find('@')
                        && let Some(func_end) = line[func_start + 1..].find('(')
                    {
                        let func_name = &line[func_start + 1..func_start + 1 + func_end];
                        debug!("Found entry point function: {func_name}");
                        return Ok(Some(func_name.to_string()));
                    }
                }
            }
        }
    }

    // No function with EntryPoint attribute found
    debug!(
        "No function with EntryPoint attribute found in {}",
        llvm_file.display()
    );
    Ok(None)
}
