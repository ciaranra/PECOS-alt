/*!
PECOS PMIR (PECOS MLIR) - Alternative compilation pipeline via MLIR

This crate provides an alternative compilation path from HUGR to LLVM IR via:
1. Pest parsing of HUGR JSON to PAST (PECOS AST) in RON format
2. Lowering from PAST to PMIR (PECOS Middle-level IR) expressed as MLIR text
3. Using MLIR tools to compile PMIR to LLVM IR

It also supports direct execution of PMIR without LLVM compilation.
*/

use pecos_core::errors::PecosError;
use std::path::Path;

pub mod angle_resolver;
pub mod ast;
pub mod hugr_parser;
pub mod mlir_lowering;
pub mod mlir_toolchain;
#[cfg(feature = "python-bindings")]
pub mod python_api;

// Re-export key types for convenience
pub use ast::PastModule;
pub use mlir_lowering::MlirModule;

/// Configuration for the PMIR (PECOS Middle-level IR) compilation pipeline
#[derive(Debug, Clone)]
pub struct PmirConfig {
    /// Enable debug output of intermediate representations
    pub debug_output: bool,
    /// Optimization level (0-3)
    pub optimization_level: u8,
    /// Target triple for LLVM
    pub target_triple: Option<String>,
}

impl Default for PmirConfig {
    fn default() -> Self {
        Self {
            debug_output: false,
            optimization_level: 2,
            target_triple: None,
        }
    }
}

/// Convert binary HUGR format to JSON by stripping the header
///
/// The binary HUGR format consists of a 10-byte header followed by JSON data.
/// This function strips the header and returns the JSON string.
///
/// # Errors
///
/// Returns `PecosError` if the data is not valid HUGR format
pub fn binary_hugr_to_json(hugr_bytes: &[u8]) -> Result<String, PecosError> {
    // Check if it's binary HUGR format (starts with "HUGR")
    if hugr_bytes.len() >= 10 && &hugr_bytes[0..4] == b"HUGR" {
        // Skip the 10-byte header and decode the JSON
        let json_bytes = &hugr_bytes[10..];
        String::from_utf8(json_bytes.to_vec())
            .map_err(|e| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: format!("Invalid UTF-8 in HUGR data: {}", e),
            })
    } else if hugr_bytes.starts_with(b"{") {
        // Already JSON format
        String::from_utf8(hugr_bytes.to_vec())
            .map_err(|e| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: format!("Invalid UTF-8 in JSON data: {}", e),
            })
    } else {
        Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Data does not appear to be HUGR format (neither binary nor JSON)".to_string(),
        })
    }
}

/// Main entry point for HUGR → PAST → MLIR → LLVM compilation
///
/// Accepts HUGR in JSON format. For binary HUGR format, use `compile_hugr_bytes_via_pmir`.
///
/// # Errors
///
/// Returns `PecosError` if any step in the compilation pipeline fails
pub fn compile_hugr_via_pmir(hugr_json: &str, config: &PmirConfig) -> Result<String, PecosError> {
    // Step 1: Parse HUGR JSON to PAST using Pest
    let past = hugr_parser::parse_hugr_to_past(hugr_json)?;

    if config.debug_output {
        match past.to_ron_string() {
            Ok(ron_str) => log::debug!("PAST representation:\n{}", ron_str),
            Err(e) => log::warn!("Failed to serialize PAST to RON: {:?}", e),
        }
    }

    // Step 2: Lower PAST to PMIR (PECOS Middle-level IR) as MLIR text
    let mlir_module = mlir_lowering::lower_past_to_pmir(&past, config)?;
    let mlir_text = mlir_module.to_string();

    if config.debug_output {
        log::debug!("PMIR (as MLIR text):\n{}", mlir_text);
    }

    // Step 3: Use MLIR toolchain to generate LLVM IR
    let toolchain_config = mlir_toolchain::MlirToolchainConfig {
        keep_intermediate_files: config.debug_output,
        ..Default::default()
    };

    // Check if MLIR tools are available
    mlir_toolchain::check_mlir_tools(&toolchain_config)?;

    let llvm_ir = mlir_toolchain::mlir_to_llvm_ir(&mlir_text, &toolchain_config)?;

    // Post-process LLVM IR to add EntryPoint attribute (needed for PECOS runtime)
    let fixed_llvm_ir = fix_entry_point_attribute(&llvm_ir);

    Ok(fixed_llvm_ir)
}

/// Compile binary HUGR format via PMIR
///
/// This is a convenience function that handles the binary HUGR format
/// by stripping the header and calling the JSON-based compiler.
///
/// # Errors
///
/// Returns `PecosError` if any step in the compilation pipeline fails
pub fn compile_hugr_bytes_via_pmir(hugr_bytes: &[u8], config: &PmirConfig) -> Result<String, PecosError> {
    let hugr_json = binary_hugr_to_json(hugr_bytes)?;
    compile_hugr_via_pmir(&hugr_json, config)
}

