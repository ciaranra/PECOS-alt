/*!
PMIR (PECOS Middle-level IR) Compilation Pipeline

This module provides an alternative compilation path from HUGR to LLVM IR via:
1. Pest parsing of HUGR JSON to PAST (PECOS AST) in RON format
2. Lowering from PAST to PMIR (PECOS Middle-level IR) expressed as MLIR text
3. Using MLIR tools to compile PMIR to LLVM IR

This runs alongside the existing HUGR→LLVM pipeline for comparison and development.
*/

use pecos_core::errors::PecosError;
use std::path::Path;

pub mod ast;
pub mod hugr_parser;
pub mod mlir_lowering;
pub mod mlir_toolchain;

// Python API module removed - now in pecos-rslib

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

/// Main entry point for HUGR → PAST → MLIR → LLVM compilation
pub fn compile_hugr_via_pmir(
    hugr_json: &str,
    config: &PmirConfig,
) -> Result<String, PecosError> {
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
    
    Ok(llvm_ir)
}


/// Convert HUGR JSON to PAST RON representation
pub fn hugr_to_past_ron(hugr_json: &str) -> Result<String, PecosError> {
    let past = hugr_parser::parse_hugr_to_past(hugr_json)?;
    past.to_ron_string()
        .map_err(|e| PecosError::ParseSyntax {
            language: "RON".to_string(),
            message: format!("Failed to serialize PAST to RON: {:?}", e),
        })
}

/// Convert HUGR JSON to PMIR (MLIR text format)
pub fn hugr_to_pmir_mlir(hugr_json: &str, config: &PmirConfig) -> Result<String, PecosError> {
    let past = hugr_parser::parse_hugr_to_past(hugr_json)?;
    let mlir_module = mlir_lowering::lower_past_to_pmir(&past, config)?;
    Ok(mlir_module.to_string())
}

/// Convert PAST RON to PMIR (MLIR text format)
pub fn past_ron_to_pmir_mlir(past_ron: &str, config: &PmirConfig) -> Result<String, PecosError> {
    let past: ast::PastModule = ron::from_str(past_ron)
        .map_err(|e| PecosError::ParseSyntax {
            language: "RON".to_string(),
            message: format!("Failed to deserialize PAST from RON: {:?}", e),
        })?;
    
    let mlir_module = mlir_lowering::lower_past_to_pmir(&past, config)?;
    Ok(mlir_module.to_string())
}

/// Compile HUGR from file using PMIR pipeline
pub fn compile_hugr_file_via_pmir(
    input_path: &Path,
    output_path: &Path,
    config: &PmirConfig,
) -> Result<(), PecosError> {
    let hugr_json = std::fs::read_to_string(input_path)
        .map_err(|e| PecosError::IO(e))?;
    
    let llvm_ir = compile_hugr_via_pmir(&hugr_json, config)?;
    
    std::fs::write(output_path, llvm_ir)
        .map_err(|e| PecosError::IO(e))?;
    
    Ok(())
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