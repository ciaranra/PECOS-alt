/*!
HUGR to QIR Compiler Module

This module provides functionality to compile HUGR (Hierarchical Unified Graph Representation)
files to LLVM IR/QIR for execution on PECOS.

The compiler bridges the gap between quantum programs written in languages like Guppy
(which compile to HUGR) and the PECOS quantum execution infrastructure.

Based on the working implementation from quantum-compilation-examples.
*/

#[cfg(feature = "hugr-llvm-pipeline")]
use hugr_core::package::Package;
#[cfg(feature = "hugr-llvm-pipeline")]
use hugr_core::{Hugr, std_extensions};
#[cfg(feature = "hugr-llvm-pipeline")]
use hugr_llvm::emit::EmitHugr;
#[cfg(feature = "hugr-llvm-pipeline")]
use hugr_llvm::extension::{int::IntCodegenExtension, prelude::DefaultPreludeCodegen};
#[cfg(feature = "hugr-llvm-pipeline")]
use hugr_llvm::inkwell::context::Context;
#[cfg(feature = "hugr-llvm-pipeline")]
use hugr_llvm::utils::fat::FatExt;
#[cfg(feature = "hugr-llvm-pipeline")]
use log::{debug, info};
#[cfg(feature = "hugr-llvm-pipeline")]
use pecos_core::errors::PecosError;
#[cfg(feature = "hugr-llvm-pipeline")]
use std::fs;
#[cfg(feature = "hugr-llvm-pipeline")]
use std::path::{Path, PathBuf};
#[cfg(feature = "hugr-llvm-pipeline")]
use std::rc::Rc;

#[cfg(feature = "hugr-llvm-pipeline")]
use super::result_extractor::ResultNameExtractor;
// Removed simple fallback - we should fix the actual issues instead
#[cfg(feature = "hugr-llvm-pipeline")]
use super::extensions::tket2_bool_extension::Tket2BoolExtension;
#[cfg(feature = "hugr-llvm-pipeline")]
use super::extensions::tket2_rotation_extension::Tket2RotationExtension;
#[cfg(feature = "hugr-llvm-pipeline")]
use super::generators::standard_qir_generator::StandardQirExtension;
// Version translator no longer needed - Guppy 0.20.0 and PECOS use same HUGR version

// Imports for non-hugr builds
#[cfg(not(feature = "hugr-llvm-pipeline"))]
use pecos_core::errors::PecosError;
#[cfg(not(feature = "hugr-llvm-pipeline"))]
use std::path::{Path, PathBuf};

/// Configuration for HUGR compilation
#[derive(Debug, Clone, Default)]
pub struct HugrCompilerConfig {
    /// Output file path for the generated LLVM IR
    pub output_path: Option<PathBuf>,
    /// Whether to include debug information in the output
    pub debug_info: bool,
}

