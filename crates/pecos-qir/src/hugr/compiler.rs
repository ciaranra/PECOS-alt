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
#[cfg(feature = "hugr-llvm-pipeline")]
use super::simple_llvm_fallback::{can_handle_simple, generate_simple_llvm};
#[cfg(feature = "hugr-llvm-pipeline")]
use super::standard_qir_generator::StandardQirExtension;
#[cfg(feature = "hugr-llvm-pipeline")]
use super::version_translator::translate_hugr_versions;

// Imports for non-hugr builds
#[cfg(not(feature = "hugr-llvm-pipeline"))]
use pecos_core::errors::PecosError;
#[cfg(not(feature = "hugr-llvm-pipeline"))]
use std::path::{Path, PathBuf};

/// Configuration for HUGR compilation
#[derive(Debug, Clone)]
pub struct HugrCompilerConfig {
    /// Output file path for the generated LLVM IR
    pub output_path: Option<PathBuf>,
    /// Whether to include debug information in the output
    pub debug_info: bool,
    /// Quantum operation naming convention to use
    pub quantum_naming: QuantumNamingConvention,
}

impl Default for HugrCompilerConfig {
    fn default() -> Self {
        Self {
            output_path: None,
            debug_info: false,
            quantum_naming: QuantumNamingConvention::StandardQir,
        }
    }
}

/// Quantum operation naming conventions
#[derive(Debug, Clone, PartialEq)]
pub enum QuantumNamingConvention {
    /// Standard QIR naming: __`quantum__qis__h__body`, etc.
    StandardQir,
    /// HUGR naming: __`hugr__quantum__h`, etc.
    Hugr,
    /// PECOS naming: custom mappings for PECOS runtime
    Pecos,
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

    /// Set the quantum operation naming convention
    #[must_use]
    pub fn with_quantum_naming(mut self, naming: QuantumNamingConvention) -> Self {
        self.config.quantum_naming = naming;
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
        let hugr_json: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR JSON"))?;

        // Check if we can use simple fallback
        if can_handle_simple(&hugr_json) {
            info!("Using simple LLVM fallback for basic Guppy function");
            let llvm_ir = generate_simple_llvm(&hugr_json)?;
            std::fs::write(output_path, llvm_ir).map_err(|e| {
                PecosError::with_context(
                    e,
                    format!("Failed to write LLVM IR to {}", output_path.display()),
                )
            })?;
            return Ok(output_path.to_path_buf());
        }

        // Otherwise, use normal compilation path
        debug!("Translating HUGR versions for compatibility");
        let transformed_bytes = translate_hugr_versions(hugr_bytes)?;

        // Load HUGR package with transformed types
        let reader = std::io::Cursor::new(transformed_bytes);
        let mut package = Package::load(reader, Some(&std_extensions::std_reg()))
            .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR"))?;

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

        // Create extensions with standard QIR quantum support
        let mut builder = hugr_llvm::CodegenExtsBuilder::<Hugr>::default();

        // Add all standard extensions
        builder = builder.add_default_prelude_extensions();
        builder = builder.add_logic_extensions();

        // Add our custom quantum extensions
        builder = builder.add_extension(StandardQirExtension::new(result_names));

        let extensions = builder.finish();

        // Create a namer that doesn't add prefixes for cleaner function names
        let namer = hugr_llvm::emit::Namer::new("", false);

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

        // Add standard QIR prologue (type definitions and function declarations)
        let standard_qir = add_standard_qir_prologue(&llvm_ir);

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
/// Add standard QIR prologue and rename functions to make the generated IR compatible with `QirEngine`
fn add_standard_qir_prologue(llvm_ir: &str) -> String {
    // PECOS QIR prologue with integer-based function signatures
    let prologue = r"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__x__body(i64)
declare void @__quantum__qis__y__body(i64)
declare void @__quantum__qis__z__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare void @__quantum__qis__cz__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

";

    // Process the LLVM IR line by line
    let lines: Vec<&str> = llvm_ir.lines().collect();
    let mut result = String::new();
    let mut prologue_added = false;
    let mut main_function_found = false;

    for line in lines {
        let trimmed = line.trim();

        // Add prologue before the first substantial line (not comments, not empty)
        if !prologue_added && !trimmed.is_empty() && !trimmed.starts_with(';') {
            result.push_str(prologue);
            prologue_added = true;
        }

        // Rename the first user-defined function to "main"
        if !main_function_found
            && line.starts_with("define ")
            && !line.contains("@llvm.")
            && !line.contains("@__")
        {
            // Extract function signature and rename to @main
            if let Some(at_pos) = line.find('@') {
                if let Some(paren_pos) = line.find('(') {
                    let before_name = &line[..=at_pos];
                    let after_name = &line[paren_pos..];
                    let new_line = format!("{before_name}main{after_name}");
                    result.push_str(&new_line);
                    result.push('\n');
                    main_function_found = true;
                    continue;
                }
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // If we didn't find a good place to insert, just prepend
    if prologue_added {
        result
    } else {
        format!("{prologue}{llvm_ir}")
    }
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
        assert_eq!(
            compiler.config.quantum_naming,
            QuantumNamingConvention::StandardQir
        );
    }

    #[test]
    fn test_hugr_compiler_configuration() {
        let compiler = HugrCompiler::new()
            .with_debug_info(true)
            .with_quantum_naming(QuantumNamingConvention::Hugr)
            .with_output_path("/tmp/test.ll");

        assert!(compiler.config.debug_info);
        assert_eq!(
            compiler.config.quantum_naming,
            QuantumNamingConvention::Hugr
        );
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
