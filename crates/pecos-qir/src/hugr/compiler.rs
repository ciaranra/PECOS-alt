/*!
HUGR to QIR Compiler Module

This module provides functionality to compile HUGR (Hierarchical Unified Graph Representation)
files to LLVM IR/QIR for execution on PECOS.

The compiler bridges the gap between quantum programs written in languages like Guppy
(which compile to HUGR) and the PECOS quantum execution infrastructure.

Based on the working implementation from quantum-compilation-examples.
*/

#[cfg(feature = "hugr-support")]
use hugr_core::{Hugr, std_extensions};
#[cfg(feature = "hugr-support")]
use hugr_core::package::Package;
#[cfg(feature = "hugr-support")]
use hugr_llvm::emit::EmitHugr;
#[cfg(feature = "hugr-support")]
use hugr_llvm::inkwell::context::Context;
#[cfg(feature = "hugr-support")]
use hugr_llvm::utils::fat::FatExt;
#[cfg(feature = "hugr-support")]
use log::{debug, info};
#[cfg(feature = "hugr-support")]
use pecos_core::errors::PecosError;
#[cfg(feature = "hugr-support")]
use std::path::{Path, PathBuf};
#[cfg(feature = "hugr-support")]
use std::fs;
#[cfg(feature = "hugr-support")]
use std::rc::Rc;

#[cfg(feature = "hugr-support")]
use super::result_extractor::ResultNameExtractor;
#[cfg(feature = "hugr-support")]
use super::standard_qir_generator::StandardQirExtension;