/// HUGR to QIR compiler
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

    #[cfg(feature = "hugr-llvm-pipeline")]
    /// Compile a HUGR file to LLVM IR/QIR
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file to compile
    ///
    /// # Returns
    /// Path to the generated LLVM IR file
    ///
    /// # Errors
    /// Returns `PecosError::Compilation` if compilation fails
    pub fn compile_hugr<P: AsRef<Path>>(&self, hugr_path: P) -> Result<PathBuf, PecosError> {
        let hugr_path = hugr_path.as_ref();
        info!("Compiling HUGR file: {}", hugr_path.display());

        // Load HUGR from file
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

    #[cfg(feature = "hugr-llvm-pipeline")]
    /// Compile HUGR bytes to LLVM IR
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
        // First, try to parse the HUGR to check if we can use simple fallback
        let json_start = hugr_bytes
            .iter()
            .position(|&b| b == b'{')
            .ok_or_else(|| PecosError::Processing("HUGR doesn't contain JSON data".to_string()))?;

        let json_bytes = &hugr_bytes[json_start..];
        let json_str = std::str::from_utf8(json_bytes)
            .map_err(|e| PecosError::with_context(e, "Invalid UTF-8 in HUGR JSON"))?;
        serde_json::from_str::<serde_json::Value>(json_str)
            .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR JSON"))?;

        // No fallbacks - we'll fix the actual compilation issues
        debug!("Proceeding with full HUGR compilation");

        // Since both Guppy and PECOS use hugr 0.20.1, no translation needed
        debug!("Using HUGR directly without version translation");
        let transformed_bytes = hugr_bytes.to_vec();

        // Fix duplicate function names in HUGR
        let transformed_bytes = fix_duplicate_functions(&transformed_bytes)?;

        // Load HUGR package with transformed types
        let reader = std::io::Cursor::new(transformed_bytes.clone());
        let mut package = match Package::load(reader, Some(&std_extensions::std_reg())) {
            Ok(pkg) => pkg,
            Err(e) => {
                // Log the error details
                let err_str = e.to_string();
                if err_str.contains("missing field") {
                    // Try to debug what's happening
                    let json_start = transformed_bytes
                        .iter()
                        .position(|&b| b == b'{')
                        .unwrap_or(0);
                    let json_bytes = &transformed_bytes[json_start..];
                    if let Ok(json_str) = std::str::from_utf8(json_bytes) {
                        debug!(
                            "Failed HUGR JSON preview: {}",
                            &json_str[..json_str.len().min(500)]
                        );
                    }
                }
                return Err(PecosError::with_context(e, "Failed to parse HUGR"));
            }
        };

        let hugr = std::mem::take(&mut package.modules[0]);

        debug!("Loaded HUGR successfully");

        // Extract result names from the HUGR
        let result_names = ResultNameExtractor::extract_result_names(&hugr)
            .map_err(|e| PecosError::with_context(e, "Failed to extract result names"))?;

        if !result_names.is_empty() {
            debug!(
                "Extracted {} result name mappings from HUGR",
                result_names.len()
            );
        }

        // Create LLVM context and module
        let context = Context::create();
        let module = context.create_module("quantum_module");

        // Create extensions with appropriate QIR quantum support based on naming convention
        let mut builder = hugr_llvm::CodegenExtsBuilder::<Hugr>::default();

        // Add our custom extensions FIRST (before standard extensions)
        // This ensures our tket2.bool handler takes precedence
        builder = builder.add_extension(Tket2BoolExtension::new());
        builder = builder.add_extension(Tket2RotationExtension::new());

        // Use HUGR-style format with integer types
        builder = builder.add_extension(StandardQirExtension::new(result_names));

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
        let emit_hugr = EmitHugr::new(&context, module, Rc::new(namer), Rc::new(extensions));

        // Emit module
        let root = hugr
            .fat_root()
            .ok_or_else(|| PecosError::Feature("HUGR root not available".to_string()))?;

        let llvm_module = emit_hugr
            .emit_module(root)
            .map_err(PecosError::from)?
            .finish();

        // Add EntryPoint attributes to user-defined functions
        // Note: Function renaming to "main" will be handled in post-processing
        for func in llvm_module.get_functions() {
            if func.count_basic_blocks() > 0 {
                // Has a body (not just a declaration)
                let name = func.get_name().to_str().unwrap_or("");
                if !name.starts_with("llvm.") && !name.starts_with("__") {
                    // Add EntryPoint attribute to user functions
                    func.add_attribute(
                        hugr_llvm::inkwell::attributes::AttributeLoc::Function,
                        context.create_string_attribute("EntryPoint", ""),
                    );
                    debug!("Marked function '{}' as EntryPoint", name);
                }
            }
        }

        // Generate LLVM IR string
        let llvm_ir = llvm_module.to_string();

        // Fix entry point signature for HUGR convention
        let standard_qir = fix_entry_point_signature(&llvm_ir);

        // Keep native HUGR functions - no conversion needed
        debug!("Using native HUGR convention functions");

        // Write to output file
        fs::write(output_path, standard_qir).map_err(|e| {
            PecosError::with_context(
                e,
                format!("Failed to write LLVM IR to {}", output_path.display()),
            )
        })?;

        info!("Generated LLVM IR: {}", output_path.display());
        Ok(output_path.to_path_buf())
    }

    #[cfg(feature = "hugr-llvm-pipeline")]
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

        let standard_qir = fs::read_to_string(temp_file.path())
            .map_err(|e| PecosError::with_context(e, "Failed to read generated standard QIR"))?;

        Ok(standard_qir)
    }

    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    /// Compile a HUGR file to LLVM IR/QIR (disabled when hugr feature is not enabled)
    pub fn compile_hugr<P: AsRef<Path>>(&self, _hugr_path: P) -> Result<PathBuf, PecosError> {
        use std::io::{Error, ErrorKind};
        Err(PecosError::with_context(
            Error::new(ErrorKind::Unsupported, "HUGR support not compiled in"),
            "Enable 'hugr' feature to use HUGR compilation",
        ))
    }
}

