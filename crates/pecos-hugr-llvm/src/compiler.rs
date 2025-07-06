/*!
Pure HUGR to LLVM IR Compiler

This module provides functionality to compile HUGR (Hierarchical Unified Graph Representation)
files to LLVM IR. It contains no execution engine dependencies - only compilation logic.
*/

use hugr_core::package::Package;
use hugr_core::{Hugr, std_extensions};
use hugr_llvm::emit::EmitHugr;
use hugr_llvm::extension::{int::IntCodegenExtension, prelude::DefaultPreludeCodegen};
use hugr_llvm::inkwell::context::Context;
use hugr_llvm::utils::fat::FatExt;
use log::{debug, info, trace};
use pecos_core::errors::PecosError;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::extensions::tket2_bool_extension::Tket2BoolExtension;
use super::extensions::tket2_rotation_extension::Tket2RotationExtension;
use super::generators::standard_llvm_generator::StandardLlvmExtension;
use super::result_extractor::ResultNameExtractor;

/// Configuration for HUGR compilation
#[derive(Debug, Clone, Default)]
pub struct HugrCompilerConfig {
    /// Output file path for the generated LLVM IR
    pub output_path: Option<PathBuf>,
    /// Whether to include debug information in the output
    pub debug_info: bool,
}

/// Pure HUGR to LLVM IR compiler
pub struct HugrCompiler {
    config: HugrCompilerConfig,
}