// Imports for non-hugr-support builds
#[cfg(not(feature = "hugr-support"))]
use pecos_core::errors::PecosError;
#[cfg(not(feature = "hugr-support"))]
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
    /// Standard QIR naming: __quantum__qis__h__body, etc.
    StandardQir,
    /// HUGR naming: __hugr__quantum__h, etc.
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
    pub fn new() -> Self {
        Self {
            config: HugrCompilerConfig::default(),
        }
    }

    /// Create a new HUGR compiler with custom configuration
    pub fn with_config(config: HugrCompilerConfig) -> Self {
        Self { config }
    }

    /// Set the output path for compiled LLVM IR
    pub fn with_output_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.output_path = Some(path.into());
        self
    }

    /// Enable or disable debug information
    pub fn with_debug_info(mut self, debug: bool) -> Self {
        self.config.debug_info = debug;
        self
    }

    /// Set the quantum operation naming convention
    pub fn with_quantum_naming(mut self, naming: QuantumNamingConvention) -> Self {
        self.config.quantum_naming = naming;
        self
    }

    #[cfg(feature = "hugr-support")]
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
        let hugr_bytes = fs::read(hugr_path)
            .map_err(|e| PecosError::with_context(e, format!("Failed to read HUGR file: {}", hugr_path.display())))?;

        // Determine output path
        let output_path = self.config.output_path.clone().unwrap_or_else(|| {
            hugr_path.with_extension("ll")
        });

        // Compile to LLVM IR
        self.compile_hugr_bytes(&hugr_bytes, &output_path)
    }

    #[cfg(feature = "hugr-support")]
    /// Compile HUGR bytes to LLVM IR
    pub fn compile_hugr_bytes(&self, hugr_bytes: &[u8], output_path: &Path) -> Result<PathBuf, PecosError> {
        // Load HUGR package
        let mut package = Package::load(hugr_bytes, Some(&std_extensions::std_reg()))
            .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR"))?;
        
        let hugr = std::mem::take(&mut package.modules[0]);
        
        debug!("Loaded HUGR successfully");

        // Extract result names from the HUGR
        let result_names = ResultNameExtractor::extract_result_names(&hugr)
            .map_err(|e| PecosError::with_context(e, "Failed to extract result names"))?;
        
        if !result_names.is_empty() {
            debug!("Extracted {} result name mappings from HUGR", result_names.len());
        }

        // Create LLVM context and module
        let context = Context::create();
        let module = context.create_module("quantum_module");

        // Create extensions with standard QIR quantum support
        let extensions = hugr_llvm::CodegenExtsBuilder::<Hugr>::default()
            .add_default_prelude_extensions()
            .add_logic_extensions()
            .add_extension(StandardQirExtension::new(result_names))
            .finish();

        // Create a namer that doesn't add prefixes for cleaner function names
        let namer = hugr_llvm::emit::Namer::new("", false);
        
        // Create emitter
        let emit_hugr = EmitHugr::new(
            &context,
            module,
            Rc::new(namer),
            Rc::new(extensions)
        );

        // Emit module
        let root = hugr.fat_root()
            .ok_or_else(|| PecosError::Feature("HUGR root not available".to_string()))?;
        
        let llvm_module = emit_hugr.emit_module(root)
            .map_err(|e| PecosError::from(e))?
            .finish();

        // Add EntryPoint attributes to user-defined functions
        // Note: Function renaming to "main" will be handled in post-processing
        for func in llvm_module.get_functions() {
            if func.count_basic_blocks() > 0 {  // Has a body (not just a declaration)
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
        fs::write(output_path, standard_qir)
            .map_err(|e| PecosError::with_context(e, format!("Failed to write LLVM IR to {}", output_path.display())))?;

        info!("Generated LLVM IR: {}", output_path.display());
        Ok(output_path.to_path_buf())
    }

    #[cfg(feature = "hugr-support")]
    /// Compile HUGR bytes to LLVM IR string
    pub fn compile_hugr_bytes_to_string(&self, hugr_bytes: &[u8]) -> Result<String, PecosError> {
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new()
            .map_err(|e| PecosError::with_context(e, "Failed to create temporary file"))?;
        
        self.compile_hugr_bytes(hugr_bytes, temp_file.path())?;
        
        let standard_qir = fs::read_to_string(temp_file.path())
            .map_err(|e| PecosError::with_context(e, "Failed to read generated standard QIR"))?;
        
        Ok(standard_qir)
    }

    #[cfg(not(feature = "hugr-support"))]
    /// Compile a HUGR file to LLVM IR/QIR (disabled when hugr-support feature is not enabled)
    pub fn compile_hugr<P: AsRef<Path>>(&self, _hugr_path: P) -> Result<PathBuf, PecosError> {
        use std::io::{Error, ErrorKind};
        Err(PecosError::with_context(
            Error::new(ErrorKind::Unsupported, "HUGR support not compiled in"),
            "Enable 'hugr-support' feature to use HUGR compilation"
        ))
    }
}

#[cfg(feature = "hugr-support")]
/// Add standard QIR prologue and rename functions to make the generated IR compatible with QirEngine
fn add_standard_qir_prologue(llvm_ir: &str) -> String {
    // Standard QIR prologue with type definitions and function declarations
    let prologue = r#"%Result = type opaque
%Qubit = type opaque

declare void @__quantum__qis__h__body(%Qubit*)
declare void @__quantum__qis__x__body(%Qubit*)
declare void @__quantum__qis__y__body(%Qubit*)
declare void @__quantum__qis__z__body(%Qubit*)
declare void @__quantum__qis__cx__body(%Qubit*, %Qubit*)
declare void @__quantum__qis__m__body(%Qubit*, %Result*)
declare void @__quantum__rt__result_record_output(%Result*, i8*)

"#;
    
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
        if !main_function_found && line.starts_with("define ") && !line.contains("@llvm.") && !line.contains("@__") {
            // Extract function signature and rename to @main
            if let Some(at_pos) = line.find('@') {
                if let Some(paren_pos) = line.find('(') {
                    let before_name = &line[..at_pos + 1];
                    let after_name = &line[paren_pos..];
                    let new_line = format!("{}main{}", before_name, after_name);
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
    if !prologue_added {
        format!("{}{}", prologue, llvm_ir)
    } else {
        result
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
    use tempfile::NamedTempFile;

    #[test]
    fn test_hugr_compiler_creation() {
        let compiler = HugrCompiler::new();
        assert!(!compiler.config.debug_info);
        assert_eq!(compiler.config.quantum_naming, QuantumNamingConvention::StandardQir);
    }

    #[test]
    fn test_hugr_compiler_configuration() {
        let compiler = HugrCompiler::new()
            .with_debug_info(true)
            .with_quantum_naming(QuantumNamingConvention::Hugr)
            .with_output_path("/tmp/test.ll");

        assert!(compiler.config.debug_info);
        assert_eq!(compiler.config.quantum_naming, QuantumNamingConvention::Hugr);
        assert_eq!(compiler.config.output_path, Some(PathBuf::from("/tmp/test.ll")));
    }

    #[cfg(not(feature = "hugr-support"))]
    #[test]
    fn test_hugr_compilation_without_feature() {
        let temp_file = NamedTempFile::new().unwrap();
        let compiler = HugrCompiler::new();
        
        let result = compiler.compile_hugr(temp_file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HUGR support not compiled"));
    }
}