#[cfg(feature = "hugr-llvm-pipeline")]
/// Fix duplicate function names in HUGR JSON
fn fix_duplicate_functions(hugr_bytes: &[u8]) -> Result<Vec<u8>, PecosError> {
    // Find JSON start
    let json_start = hugr_bytes.iter().position(|&b| b == b'{').unwrap_or(0);
    let prefix = &hugr_bytes[..json_start];
    let json_bytes = &hugr_bytes[json_start..];

    // Parse JSON
    let json_str = std::str::from_utf8(json_bytes)
        .map_err(|e| PecosError::Generic(format!("Invalid UTF-8 in HUGR: {e}")))?;
    let mut json_data: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| PecosError::Generic(format!("Failed to parse HUGR JSON: {e}")))?;

    // Track seen function names and rename duplicates
    let mut seen_functions = std::collections::HashSet::new();
    let mut duplicate_count = 0;

    if let Some(modules) = json_data.get_mut("modules").and_then(|m| m.as_array_mut()) {
        for module in modules {
            if let Some(nodes) = module.get_mut("nodes").and_then(|n| n.as_array_mut()) {
                for node in nodes {
                    if let Some(op) = node.get("op").and_then(|o| o.as_str()) {
                        if op == "FuncDefn" {
                            if let Some(name) = node.get_mut("name").and_then(|n| n.as_str()) {
                                if !seen_functions.insert(name.to_string()) {
                                    // Duplicate found
                                    duplicate_count += 1;
                                    let new_name = format!("{name}_duplicate{duplicate_count}");
                                    debug!(
                                        "Renaming duplicate function '{}' to '{}'",
                                        name, new_name
                                    );
                                    *node.get_mut("name").unwrap() =
                                        serde_json::Value::String(new_name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Convert back to bytes
    let fixed_json = serde_json::to_string(&json_data)
        .map_err(|e| PecosError::Generic(format!("Failed to serialize fixed HUGR: {e}")))?;

    let mut result = prefix.to_vec();
    result.extend_from_slice(fixed_json.as_bytes());

    Ok(result)
}

/// Fix entry point function signature for HUGR compatibility
fn fix_entry_point_signature(llvm_ir: &str) -> String {
    // Entry points need void return type to work with the runtime
    let lines: Vec<&str> = llvm_ir.lines().collect();
    let mut result = String::new();
    let mut found_entry_point = false;
    let mut attribute_number = "#0";

    for line in lines {
        if line.contains("define i1 @")
            || line.contains("define i16 @")
            || line.contains("define i32 @")
            || line.contains("define void @")
        {
            // Check if this is a user-defined function (entry point candidate)
            if let Some(func_name_start) = line.find('@') {
                let func_name_end =
                    line[func_name_start + 1..].find('(').unwrap_or(0) + func_name_start + 1;
                let func_name = &line[func_name_start + 1..func_name_end];

                // Skip LLVM intrinsics and runtime functions
                if !func_name.starts_with("llvm.") && !func_name.starts_with("__") {
                    found_entry_point = true;
                    // Check if line already has an attribute
                    if line.contains(" #") {
                        // Extract existing attribute number
                        if let Some(attr_start) = line.rfind(" #") {
                            let attr_end = line[attr_start + 2..]
                                .find(|c: char| !c.is_numeric())
                                .unwrap_or(line.len() - attr_start - 2)
                                + attr_start
                                + 2;
                            attribute_number = &line[attr_start + 1..attr_end];
                        }
                    } else {
                        // Add #0 attribute to the function definition
                        let insertion_point = line.rfind('{').unwrap_or(line.len() - 1);
                        let modified_line =
                            format!("{} #0 {{", &line[..insertion_point].trim_end());
                        result.push_str(&modified_line);
                        result.push('\n');
                        continue;
                    }

                    // Change return type to void if needed
                    if line.contains("define i1 @")
                        || line.contains("define i16 @")
                        || line.contains("define i32 @")
                    {
                        let modified_line = line
                            .replace("define i1 @", "define void @")
                            .replace("define i16 @", "define void @")
                            .replace("define i32 @", "define void @");
                        result.push_str(&modified_line);
                        result.push('\n');
                        continue;
                    }
                }
            }
        } else if line.trim().starts_with("ret i1 ")
            || line.trim().starts_with("ret i16 ")
            || line.trim().starts_with("ret i32 ")
        {
            // Replace the return statement with just "ret void"
            result.push_str("  ret void");
            result.push('\n');
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    // Add the EntryPoint attribute definition if we found an entry point
    if found_entry_point && !llvm_ir.contains("attributes #0 = {") {
        use std::fmt::Write;
        write!(
            result,
            "\nattributes {attribute_number} = {{ \"EntryPoint\" }}\n"
        )
        .expect("Writing to String should never fail");
    }

    result
}

impl Default for HugrCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to compile a HUGR file to QIR
///
/// # Arguments
/// * `hugr_path` - Path to the HUGR file
/// * `output_path` - Optional output path for the QIR file
///
/// # Returns
/// Path to the generated QIR file
///
/// # Errors
/// Returns `PecosError` if:
/// - The HUGR file cannot be read
/// - HUGR parsing fails
/// - Compilation fails
/// - File writing fails
pub fn compile_hugr_to_qir<P: AsRef<Path>, Q: Into<PathBuf>>(
    hugr_path: P,
    output_path: Option<Q>,
) -> Result<PathBuf, PecosError> {
    let mut compiler = HugrCompiler::new();

    if let Some(output) = output_path {
        compiler = compiler.with_output_path(output);
    }

    compiler.compile_hugr(hugr_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    use tempfile::NamedTempFile;

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

    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    #[test]
    fn test_hugr_compilation_without_feature() {
        let temp_file = NamedTempFile::new().unwrap();
        let compiler = HugrCompiler::new();

        let result = compiler.compile_hugr(temp_file.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HUGR support not compiled")
        );
    }
}