/// Convert HUGR JSON to PAST RON representation
///
/// # Errors
///
/// Returns `PecosError` if parsing or serialization fails
pub fn hugr_to_past_ron(hugr_json: &str) -> Result<String, PecosError> {
    let past = hugr_parser::parse_hugr_to_past(hugr_json)?;
    past.to_ron_string().map_err(|e| PecosError::ParseSyntax {
        language: "RON".to_string(),
        message: format!("Failed to serialize PAST to RON: {e:?}"),
    })
}

/// Convert HUGR JSON to PMIR (MLIR text format)
///
/// # Errors
///
/// Returns `PecosError` if parsing or lowering fails
pub fn hugr_to_pmir_mlir(hugr_json: &str, config: &PmirConfig) -> Result<String, PecosError> {
    let past = hugr_parser::parse_hugr_to_past(hugr_json)?;
    let mlir_module = mlir_lowering::lower_past_to_pmir(&past, config)?;
    Ok(mlir_module.to_string())
}

/// Convert PAST RON to PMIR (MLIR text format)
///
/// # Errors
///
/// Returns `PecosError` if deserialization or lowering fails
pub fn past_ron_to_pmir_mlir(past_ron: &str, config: &PmirConfig) -> Result<String, PecosError> {
    let past: ast::PastModule = ron::from_str(past_ron).map_err(|e| PecosError::ParseSyntax {
        language: "RON".to_string(),
        message: format!("Failed to deserialize PAST from RON: {e:?}"),
    })?;

    let mlir_module = mlir_lowering::lower_past_to_pmir(&past, config)?;
    Ok(mlir_module.to_string())
}

/// Compile HUGR from file using PMIR pipeline
///
/// # Errors
///
/// Returns `PecosError` if:
/// - Failed to read input file
/// - Compilation fails
/// - Failed to write output file
pub fn compile_hugr_file_via_pmir(
    input_path: &Path,
    output_path: &Path,
    config: &PmirConfig,
) -> Result<(), PecosError> {
    let hugr_json = std::fs::read_to_string(input_path).map_err(PecosError::IO)?;

    let llvm_ir = compile_hugr_via_pmir(&hugr_json, config)?;

    std::fs::write(output_path, llvm_ir).map_err(PecosError::IO)?;

    Ok(())
}

/// Fix LLVM IR to add EntryPoint attribute for PECOS runtime compatibility
///
/// This function ensures that the main function has the EntryPoint attribute
/// needed by the PECOS LLVM engine, similar to how HUGR-LLVM works.
fn fix_entry_point_attribute(llvm_ir: &str) -> String {
    // Find the first function definition (should be @main) and add EntryPoint attribute
    let mut result = String::new();
    let mut found_main_function = false;
    let mut in_attributes_section = false;
    
    for line in llvm_ir.lines() {
        if !found_main_function && line.starts_with("define ") && line.contains("@main") {
            // This is the main function - mark it as entry point
            // Check if it already has an attribute
            if line.contains(" #") {
                // Function already has attributes, just note we found it
                found_main_function = true;
                result.push_str(line);
            } else {
                // Need to insert #0 attribute at the correct position
                // LLVM syntax: attributes come before debug metadata
                // "define { i32, i32 } @main() #0 !dbg !3 {"
                if let Some(dbg_pos) = line.find(" !dbg ") {
                    // Insert #0 before the debug metadata
                    result.push_str(&line[..dbg_pos]);
                    result.push_str(" #0");
                    result.push_str(&line[dbg_pos..]);
                    found_main_function = true;
                } else if let Some(pos) = line.rfind(" {") {
                    // No debug metadata, insert #0 before opening brace
                    result.push_str(&line[..pos]);
                    result.push_str(" #0");
                    result.push_str(&line[pos..]);
                    found_main_function = true;
                } else {
                    // No opening brace on this line (shouldn't happen)
                    result.push_str(line);
                }
            }
        } else if line.starts_with("attributes #") {
            in_attributes_section = true;
            result.push_str(line);
        } else if in_attributes_section && line.trim().is_empty() {
            // End of attributes section - add our EntryPoint attribute if needed
            if found_main_function && !llvm_ir.contains("\"EntryPoint\"") {
                result.push_str("\nattributes #0 = { \"EntryPoint\" }");
            }
            in_attributes_section = false;
            result.push_str(line);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    
    // If we didn't find an attributes section, add it at the end
    if found_main_function && !llvm_ir.contains("\"EntryPoint\"") && !llvm_ir.contains("attributes #") {
        result.push_str("\nattributes #0 = { \"EntryPoint\" }\n");
    }
    
    result
}

/// Direct execution of PMIR without LLVM compilation (future feature)
#[cfg(feature = "direct-execution")]
pub mod direct_execution {
    use super::*;
    #[allow(unused_imports)]
    use pecos_engines::prelude::*;

    /// Execute PMIR directly using PECOS simulators
    pub fn execute_pmir_directly(
        _pmir_module: &MlirModule,
        _config: &PmirConfig,
    ) -> Result<(), PecosError> {
        // TODO: Implement direct PMIR execution
        // This would involve:
        // 1. Interpreting PMIR operations directly
        // 2. Managing quantum state with PECOS simulators
        // 3. Handling classical control flow
        // 4. Returning results

        todo!("Direct PMIR execution not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PmirConfig::default();
        assert_eq!(config.optimization_level, 2);
        assert!(!config.debug_output);
    }
}
