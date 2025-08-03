/*!
Pure HUGR to LLVM IR Compiler

This module provides functionality to compile HUGR (Hierarchical Unified Graph Representation)
files to LLVM IR. It contains no execution engine dependencies - only compilation logic.
*/

use hugr_core::package::Package;
use hugr_core::Hugr;
use hugr_core::extension::ExtensionRegistry;
use hugr_llvm::emit::EmitHugr;
use hugr_llvm::extension::{int::IntCodegenExtension, prelude::DefaultPreludeCodegen};
use hugr_llvm::inkwell::context::Context;
use hugr_llvm::utils::fat::FatExt;
use log::{debug, info, trace};
use pecos_core::errors::PecosError;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::extensions::const_bool_extension::ConstBoolExtension;
use super::extensions::tket2_bool_extension::Tket2BoolExtension;
use super::extensions::tket2_rotation_extension::Tket2RotationExtension;
use super::generators::standard_llvm_generator::StandardLlvmExtension;
use super::result_extractor::ResultNameExtractor;

/// Configuration for HUGR compilation
#[derive(Debug, Clone, Default)]
pub struct HugrCompilerConfig {
    /// Output file path for the generated LLVM IR
    pub output_path: Option<PathBuf>,
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

    /// Create an extension registry with all required extensions
    fn create_extension_registry() -> ExtensionRegistry {
        // Use the standard HUGR registry which includes:
        // - prelude
        // - arithmetic.int.types, arithmetic.float.types
        // - collections.array
        // - logic, ptr, etc.
        
        
        // The HUGR package includes its own tket2 extension definitions
        // We don't need to add them here - just return the standard registry
        hugr_core::std_extensions::std_reg()
    }

    // TODO: Implement tket2.bool extension creation if needed
    // Currently the extension registry loads extensions from the HUGR package

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
        
        // Preprocess HUGR to replace ConstBool with standard values
        let preprocessed_hugr_bytes = preprocess_hugr_for_constbool(&fixed_hugr_bytes)?;

        // Create extension registry with tket2 extensions
        let registry = Self::create_extension_registry();

        // Load HUGR package with the extension registry
        let _reader = std::io::Cursor::new(preprocessed_hugr_bytes.clone());
        
        // Always try relaxed validation first for ConstBool handling
        debug!("HUGR: Attempting to load package with relaxed validation for ConstBool support");
        let mut hugr_package = load_package_with_relaxed_validation(&preprocessed_hugr_bytes, &registry)
            .or_else(|e| {
                debug!("HUGR: Relaxed validation failed: {e}, trying standard loading");
                let reader = std::io::Cursor::new(preprocessed_hugr_bytes.clone());
                Package::load(reader, Some(&registry))
            })
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
        builder = builder.add_extension(ConstBoolExtension::new());
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

        // Convert to string and apply fixes
        let llvm_ir_string = llvm_module.print_to_string().to_string();
        let fixed_entry_point = fix_entry_point_signature(&llvm_ir_string);
        let fixed_alignment = fix_struct_alignment(&fixed_entry_point);
        let with_wrappers = add_struct_return_wrappers(&fixed_alignment);
        debug!("HUGR: Calling add_main_wrapper_if_needed");
        let fixed_llvm_ir = add_main_wrapper_if_needed(&with_wrappers);
        