impl HugrCompiler {
    /// Create a new HUGR compiler with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: HugrCompilerConfig::default(),
        }
    }

    /// Create a new HUGR compiler with custom configuration
    #[must_use]
    pub fn with_config(config: HugrCompilerConfig) -> Self {
        Self { config }
    }

    /// Set the output path for compiled LLVM IR
    #[must_use]
    pub fn with_output_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.output_path = Some(path.into());
        self
    }

    /// Enable or disable debug information
    #[must_use]
    pub fn with_debug_info(mut self, debug: bool) -> Self {
        self.config.debug_info = debug;
        self
    }

    /// Compile a HUGR file to LLVM IR
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file to compile
    ///
    /// # Returns
    /// Path to the generated LLVM IR file
    ///
    /// # Errors
    /// Returns `PecosError` if:
    /// - The HUGR file cannot be read
    /// - HUGR parsing fails
    /// - LLVM compilation fails
    /// - File I/O operations fail
    pub fn compile_hugr<P: AsRef<Path>>(&self, hugr_path: P) -> Result<PathBuf, PecosError> {
        let hugr_path = hugr_path.as_ref();
        debug!("HUGR: Compiling HUGR file: {hugr_path:?}");

        // Read HUGR file
        let hugr_bytes = fs::read(hugr_path).map_err(|e| {
            PecosError::with_context(
                e,
                format!("Failed to read HUGR file: {}", hugr_path.display()),
            )
        })?;

        // Determine output path
        let output_path = self
            .config
            .output_path
            .clone()
            .unwrap_or_else(|| hugr_path.with_extension("ll"));

        // Compile to LLVM IR
        self.compile_hugr_bytes(&hugr_bytes, &output_path)
    }

    /// Compile HUGR bytes to LLVM IR file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HUGR deserialization fails
    /// - LLVM compilation fails
    /// - File I/O operations fail
    pub fn compile_hugr_bytes(
        &self,
        hugr_bytes: &[u8],
        output_path: &Path,
    ) -> Result<PathBuf, PecosError> {
        debug!("HUGR: Compiling HUGR bytes to {}", output_path.display());

        // Fix duplicate function names before processing
        let fixed_hugr_bytes = fix_duplicate_functions(hugr_bytes)?;

        // Load HUGR package
        let reader = std::io::Cursor::new(fixed_hugr_bytes.clone());
        let mut hugr_package = Package::load(reader, Some(&std_extensions::std_reg()))
            .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR"))?;

        debug!(
            "HUGR: Parsed HUGR package with {} modules",
            hugr_package.modules.len()
        );

        if hugr_package.modules.is_empty() {
            return Err(PecosError::Input(
                "HUGR package contains no modules".to_string(),
            ));
        }

        // Get the main module (first module)
        let main_module = std::mem::take(&mut hugr_package.modules[0]);
        debug!("HUGR: Processing main module");

        // Create LLVM context and generate IR
        let context = Context::create();
        let llvm_ir = Self::generate_llvm_ir(&context, &main_module)?;

        // Write LLVM IR to file
        fs::write(output_path, llvm_ir).map_err(|e| {
            PecosError::with_context(
                e,
                format!("Failed to write LLVM IR to {}", output_path.display()),
            )
        })?;

        info!("HUGR: Generated LLVM IR: {}", output_path.display());
        Ok(output_path.to_path_buf())
    }

    /// Compile HUGR bytes to LLVM IR string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HUGR deserialization fails
    /// - LLVM compilation fails
    /// - Temporary file creation fails
    pub fn compile_hugr_bytes_to_string(&self, hugr_bytes: &[u8]) -> Result<String, PecosError> {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new()
            .map_err(|e| PecosError::with_context(e, "Failed to create temporary file"))?;

        self.compile_hugr_bytes(hugr_bytes, temp_file.path())?;

        let llvm_ir = fs::read_to_string(temp_file.path())
            .map_err(|e| PecosError::with_context(e, "Failed to read generated LLVM IR"))?;

        Ok(llvm_ir)
    }

    /// Generate LLVM IR from a HUGR module
    fn generate_llvm_ir(context: &Context, hugr: &Hugr) -> Result<String, PecosError> {
        debug!("HUGR: Starting LLVM IR generation");

        // Extract result names for proper variable naming
        let result_names = ResultNameExtractor::extract_result_names(hugr)?;
        debug!("HUGR: Extracted {} result names", result_names.len());

        // Create LLVM module
        let module = context.create_module("quantum_module");

        // Create extensions with appropriate QIR quantum support based on naming convention
        let mut builder = hugr_llvm::CodegenExtsBuilder::<Hugr>::default();

        // Add our custom extensions FIRST (before standard extensions)
        // This ensures our tket2.bool handler takes precedence
        builder = builder.add_extension(Tket2BoolExtension::new());
        builder = builder.add_extension(Tket2RotationExtension::new());

        // Use HUGR-style format with integer types
        builder = builder.add_extension(StandardLlvmExtension::new(result_names));

        // Add all standard extensions
        builder = builder.add_default_prelude_extensions();
        builder = builder.add_logic_extensions();

        // Add arithmetic extensions for int(6) and float64 support
        builder = builder.add_extension(IntCodegenExtension::new(DefaultPreludeCodegen));
        builder = builder.add_float_extensions();
        builder = builder.add_conversion_extensions();

        let extensions = builder.finish();

        // Create a namer that doesn't add prefixes for cleaner function names
        let namer = hugr_llvm::emit::Namer::new("_hugr_", false);

        // Create emitter
        let emit_hugr = EmitHugr::new(context, module, Rc::new(namer), Rc::new(extensions));

        // Emit module
        let root = hugr
            .fat_root()
            .ok_or_else(|| PecosError::Feature("HUGR root not available".to_string()))?;

        let llvm_module = emit_hugr
            .emit_module(root)
            .map_err(PecosError::from)?
            .finish();

        debug!("HUGR: Generated LLVM module");

        // Convert to string and fix entry point
        let llvm_ir_string = llvm_module.print_to_string().to_string();
        let fixed_llvm_ir = fix_entry_point_signature(&llvm_ir_string);

        trace!("HUGR: Generated LLVM IR:\n{fixed_llvm_ir}");
        debug!("HUGR: LLVM IR generation completed successfully");

        Ok(fixed_llvm_ir)
    }
}