        // Double check if main was added
        if fixed_llvm_ir.contains("@main") {
            debug!("HUGR: Main function successfully added to LLVM IR");
        } else {
            debug!("HUGR: WARNING - Main function was NOT added to LLVM IR");
        }

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

/// Load package with relaxed validation for `ConstBool` issues
fn load_package_with_relaxed_validation(
    hugr_bytes: &[u8],
    _registry: &ExtensionRegistry,
) -> Result<Package, hugr_core::envelope::EnvelopeError> {
    use hugr_core::envelope::read_envelope;
    use hugr_core::extension::ExtensionRegistry;
    
    debug!("HUGR: Loading package with minimal extension validation");
    
    // Create a minimal extension registry that only includes basic types
    let minimal_registry = ExtensionRegistry::new(std::iter::empty());
    
    // Read the envelope with minimal validation to bypass ConstBool issues
    let reader = std::io::Cursor::new(hugr_bytes);
    let (_, package) = read_envelope(reader, &minimal_registry)?;
    
    debug!("HUGR: Package loaded successfully with relaxed validation");
    Ok(package)
}

/// Preprocess HUGR to fix `ConstBool` extension references
fn preprocess_hugr_for_constbool(hugr_bytes: &[u8]) -> Result<Vec<u8>, PecosError> {
    use serde_json::Value;
    
    // Find JSON start
    let json_start = hugr_bytes.iter().position(|&b| b == b'{').unwrap_or(0);
    let prefix = &hugr_bytes[..json_start];
    let json_bytes = &hugr_bytes[json_start..];
    
    // Parse JSON
    let json_str = std::str::from_utf8(json_bytes)
        .map_err(|e| PecosError::with_context(e, "Invalid UTF-8 in HUGR data"))?;
    
    let mut json: Value = serde_json::from_str(json_str)
        .map_err(|e| PecosError::with_context(e, "Failed to parse HUGR JSON"))?;
    
    // Process all modules to fix ConstBool values
    if let Some(modules) = json.get_mut("modules").and_then(|m| m.as_array_mut()) {
        for module in modules {
            fix_constbool_in_module(module);
        }
    }
    
    // Remove tket2.bool from required extensions in any nodes that use it
    // This is crucial to avoid the "requires extensions" error
    if let Some(modules) = json.get_mut("modules").and_then(|m| m.as_array_mut()) {
        for module in modules {
            remove_tket2_bool_references(module);
        }
    }
    
    // Serialize back to bytes
    let fixed_json = serde_json::to_string(&json)
        .map_err(|e| PecosError::with_context(e, "Failed to serialize fixed JSON"))?;
    
    let mut result = prefix.to_vec();
    result.extend_from_slice(fixed_json.as_bytes());
    
    Ok(result)
}

/// Fix `ConstBool` values in a module
fn fix_constbool_in_module(module: &mut serde_json::Value) {
    if let Some(nodes) = module.get_mut("nodes").and_then(|n| n.as_array_mut()) {
        for node in nodes {
            // Check if this is a Const node (can be string or object)
            let is_const = match node.get("op") {
                Some(serde_json::Value::String(s)) => s == "Const",
                Some(serde_json::Value::Object(obj)) => obj.get("op").is_some_and(|v| v == "Const"),
                _ => false,
            };
            
            if is_const {
                // Handle Const nodes with ConstBool values
                if let Some(v) = node.get_mut("v") {
                    if let Some(v_type) = v.get("v").and_then(|vt| vt.as_str()) {
                        if v_type == "Extension" {
                            if let Some(value) = v.get_mut("value") {
                                if let Some(c) = value.get("c") {
                                    if c == "ConstBool" {
                                        debug!("Found ConstBool value: {value:?}");
                                        
                                        // Get the boolean value
                                        let bool_val = value.get("v").and_then(serde_json::Value::as_bool).unwrap_or(false);
                                        
                                        // Replace the entire const value with a Sum constant
                                        *v = serde_json::json!({
                                            "v": "Sum",
                                            "tag": i32::from(bool_val),
                                            "typ": {
                                                "t": "Sum",
                                                "s": "Unit", 
                                                "size": 2
                                            },
                                            "vs": []
                                        });
                                        
                                        debug!("Fixed ConstBool to Sum constant");
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

/// Remove references to tket2.bool extension from nodes
fn remove_tket2_bool_references(module: &mut serde_json::Value) {
    if let Some(nodes) = module.get_mut("nodes").and_then(|n| n.as_array_mut()) {
        for node in nodes {
            // Remove tket2.bool from type references
            if let Some(op) = node.get_mut("op") {
                if let Some(op_obj) = op.as_object_mut() {
                    // Check LoadConstant nodes
                    if let Some(datatype) = op_obj.get_mut("datatype") {
                        if let Some(dt_obj) = datatype.as_object_mut() {
                            if dt_obj.get("extension").and_then(|e| e.as_str()) == Some("tket2.bool") {
                                // Convert to standard bool type
                                *datatype = serde_json::json!({
                                    "t": "Sum",
                                    "s": "Unit",
                                    "size": 2
                                });
                            }
                        }
                    }
                    
                    // Check function signatures
                    if let Some(signature) = op_obj.get_mut("signature") {
                        update_signature_types(signature);
                    }
                }
            }
            
            // Check Input/Output nodes
            if let Some(types) = node.get_mut("types").and_then(|t| t.as_array_mut()) {
                for typ in types {
                    if let Some(t_obj) = typ.as_object_mut() {
                        if t_obj.get("extension").and_then(|e| e.as_str()) == Some("tket2.bool") {
                            *typ = serde_json::json!({
                                "t": "Sum",
                                "s": "Unit",
                                "size": 2
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Update signature types to replace tket2.bool with standard bool
fn update_signature_types(sig: &mut serde_json::Value) {
    if let Some(body) = sig.get_mut("body") {
        if let Some(input) = body.get_mut("input").and_then(|i| i.as_array_mut()) {
            for typ in input {
                replace_tket2_bool_type(typ);
            }
        }
        if let Some(output) = body.get_mut("output").and_then(|o| o.as_array_mut()) {
            for typ in output {
                replace_tket2_bool_type(typ);
            }
        }
    }
}

/// Replace a tket2.bool type reference with standard bool type
fn replace_tket2_bool_type(typ: &mut serde_json::Value) {
    if let Some(t_obj) = typ.as_object_mut() {
        if t_obj.get("extension").and_then(|e| e.as_str()) == Some("tket2.bool") {
            *typ = serde_json::json!({
                "t": "Sum",
                "s": "Unit",
                "size": 2
            });
        }
    }
}

/// Fix struct store alignment mismatches in LLVM IR
/// 
/// The hugr-llvm crate generates struct stores with align 1, but LLVM allocates
/// structs with their natural alignment (typically 8 bytes). This mismatch causes
/// segmentation faults when accessing the misaligned memory.
fn fix_struct_alignment(llvm_ir: &str) -> String {
    use regex::Regex;
    
    // Fix alignment mismatches for struct stores
    // Pattern: store { i1, ... } %value, { i1, ... }* %ptr, align 1
    // Should be: store { i1, ... } %value, { i1, ... }* %ptr, align 8
    let struct_store_pattern = Regex::new(
        r"store\s+(\{[^}]+\})\s+([^,]+),\s+(\{[^}]+\}\*)\s+([^,]+),\s+align\s+1\b"
    ).unwrap();
    
    let result = struct_store_pattern.replace_all(llvm_ir, "store $1 $2, $3 $4, align 8");
    
    // Also fix empty struct stores: store {} %value, {}* %ptr, align 1
    let empty_struct_pattern = Regex::new(
        r"store\s+(\{\})\s+([^,]+),\s+(\{\}\*)\s+([^,]+),\s+align\s+1\b"
    ).unwrap();
    
    empty_struct_pattern.replace_all(&result, "store $1 $2, $3 $4, align 8").to_string()
}

/// Convert small struct returns to integer returns for ABI compatibility
/// 
/// LLVM functions that return { i1, i1, i1, i1 } cause issues with FFI.
/// This converts them to return i8 instead, packing the bits.
#[allow(dead_code)]
fn fix_small_struct_returns(llvm_ir: &str) -> String {
    use regex::Regex;
    
    // Check if this function returns a small struct of booleans
    let func_sig_pattern = Regex::new(
        r"define\s+\{\s*i1,\s*i1,\s*i1,\s*i1\s*\}\s+@(\w+)"
    ).unwrap();
    
    if let Some(captures) = func_sig_pattern.captures(llvm_ir) {
        let func_name = &captures[1];
        debug!("Found function {func_name} returning {{ i1, i1, i1, i1 }}, converting to i8");
        
        // Replace the function signature
        let mut result = func_sig_pattern.replace(
            llvm_ir, 
            format!("define i8 @{func_name}")
        ).to_string();
        
        // Find the return statement and convert it
        // Pattern: ret { i1, i1, i1, i1 } %value
        let ret_pattern = Regex::new(
            r"ret\s+\{\s*i1,\s*i1,\s*i1,\s*i1\s*\}\s+(%\w+)"
        ).unwrap();
        
        if let Some(ret_captures) = ret_pattern.captures(&result) {
            let ret_var = &ret_captures[1];
            
            // Insert code to pack the struct into an i8
            // We need to extract each field and pack them
            let pack_code = format!(
                r"  ; Pack {{ i1, i1, i1, i1 }} into i8
  %pack0 = extractvalue {{ i1, i1, i1, i1 }} {ret_var}, 0
  %pack1 = extractvalue {{ i1, i1, i1, i1 }} {ret_var}, 1
  %pack2 = extractvalue {{ i1, i1, i1, i1 }} {ret_var}, 2
  %pack3 = extractvalue {{ i1, i1, i1, i1 }} {ret_var}, 3
  %pack0_i8 = zext i1 %pack0 to i8
  %pack1_i8 = zext i1 %pack1 to i8
  %pack2_i8 = zext i1 %pack2 to i8
  %pack3_i8 = zext i1 %pack3 to i8
  %pack1_shift = shl i8 %pack1_i8, 1
  %pack2_shift = shl i8 %pack2_i8, 2
  %pack3_shift = shl i8 %pack3_i8, 3
  %pack01 = or i8 %pack0_i8, %pack1_shift
  %pack012 = or i8 %pack01, %pack2_shift
  %packed = or i8 %pack012, %pack3_shift
  ret i8 %packed"
            );
            
            result = ret_pattern.replace(&result, &pack_code).to_string();
        }
        
        return result;
    }
    
    llvm_ir.to_string()
}

/// Add a main function wrapper if there's no main function but there's an EntryPoint
fn add_main_wrapper_if_needed(llvm_ir: &str) -> String {
    use log::debug;
    
    // Check if main already exists
    if llvm_ir.contains("@main(") || llvm_ir.contains("@main ") {
        debug!("HUGR: Main function already exists, skipping wrapper");
        return llvm_ir.to_string();
    }
    
    // For HUGR-generated code, we know the pattern: functions start with _hugr_
    // and the entry point is the one without _wrapper, _get_ etc suffixes
    let mut entry_function: Option<(&str, &str)> = None;
    
    for line in llvm_ir.lines() {
        if line.starts_with("define ") && line.contains("@_hugr_") {
            // Skip wrapper and accessor functions
            if line.contains("_wrapper") || line.contains("_get_") {
                continue;
            }
            
            // This should be the main entry function
            if let Some(func_start) = line.find('@') {
                if let Some(func_end) = line[func_start + 1..].find('(') {
                    let func_name = &line[func_start + 1..func_start + 1 + func_end];
                    
                    // Extract the return type
                    let return_type = if let Some(define_end) = line.find("define ") {
                        let after_define = &line[define_end + 7..];
                        if let Some(at_pos) = after_define.find('@') {
                            after_define[..at_pos].trim()
                        } else {
                            "void"
                        }
                    } else {
                        "void"
                    };
                    
                    entry_function = Some((func_name, return_type));
                    break;
                }
            }
        }
    }
    
    debug!("HUGR: Entry function found: {:?}", entry_function);
    
    // If we found an entry point function, add a main wrapper
    if let Some((entry_name, return_type)) = entry_function {
        debug!("HUGR: Adding main wrapper for {} with return type {}", entry_name, return_type);
        
        // Create main wrapper that calls the entry function
        let main_wrapper = format!(
            r#"
; Main wrapper for Selene compatibility
define i32 @main() {{
entry:
  %result = call {return_type} @{entry_name}()
  ret i32 0
}}
"#
        );
        
        // Add the wrapper before the attributes section or at the end
        // Look for attributes on its own line or after other content
        if let Some(attr_pos) = llvm_ir.find("attributes ") {
            // Find the start of the line
            let line_start = llvm_ir[..attr_pos].rfind('\n').unwrap_or(0);
            let mut result = String::new();
            result.push_str(&llvm_ir[..line_start]);
            result.push_str(&main_wrapper);
            result.push_str(&llvm_ir[line_start..]);
            debug!("HUGR: Inserted main wrapper before attributes section");
            return result;
        } else {
            // Just append at the end
            debug!("HUGR: Appending main wrapper at the end");
            return format!("{llvm_ir}\n{main_wrapper}");
        }
    }
    
    llvm_ir.to_string()
}

/// Add wrapper functions for struct returns that are FFI-safe
/// This creates a global variable to hold the struct and a wrapper that returns a pointer
fn add_struct_return_wrappers(llvm_ir: &str) -> String {
    use regex::Regex;
    
    // Find functions that return structs
    let func_pattern = Regex::new(r"define\s+(\{[^}]+\})\s+@(\w+)\(\)\s+").unwrap();
    
    let mut result = llvm_ir.to_string();
    let mut wrappers = String::new();
    
    // Process each function that returns a struct
    for cap in func_pattern.captures_iter(llvm_ir) {
        let struct_type = &cap[1];
        let func_name = &cap[2];
        
        // Skip if it's not a struct with multiple elements
        if !struct_type.contains(',') {
            continue;
        }
        
        // Create a global variable to hold the result
        wrappers.push_str(&format!(
            "\n; Global storage for {func_name} result\n\
             @{func_name}_result = internal global {struct_type} zeroinitializer, align 8\n"
        ));
        
        // Create a wrapper function that stores to global and returns pointer
        wrappers.push_str(&format!(
            "\n; FFI-safe wrapper for {func_name}\n\
             define {struct_type}* @{func_name}_wrapper() {{\n\
               %1 = call {struct_type} @{func_name}()\n\
               store {struct_type} %1, {struct_type}* @{func_name}_result, align 8\n\
               ret {struct_type}* @{func_name}_result\n\
             }}\n"
        ));
        
        // Also create accessor functions to get individual elements
        // Count elements in the struct
        let elements: Vec<&str> = struct_type[1..struct_type.len()-1]
            .split(',')
            .map(str::trim)
            .collect();
        
        for (i, elem_type) in elements.iter().enumerate() {
            wrappers.push_str(&format!(
                "\n; Accessor for element {i} of {func_name} result\n\
                 define {elem_type} @{func_name}_get_{i}() {{\n\
                   %1 = load {struct_type}, {struct_type}* @{func_name}_result, align 8\n\
                   %2 = extractvalue {struct_type} %1, {i}\n\
                   ret {elem_type} %2\n\
                 }}\n"
            ));
        }
    }
    
    // Add wrappers at the end of the module
    if !wrappers.is_empty() {
        result.push_str(&wrappers);
    }
    
    result
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
        let _compiler = HugrCompiler::new();
        // Basic creation test
    }

    #[test]
    fn test_hugr_compiler_configuration() {
        let compiler = HugrCompiler::new()
            .with_output_path("/tmp/test.ll");

        assert_eq!(
            compiler.config.output_path,
            Some(PathBuf::from("/tmp/test.ll"))
        );
    }
}