impl Default for HugrCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Fix duplicate function names in HUGR JSON
fn fix_duplicate_functions(hugr_bytes: &[u8]) -> Result<Vec<u8>, PecosError> {
    use std::collections::HashSet;

    // Find JSON start
    let json_start = hugr_bytes.iter().position(|&b| b == b'{').unwrap_or(0);
    let prefix = &hugr_bytes[..json_start];
    let json_bytes = &hugr_bytes[json_start..];

    // Parse JSON
    let json_str = std::str::from_utf8(json_bytes)
        .map_err(|e| PecosError::with_context(e, "Invalid UTF-8 in HUGR data"))?;

    let mut json: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        PecosError::with_context(e, "Failed to parse HUGR JSON for duplicate fixing")
    })?;

    // Track seen function names
    let mut seen_names = HashSet::new();
    let mut name_counter = std::collections::HashMap::new();

    // Fix duplicate function names in all modules
    if let Some(modules) = json.get_mut("modules").and_then(|m| m.as_array_mut()) {
        for module in modules {
            if let Some(nodes) = module.get_mut("nodes").and_then(|n| n.as_array_mut()) {
                for node in nodes {
                    if let Some(op) = node.get("op").and_then(|o| o.as_str()) {
                        if op == "FuncDefn" || op == "FuncDecl" {
                            if let Some(name_value) = node.get("name") {
                                if let Some(name) = name_value.as_str() {
                                    let name_owned = name.to_string();
                                    if seen_names.contains(&name_owned) {
                                        // Generate a unique name
                                        let count =
                                            name_counter.entry(name_owned.clone()).or_insert(1);
                                        *count += 1;
                                        let new_name = format!("{name_owned}_{count}");
                                        debug!(
                                            "Renamed duplicate function '{name_owned}' to '{new_name}'"
                                        );
                                        node["name"] = serde_json::Value::String(new_name);
                                    } else {
                                        seen_names.insert(name_owned.clone());
                                        name_counter.insert(name_owned, 1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Serialize back to bytes
    let fixed_json = serde_json::to_string(&json)
        .map_err(|e| PecosError::with_context(e, "Failed to serialize fixed JSON"))?;

    let mut result = prefix.to_vec();
    result.extend_from_slice(fixed_json.as_bytes());

    Ok(result)
}

/// Fix entry point signature to work with PECOS runtime
fn fix_entry_point_signature(llvm_ir: &str) -> String {
    // Find the first function definition and add EntryPoint attribute
    let mut result = String::new();
    let mut found_first_function = false;
    let mut in_attributes_section = false;

    for line in llvm_ir.lines() {
        if !found_first_function && line.starts_with("define ") && line.contains("@_hugr_") {
            // This is the first HUGR function - mark it as entry point
            // Check if it already has an attribute
            if line.contains(" #") {
                // Function already has attributes, just note we found it
                found_first_function = true;
                result.push_str(line);
            } else if let Some(pos) = line.rfind(" {") {
                // Add #0 attribute before the opening brace
                result.push_str(&line[..pos]);
                result.push_str(" #0 {");
                found_first_function = true;
            } else {
                // Shouldn't happen but handle gracefully
                result.push_str(line);
            }
        } else if line.starts_with("attributes #") {
            in_attributes_section = true;
            result.push_str(line);
        } else if in_attributes_section && line.trim().is_empty() {
            // End of attributes section - add our EntryPoint attribute if needed
            if found_first_function && !llvm_ir.contains("\"EntryPoint\"") {
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
    if found_first_function
        && !llvm_ir.contains("\"EntryPoint\"")
        && !llvm_ir.contains("attributes #")
    {
        result.push_str("\nattributes #0 = { \"EntryPoint\" }\n");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hugr_compiler_creation() {
        let compiler = HugrCompiler::new();
        assert!(!compiler.config.debug_info);
    }

    #[test]
    fn test_hugr_compiler_configuration() {
        let compiler = HugrCompiler::new()
            .with_debug_info(true)
            .with_output_path("/tmp/test.ll");

        assert!(compiler.config.debug_info);
        assert_eq!(
            compiler.config.output_path,
            Some(PathBuf::from("/tmp/test.ll"))
        );
    }
}
