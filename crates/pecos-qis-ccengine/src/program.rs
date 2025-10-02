//! Program abstraction for QIS Classical Control Engine
//!
//! This module provides a unified program interface that allows different
//! program types (QisProgram, HUGR, raw QisInterface) to be used with
//! the QisControlEngine through a consistent `.program()` API.
//!
//! Default implementations use Selene-based interfaces with explicit
//! error handling - no silent fallbacks are provided.

use pecos_core::errors::PecosError;
use pecos_programs::{QisProgram, HugrProgram};
use pecos_qis_interface::QisInterface;
use std::process::Command;
use tempfile::NamedTempFile;
use crate::jit_executor::JitExecutor;

/// A trait for types that can be converted into a QisInterface
///
/// This allows the QisControlEngine builder to accept different program types
/// through a unified `.program()` method, similar to how QASMEngine works.
///
/// Default implementations use Selene-based interfaces (Helios for QIS/HUGR programs).
/// If the default is not available, explicit error messages guide users to alternatives.
pub trait IntoQisInterface {
    /// Convert this program into a QisInterface using default backend (Selene-based)
    ///
    /// # Errors
    /// Returns an error if the conversion fails (e.g., compilation errors,
    /// invalid program format, missing Selene dependencies). No silent fallbacks.
    fn into_qis_interface(self) -> Result<QisInterface, PecosError>;

    /// Convert this program into a QisInterface using JIT backend
    fn into_qis_interface_with_jit(self) -> Result<QisInterface, PecosError>
    where
        Self: Sized,
    {
        // Default implementation converts to default format then uses JIT
        self.into_qis_interface()
    }

    /// Convert this program into a QisInterface using Helios backend
    fn into_qis_interface_with_helios(self) -> Result<QisInterface, PecosError>
    where
        Self: Sized,
    {
        // Default implementation converts to default format then uses Helios
        self.into_qis_interface()
    }
}

/// Program type classification for interface provider selection
#[derive(Debug, Clone, PartialEq)]
pub enum ProgramType {
    /// LLVM IR text format
    LlvmIr,
    /// QIS bitcode format
    QisBitcode,
    /// HUGR bytes format
    HugrBytes,
}

/// Trait for different QisInterface implementation strategies
///
/// This allows pluggable compilation strategies - JIT execution,
/// Selene Helios compilation, or other future approaches.
pub trait QisInterfaceProvider: Send + Sync {
    /// Get the interface (may involve compilation/linking)
    fn get_interface(&mut self) -> Result<QisInterface, PecosError>;

    /// Get provider type for debugging and logging
    fn provider_type(&self) -> &'static str;

    /// Check if this provider can handle the given program type
    fn can_handle(&self, program_type: &ProgramType) -> bool;

    /// Get any metadata about the compilation process
    fn get_metadata(&self) -> std::collections::BTreeMap<String, String> {
        std::collections::BTreeMap::new()
    }
}

/// JIT-based QisInterface provider using PECOS JIT executor
///
/// This provider compiles LLVM IR using inkwell and executes it with
/// proper FFI function registration to collect quantum operations.
///
/// Designed for the MonteCarloEngine template pattern:
/// - Create a template QisJitInterface
/// - Clone it for each worker/thread
/// - Each clone's JitExecutor will create its own LLVM Context
/// - Reuse for multiple shots by resetting state as needed
#[derive(Debug, Clone)]
pub struct QisJitInterface {
    llvm_ir: String,
    // executor field removed - we create fresh JitExecutor instances for each execution
    // to avoid LLVM global state accumulation issues
    metadata: std::collections::BTreeMap<String, String>,
}

impl QisJitInterface {
    /// Create a new JIT interface provider from LLVM IR
    pub fn from_llvm_ir(llvm_ir: String) -> Self {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert("ir_size".to_string(), llvm_ir.len().to_string());
        metadata.insert("compilation_strategy".to_string(), "jit".to_string());

        Self {
            llvm_ir,
            metadata,
        }
    }

    /// Create from LLVM bitcode by converting to IR text
    pub fn from_bitcode(bitcode: Vec<u8>) -> Self {
        // Convert bitcode to IR text using inkwell
        use inkwell::context::Context;
        use inkwell::memory_buffer::MemoryBuffer;

        let context = Context::create();
        let memory_buffer = MemoryBuffer::create_from_memory_range(&bitcode, "bitcode");

        let llvm_ir = match context.create_module_from_ir(memory_buffer) {
            Ok(module) => module.to_string(),
            Err(_) => {
                // Fallback: treat as raw bitcode and attempt to parse
                log::warn!("Could not parse bitcode, attempting as raw module");
                // For now, we'll store a placeholder and handle in execute
                format!("<bitcode:{}>", bitcode.len())
            }
        };

        let mut interface = Self::from_llvm_ir(llvm_ir);
        interface.metadata.insert("original_format".to_string(), "bitcode".to_string());
        interface
    }

    /// Create from HUGR bytes by compiling to LLVM IR
    pub fn from_hugr_bytes(hugr_bytes: Vec<u8>) -> Self {
        // Use pecos-hugr-qis to compile HUGR to LLVM IR
        let llvm_ir = match pecos_hugr_qis::compile_hugr_bytes_to_string(&hugr_bytes) {
            Ok(ir) => ir,
            Err(e) => {
                log::error!("Failed to compile HUGR to LLVM IR: {}", e);
                // Return error placeholder that will fail during execution
                format!("<hugr-compilation-error: {}>", e)
            }
        };

        let mut interface = Self::from_llvm_ir(llvm_ir);
        interface.metadata.insert("original_format".to_string(), "hugr".to_string());
        interface.metadata.insert("hugr_size".to_string(), hugr_bytes.len().to_string());
        interface
    }


    /// Get compilation statistics from the JIT executor
    /// Note: Since we create fresh executors for each execution, statistics are not persistent
    pub fn get_compilation_stats(&self) -> (usize, usize, f64) {
        (0, 0, 0.0) // No persistent stats since we use fresh executors each time
    }

    /// Get cache statistics from the JIT executor
    /// Note: Since we create fresh executors for each execution, cache stats are not persistent
    pub fn get_cache_stats(&self) -> (usize, f64) {
        (0, 0.0) // No persistent cache since we use fresh executors each time
    }

    /// Ensure qubits and result slots are properly allocated based on operations
    /// This fixes the issue where LLVM IR uses qubits without explicit allocation
    fn ensure_proper_allocation(&self, interface: &mut QisInterface) {
        let mut max_qubit_id: Option<usize> = None;
        let mut max_result_id: Option<usize> = None;

        // Scan all operations to find maximum qubit and result IDs
        for operation in &interface.operations {
            match operation {
                pecos_qis_interface::Operation::Quantum(qop) => {
                    let qubits = self.extract_qubit_ids_from_op(qop);
                    if let Some(max_in_op) = qubits.into_iter().max() {
                        max_qubit_id = Some(max_qubit_id.map_or(max_in_op, |current: usize| current.max(max_in_op)));
                    }

                    // Check for measurement operations to find result IDs
                    if let pecos_qis_interface::QuantumOp::Measure(_, result_id) = qop {
                        max_result_id = Some(max_result_id.map_or(*result_id, |current: usize| current.max(*result_id)));
                    }
                }
                pecos_qis_interface::Operation::AllocateQubit { id } => {
                    max_qubit_id = Some(max_qubit_id.map_or(*id, |current: usize| current.max(*id)));
                }
                pecos_qis_interface::Operation::AllocateResult { id } => {
                    max_result_id = Some(max_result_id.map_or(*id, |current: usize| current.max(*id)));
                }
                pecos_qis_interface::Operation::ReleaseQubit { id } => {
                    max_qubit_id = Some(max_qubit_id.map_or(*id, |current: usize| current.max(*id)));
                }
                _ => {}
            }
        }

        // Allocate qubits up to the maximum ID found
        if let Some(max_id) = max_qubit_id {
            let original_qubit_count = interface.allocated_qubits.len();
            while interface.allocated_qubits.len() <= max_id {
                interface.allocate_qubit();
            }
            if interface.allocated_qubits.len() > original_qubit_count {
                log::warn!("WARNING: Auto-allocated {} qubits due to missing explicit allocation. \
                           Consider adding explicit allocation calls to your LLVM IR for better performance.",
                           interface.allocated_qubits.len() - original_qubit_count);
            }
        }

        // Allocate result slots up to the maximum ID found
        if let Some(max_id) = max_result_id {
            let original_result_count = interface.allocated_results.len();
            while interface.allocated_results.len() <= max_id {
                interface.allocate_result();
            }
            if interface.allocated_results.len() > original_result_count {
                log::warn!("WARNING: Auto-allocated {} result slots due to missing explicit allocation. \
                           Consider adding explicit allocation calls to your LLVM IR for better performance.",
                           interface.allocated_results.len() - original_result_count);
            }
        }
    }

    /// Extract qubit IDs from a quantum operation
    fn extract_qubit_ids_from_op(&self, qop: &pecos_qis_interface::QuantumOp) -> Vec<usize> {
        use pecos_qis_interface::QuantumOp;
        match qop {
            // Single-qubit gates
            QuantumOp::H(q) | QuantumOp::X(q) | QuantumOp::Y(q) | QuantumOp::Z(q) |
            QuantumOp::S(q) | QuantumOp::Sdg(q) | QuantumOp::T(q) | QuantumOp::Tdg(q) |
            QuantumOp::Reset(q) => vec![*q],

            // Rotation gates
            QuantumOp::RX(_, q) | QuantumOp::RY(_, q) | QuantumOp::RZ(_, q) |
            QuantumOp::RXY(_, _, q) => vec![*q],

            // Two-qubit gates
            QuantumOp::CX(c, t) | QuantumOp::CY(c, t) | QuantumOp::CZ(c, t) |
            QuantumOp::CH(c, t) | QuantumOp::ZZ(c, t) | QuantumOp::RZZ(_, c, t) => vec![*c, *t],

            // Controlled rotations
            QuantumOp::CRZ(_, c, t) => vec![*c, *t],

            // Three-qubit gates
            QuantumOp::CCX(c1, c2, t) => vec![*c1, *c2, *t],

            // Measurement
            QuantumOp::Measure(q, _) => vec![*q],
        }
    }
}

impl QisInterfaceProvider for QisJitInterface {
    fn get_interface(&mut self) -> Result<QisInterface, PecosError> {
        log::info!("Using JIT compilation strategy for LLVM IR");

        // CRITICAL FIX: Create a fresh JitExecutor for each execution to avoid LLVM global state issues
        // The comment in jit_executor.rs states: "each JitExecutor instance should be used once"
        // due to LLVM global state accumulation when contexts are reused
        let mut fresh_executor = JitExecutor::new();
        let mut result = fresh_executor.execute_llvm_ir(&self.llvm_ir)?;

        // Ensure qubits and results are properly allocated based on operations
        self.ensure_proper_allocation(&mut result);

        // Update metadata with execution results
        self.metadata.insert("operations_count".to_string(), result.operations.len().to_string());
        self.metadata.insert("qubits_allocated".to_string(), result.allocated_qubits.len().to_string());
        self.metadata.insert("results_allocated".to_string(), result.allocated_results.len().to_string());

        let (compilations, cache_hits, hit_rate) = self.get_compilation_stats();
        self.metadata.insert("total_compilations".to_string(), compilations.to_string());
        self.metadata.insert("cache_hits".to_string(), cache_hits.to_string());
        self.metadata.insert("cache_hit_rate".to_string(), format!("{:.2}", hit_rate));

        log::info!("JIT interface created: {} operations, {} qubits",
                   result.operations.len(), result.allocated_qubits.len());

        Ok(result)
    }

    fn provider_type(&self) -> &'static str {
        "JIT"
    }

    fn can_handle(&self, program_type: &ProgramType) -> bool {
        matches!(program_type, ProgramType::LlvmIr)
    }

    fn get_metadata(&self) -> std::collections::BTreeMap<String, String> {
        self.metadata.clone()
    }
}

/// Selene Helios-based QisInterface provider
///
/// This provider uses Selene's Helios compiler to compile QIS bitcode
/// into optimized quantum programs, then converts the result into a QisInterface.
#[derive(Debug)]
pub struct QisSeleneHeliosInterface {
    program_data: Vec<u8>,
    program_type: ProgramType,
    metadata: std::collections::BTreeMap<String, String>,
    helios_config: HeliosConfig,
}

/// Configuration for Selene Helios compilation
#[derive(Debug, Clone)]
pub struct HeliosConfig {
    /// Optimization level (0-3)
    pub opt_level: u8,
    /// Target triple for compilation
    pub target_triple: String,
    /// Additional compilation flags
    pub extra_flags: Vec<String>,
    /// Path to Selene installation
    pub selene_path: Option<std::path::PathBuf>,
}

impl Default for HeliosConfig {
    fn default() -> Self {
        Self {
            opt_level: 2,
            target_triple: "native".to_string(),
            extra_flags: Vec::new(),
            selene_path: None,
        }
    }
}

impl QisSeleneHeliosInterface {
    /// Create a new Selene Helios interface provider from QIS bitcode
    pub fn from_bitcode(bitcode: Vec<u8>) -> Self {
        Self::from_bitcode_with_config(bitcode, HeliosConfig::default())
    }

    /// Create a new Selene Helios interface provider with custom configuration
    pub fn from_bitcode_with_config(bitcode: Vec<u8>, config: HeliosConfig) -> Self {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert("bitcode_size".to_string(), bitcode.len().to_string());
        metadata.insert("compilation_strategy".to_string(), "selene_helios".to_string());
        metadata.insert("opt_level".to_string(), config.opt_level.to_string());

        Self {
            program_data: bitcode,
            program_type: ProgramType::QisBitcode,
            metadata,
            helios_config: config,
        }
    }

    /// Create a new Selene Helios interface provider from HUGR bytes
    pub fn from_hugr_bytes(hugr_bytes: Vec<u8>) -> Self {
        Self::from_hugr_bytes_with_config(hugr_bytes, HeliosConfig::default())
    }

    /// Create a new Selene Helios interface provider from HUGR bytes with custom configuration
    pub fn from_hugr_bytes_with_config(hugr_bytes: Vec<u8>, config: HeliosConfig) -> Self {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert("hugr_size".to_string(), hugr_bytes.len().to_string());
        metadata.insert("compilation_strategy".to_string(), "selene_helios".to_string());
        metadata.insert("opt_level".to_string(), config.opt_level.to_string());

        Self {
            program_data: hugr_bytes,
            program_type: ProgramType::HugrBytes,
            metadata,
            helios_config: config,
        }
    }

    /// Create from LLVM IR text by converting to bitcode
    pub fn from_llvm_ir(llvm_ir: String) -> Self {
        // Convert LLVM IR text to bitcode using inkwell
        use inkwell::context::Context;
        use inkwell::targets::{InitializationConfig, Target};

        // Initialize LLVM targets
        Target::initialize_native(&InitializationConfig::default()).ok();

        let context = Context::create();
        let bitcode = match context.create_module_from_ir(
            inkwell::memory_buffer::MemoryBuffer::create_from_memory_range(
                llvm_ir.as_bytes(),
                "llvm_ir",
            )
        ) {
            Ok(module) => {
                // Write module to bitcode
                module.write_bitcode_to_memory().as_slice().to_vec()
            }
            Err(e) => {
                log::error!("Failed to convert LLVM IR to bitcode: {}", e);
                // Store the IR text as-is and let Helios handle it
                llvm_ir.as_bytes().to_vec()
            }
        };

        let mut interface = Self::from_bitcode(bitcode);
        interface.metadata.insert("original_format".to_string(), "llvm_ir".to_string());
        interface.metadata.insert("ir_size".to_string(), llvm_ir.len().to_string());
        interface
    }

    /// Compile the program using Selene Helios and convert to QisInterface
    fn compile_with_helios(&mut self) -> Result<QisInterface, PecosError> {
        log::info!("Using Selene Helios compilation strategy for {:?}", self.program_type);

        match self.program_type {
            ProgramType::QisBitcode => {
                self.compile_bitcode_with_helios()
            }
            ProgramType::HugrBytes => {
                self.compile_hugr_with_helios()
            }
            ProgramType::LlvmIr => {
                Err(PecosError::Generic(
                    "Selene Helios interface cannot compile LLVM IR text directly.\n\
                     \n\
                     The Helios interface is designed for HUGR bytes and QIS bitcode formats.\n\
                     For LLVM IR text, please use qis_jit_interface() instead of qis_selene_helios_interface().\n\
                     \n\
                     Example:\n\
                     engine = qis_control_engine()\n\
                         .interface(qis_jit_interface())  // Use JIT for LLVM IR\n\
                         .program(qis_program)".to_string()
                ))
            }
        }
    }

    /// Compile QIS bitcode using Selene Helios
    fn compile_bitcode_with_helios(&mut self) -> Result<QisInterface, PecosError> {
        // Compile bitcode to LLVM IR using Selene Helios
        let llvm_ir = self.compile_bitcode_to_llvm_ir()?;

        // Store the LLVM IR for potential future use
        self.metadata.insert("helios_llvm_ir_size".to_string(), llvm_ir.len().to_string());

        // For now, we still need to use JIT to convert LLVM IR to QisInterface
        // TODO: In the future, Helios should directly produce a QisInterface
        let mut jit_interface = QisJitInterface::from_llvm_ir(llvm_ir);
        let result = jit_interface.get_interface()?;

        log::info!("Selene Helios compilation successful: {} operations, {} qubits",
                   result.operations.len(), result.allocated_qubits.len());

        Ok(result)
    }

    /// Compile HUGR bytes using Selene Helios
    fn compile_hugr_with_helios(&mut self) -> Result<QisInterface, PecosError> {
        // Use Selene HUGR compiler (no fallback)
        let llvm_ir = compile_hugr_with_selene(&self.program_data)?;

        // Store the LLVM IR for potential future use
        self.metadata.insert("helios_llvm_ir_size".to_string(), llvm_ir.len().to_string());

        // For now, we still need to use JIT to convert LLVM IR to QisInterface
        // TODO: In the future, Helios should directly produce a QisInterface
        let mut jit_interface = QisJitInterface::from_llvm_ir(llvm_ir);
        let result = jit_interface.get_interface()?;

        log::info!("Selene HUGR compilation successful: {} operations, {} qubits",
                   result.operations.len(), result.allocated_qubits.len());

        Ok(result)
    }

    /// Compile QIS bitcode to LLVM IR using Selene Helios compiler
    fn compile_bitcode_to_llvm_ir(&mut self) -> Result<String, PecosError> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Write bitcode to a temporary file
        let mut bitcode_file = NamedTempFile::new()
            .map_err(|e| PecosError::Generic(format!("Failed to create temp file: {}", e)))?;
        bitcode_file.write_all(&self.program_data)
            .map_err(|e| PecosError::Generic(format!("Failed to write bitcode: {}", e)))?;

        // Try multiple strategies to find and use Selene Helios
        self.try_selene_helios_compilation(&bitcode_file)
    }

    /// Try different strategies for Selene Helios compilation
    fn try_selene_helios_compilation(&mut self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let strategy_names = [
            "Custom Path",
            "Environment Variable",
            "Standard Locations",
            "Conda Environment",
            "System Installation",
        ];

        let strategies = [
            self.try_custom_selene_path(bitcode_file),
            self.try_env_selene_path(bitcode_file),
            self.try_standard_selene_locations(bitcode_file),
            self.try_conda_selene(bitcode_file),
            self.try_system_selene(bitcode_file),
        ];

        for (strategy_name, result) in strategy_names.iter().zip(strategies.iter()) {
            match result {
                Ok(llvm_ir) => {
                    log::info!("Selene Helios compilation succeeded using: {}", strategy_name);
                    self.metadata.insert("helios_strategy".to_string(), strategy_name.to_string());
                    self.metadata.insert("helios_compilation".to_string(), "success".to_string());
                    self.metadata.insert("llvm_ir_size".to_string(), llvm_ir.len().to_string());
                    return Ok(llvm_ir.clone());
                }
                Err(e) => {
                    log::debug!("Selene Helios strategy '{}' failed: {}", strategy_name, e);
                    self.metadata.insert(format!("helios_strategy_{}_error", strategy_name.to_lowercase().replace(' ', "_")), e.to_string());
                }
            }
        }

        // If all strategies fail, provide helpful error message
        Err(PecosError::Generic(format!(
            "Selene Helios compilation failed. Unable to find Selene installation after trying: {}. \n\
             \n\
             To use Helios interface, you need to:\n\
             1. Install Selene (https://github.com/CQCL/selene)\n\
             2. Set SELENE_PATH environment variable to the Selene directory\n\
             \n\
             Alternatively, use qis_jit_interface() instead of qis_selene_helios_interface(), \
             which doesn't require Selene.",
            strategy_names.join(", ")
        )))
    }

    /// Try compilation using user-provided Selene path
    fn try_custom_selene_path(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let selene_path = self.helios_config.selene_path
            .as_ref()
            .ok_or_else(|| PecosError::Generic("No custom Selene path provided".to_string()))?;

        self.run_selene_helios_compiler(selene_path, bitcode_file)
    }

    /// Try compilation using SELENE_PATH environment variable
    fn try_env_selene_path(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let selene_path = std::env::var("SELENE_PATH")
            .map_err(|_| PecosError::Generic("SELENE_PATH not set".to_string()))?;

        let path = std::path::PathBuf::from(selene_path);
        self.run_selene_helios_compiler(&path, bitcode_file)
    }

    /// Try compilation using standard Selene installation locations
    fn try_standard_selene_locations(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let standard_paths = [
            "/home/ciaranra/Repos/cl_projects/gup/selene",
            "/opt/selene",
            "/usr/local/selene",
            "~/selene",
            "./selene",
            "../selene",
        ];

        for path_str in &standard_paths {
            let path = std::path::PathBuf::from(path_str);
            if path.exists() && path.join("selene-compilers/helios/python").exists() {
                log::debug!("Found Selene at standard location: {}", path.display());
                return self.run_selene_helios_compiler(&path, bitcode_file);
            }
        }

        Err(PecosError::Generic("No Selene found in standard locations".to_string()))
    }

    /// Try compilation using conda environment
    fn try_conda_selene(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        // Check if we're in a conda environment with Selene
        let python_script = format!(r#"
import sys
try:
    import selene_helios_compiler
    print(selene_helios_compiler.__file__)
except ImportError:
    sys.exit(1)
"#);

        let output = Command::new("python3")
            .arg("-c")
            .arg(&python_script)
            .output()
            .map_err(|e| PecosError::Generic(format!("Failed to check conda Selene: {}", e)))?;

        if !output.status.success() {
            return Err(PecosError::Generic("Selene not available in conda environment".to_string()));
        }

        // Run compilation directly using available Python module
        self.run_conda_selene_compilation(bitcode_file)
    }

    /// Try compilation using system-installed Selene
    fn try_system_selene(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        // Check if selene-helios command is available in PATH
        let output = Command::new("which")
            .arg("selene-helios")
            .output()
            .map_err(|_| PecosError::Generic("selene-helios not in PATH".to_string()))?;

        if !output.status.success() {
            return Err(PecosError::Generic("selene-helios command not found".to_string()));
        }

        // Use command-line tool
        self.run_system_selene_compilation(bitcode_file)
    }

    /// Run Selene Helios compiler from a specific path
    fn run_selene_helios_compiler(&self, selene_path: &std::path::Path, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let helios_python_path = selene_path.join("selene-compilers/helios/python");

        if !helios_python_path.exists() {
            return Err(PecosError::Generic(format!(
                "Selene Helios Python path not found: {}",
                helios_python_path.display()
            )));
        }

        let python_script = format!(r#"
import sys
sys.path.insert(0, '{helios_python_path}')

try:
    from selene_helios_compiler import compile_bitcode_to_llvm_ir
except ImportError as e:
    print(f"Failed to import Selene Helios compiler: {{e}}", file=sys.stderr)
    sys.exit(1)

try:
    with open('{bitcode_path}', 'rb') as f:
        bitcode = f.read()

    llvm_ir = compile_bitcode_to_llvm_ir(
        bitcode,
        opt_level={opt_level},
        target_triple='{target_triple}'
    )
    print(llvm_ir)
except Exception as e:
    print(f"Compilation failed: {{e}}", file=sys.stderr)
    sys.exit(1)
"#,
            helios_python_path = helios_python_path.display(),
            bitcode_path = bitcode_file.path().display(),
            opt_level = self.helios_config.opt_level,
            target_triple = self.helios_config.target_triple
        );

        let output = Command::new("python3")
            .arg("-c")
            .arg(&python_script)
            .output()
            .map_err(|e| PecosError::Generic(format!("Failed to run Selene Helios: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PecosError::Generic(format!("Selene Helios compilation failed: {}", stderr)));
        }

        let llvm_ir = String::from_utf8(output.stdout)
            .map_err(|e| PecosError::Generic(format!("Invalid UTF-8 output: {}", e)))?;

        log::debug!("Successfully compiled bitcode using Selene Helios from: {}", selene_path.display());
        Ok(llvm_ir.trim().to_string())
    }

    /// Run Selene Helios compilation using conda environment
    fn run_conda_selene_compilation(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let python_script = format!(r#"
import selene_helios_compiler

try:
    with open('{bitcode_path}', 'rb') as f:
        bitcode = f.read()

    llvm_ir = selene_helios_compiler.compile_bitcode_to_llvm_ir(
        bitcode,
        opt_level={opt_level},
        target_triple='{target_triple}'
    )
    print(llvm_ir)
except Exception as e:
    import sys
    print(f"Conda Selene compilation failed: {{e}}", file=sys.stderr)
    sys.exit(1)
"#,
            bitcode_path = bitcode_file.path().display(),
            opt_level = self.helios_config.opt_level,
            target_triple = self.helios_config.target_triple
        );

        let output = Command::new("python3")
            .arg("-c")
            .arg(&python_script)
            .output()
            .map_err(|e| PecosError::Generic(format!("Failed to run conda Selene: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PecosError::Generic(format!("Conda Selene compilation failed: {}", stderr)));
        }

        let llvm_ir = String::from_utf8(output.stdout)
            .map_err(|e| PecosError::Generic(format!("Invalid UTF-8 output: {}", e)))?;

        Ok(llvm_ir.trim().to_string())
    }

    /// Run Selene Helios compilation using system command
    fn run_system_selene_compilation(&self, bitcode_file: &NamedTempFile) -> Result<String, PecosError> {
        let output = Command::new("selene-helios")
            .arg("compile")
            .arg("--input")
            .arg(bitcode_file.path())
            .arg("--output-format")
            .arg("llvm-ir")
            .arg("--opt-level")
            .arg(self.helios_config.opt_level.to_string())
            .arg("--target-triple")
            .arg(&self.helios_config.target_triple)
            .output()
            .map_err(|e| PecosError::Generic(format!("Failed to run system Selene: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PecosError::Generic(format!("System Selene compilation failed: {}", stderr)));
        }

        let llvm_ir = String::from_utf8(output.stdout)
            .map_err(|e| PecosError::Generic(format!("Invalid UTF-8 output: {}", e)))?;

        Ok(llvm_ir.trim().to_string())
    }
}

impl QisInterfaceProvider for QisSeleneHeliosInterface {
    fn get_interface(&mut self) -> Result<QisInterface, PecosError> {
        self.compile_with_helios()
    }

    fn provider_type(&self) -> &'static str {
        "Selene Helios"
    }

    fn can_handle(&self, program_type: &ProgramType) -> bool {
        matches!(program_type, ProgramType::QisBitcode | ProgramType::HugrBytes)
    }

    fn get_metadata(&self) -> std::collections::BTreeMap<String, String> {
        self.metadata.clone()
    }
}

/// Implement IntoQisInterface for QisInterface itself (identity conversion)
impl IntoQisInterface for QisInterface {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        Ok(self)
    }
}

/// Trait for building QisInterface instances from programs
///
/// This trait allows different compilation strategies (JIT, Helios, etc.)
/// to be plugged into the QisEngineBuilder through the .interface() method.
pub trait QisInterfaceBuilder: Send + Sync {
    /// Build a QisInterface from the given program using this builder's strategy
    ///
    /// Since we can't call sized methods on trait objects, each implementation
    /// needs to handle the program type directly
    fn build_from_qis_program(&self, program: QisProgram) -> Result<QisInterface, PecosError>;
    fn build_from_hugr_program(&self, program: HugrProgram) -> Result<QisInterface, PecosError>;
    fn build_from_interface(&self, interface: QisInterface) -> Result<QisInterface, PecosError>;

    /// Get a descriptive name for this builder
    fn name(&self) -> &'static str;
}

/// JIT-based interface builder
#[derive(Debug, Clone)]
pub struct JitInterfaceBuilder;

impl QisInterfaceBuilder for JitInterfaceBuilder {
    fn build_from_qis_program(&self, program: QisProgram) -> Result<QisInterface, PecosError> {
        log::info!("Building QisInterface from QisProgram using JIT compiler");
        program.into_qis_interface_with_jit()
    }

    fn build_from_hugr_program(&self, program: HugrProgram) -> Result<QisInterface, PecosError> {
        log::info!("Building QisInterface from HugrProgram using JIT compiler");
        program.into_qis_interface_with_jit()
    }

    fn build_from_interface(&self, interface: QisInterface) -> Result<QisInterface, PecosError> {
        log::info!("Using pre-built QisInterface");
        Ok(interface)
    }

    fn name(&self) -> &'static str {
        "JIT"
    }
}

/// Selene Helios-based interface builder
#[derive(Debug, Clone)]
pub struct HeliosInterfaceBuilder;

impl QisInterfaceBuilder for HeliosInterfaceBuilder {
    fn build_from_qis_program(&self, program: QisProgram) -> Result<QisInterface, PecosError> {
        log::info!("Building QisInterface from QisProgram using Selene Helios compiler");
        program.into_qis_interface_with_helios()
    }

    fn build_from_hugr_program(&self, program: HugrProgram) -> Result<QisInterface, PecosError> {
        log::info!("Building QisInterface from HugrProgram using Selene Helios compiler");
        program.into_qis_interface_with_helios()
    }

    fn build_from_interface(&self, interface: QisInterface) -> Result<QisInterface, PecosError> {
        log::info!("Using pre-built QisInterface");
        Ok(interface)
    }

    fn name(&self) -> &'static str {
        "Helios"
    }
}


/// Enum to specify which interface builder to use (for backwards compatibility)
#[derive(Debug, Clone)]
pub enum InterfaceChoice {
    /// Use JIT compilation
    Jit,
    /// Use Selene Helios compilation
    Helios,
    /// Auto-select (default to Helios, explicit error if not available)
    Auto,
}

/// Implement IntoQisInterface for QisProgram
///
/// Default uses Helios interface. If not available, returns an error with instructions.
impl IntoQisInterface for QisProgram {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        // Default: Use Helios interface
        match &self.content {
            pecos_programs::QisContent::Ir(ir_text) => {
                log::info!("Converting QisProgram LLVM IR (default: Helios interface)");
                let mut provider = QisSeleneHeliosInterface::from_llvm_ir(ir_text.clone());
                provider.get_interface().map_err(|e| {
                    PecosError::Generic(format!(
                        "Default interface (Selene Helios) failed for LLVM IR: {}\n\n\
                        To fix this:\n\
                        1. Ensure Selene repository is available and built\n\
                        2. Use explicit JIT interface: qis_control_engine().interface(qis_jit_interface()).program(qis_program)\n\
                        3. Or use explicit Helios: qis_control_engine().interface(qis_selene_helios_interface()).program(qis_program)",
                        e
                    ))
                })
            }
            pecos_programs::QisContent::Bitcode(bitcode) => {
                log::info!("Converting QisProgram bitcode (default: Helios)");
                let mut provider = QisSeleneHeliosInterface::from_bitcode(bitcode.clone());
                provider.get_interface().map_err(|e| {
                    PecosError::Generic(format!(
                        "Default interface (Selene Helios) failed for bitcode: {}\n\n\
                        To fix this:\n\
                        1. Ensure Selene repository is available and built\n\
                        2. Use explicit JIT interface: qis_control_engine().interface(qis_jit_interface()).program(qis_program)\n\
                        3. Or use explicit Helios: qis_control_engine().interface(qis_selene_helios_interface()).program(qis_program)",
                        e
                    ))
                })
            }
        }
    }

    fn into_qis_interface_with_jit(self) -> Result<QisInterface, PecosError> {
        match &self.content {
            pecos_programs::QisContent::Ir(ir_text) => {
                log::info!("Converting QisProgram LLVM IR using JIT (explicit)");
                let mut provider = QisJitInterface::from_llvm_ir(ir_text.clone());
                provider.get_interface()
            }
            pecos_programs::QisContent::Bitcode(bitcode) => {
                log::info!("Converting QisProgram bitcode using JIT (explicit)");
                let mut provider = QisJitInterface::from_bitcode(bitcode.clone());
                provider.get_interface()
            }
        }
    }

    fn into_qis_interface_with_helios(self) -> Result<QisInterface, PecosError> {
        match &self.content {
            pecos_programs::QisContent::Ir(ir_text) => {
                log::info!("Converting QisProgram LLVM IR using Helios (explicit)");
                let mut provider = QisSeleneHeliosInterface::from_llvm_ir(ir_text.clone());
                provider.get_interface()
            }
            pecos_programs::QisContent::Bitcode(bitcode) => {
                log::info!("Converting QisProgram bitcode using Helios (explicit)");
                let mut provider = QisSeleneHeliosInterface::from_bitcode(bitcode.clone());
                provider.get_interface()
            }
        }
    }
}

/// Implement IntoQisInterface for HUGR bytes
///
/// Default uses Helios interface (optimized for HUGR). If not available, returns error with instructions.
impl IntoQisInterface for &[u8] {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        // Default to Helios for HUGR as it's optimized for it
        log::info!("Converting HUGR bytes using Selene Helios interface provider (default)");
        let mut provider = QisSeleneHeliosInterface::from_hugr_bytes(self.to_vec());
        provider.get_interface().map_err(|e| {
            PecosError::Generic(format!(
                "Default interface (Selene Helios) failed for HUGR bytes: {}\n\n\
                To fix this:\n\
                1. Ensure Selene repository is available and built\n\
                2. Use explicit interface selection with appropriate builder methods",
                e
            ))
        })
    }
}

/// Implement IntoQisInterface for HUGR bytes (owned)
impl IntoQisInterface for Vec<u8> {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        // Default to Helios for HUGR
        log::info!("Converting HUGR Vec<u8> using Selene Helios interface provider (default)");
        let mut provider = QisSeleneHeliosInterface::from_hugr_bytes(self);
        provider.get_interface().map_err(|e| {
            PecosError::Generic(format!(
                "Default interface (Selene Helios) failed for HUGR Vec<u8>: {}\n\n\
                To fix this:\n\
                1. Ensure Selene repository is available and built\n\
                2. Use explicit interface selection with appropriate builder methods",
                e
            ))
        })
    }
}

/// Implement IntoQisInterface for HugrProgram
///
/// Default uses Helios interface (optimized for HUGR). If not available, returns error with instructions.
impl IntoQisInterface for HugrProgram {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        let hugr_bytes = self.into_bytes();
        // Default to Helios for HUGR
        log::info!("Converting HugrProgram using Selene Helios interface provider (default)");
        let mut provider = QisSeleneHeliosInterface::from_hugr_bytes(hugr_bytes);
        provider.get_interface().map_err(|e| {
            PecosError::Generic(format!(
                "Default interface (Selene Helios) failed for HugrProgram: {}\n\n\
                To fix this:\n\
                1. Ensure Selene repository is available and built\n\
                2. Use explicit interface selection with appropriate builder methods",
                e
            ))
        })
    }
}

/// Wrapper type to represent a QIS Control Engine Program
///
/// This is conceptually equivalent to QisInterface, but provides a
/// more semantically clear type name for the builder API.
#[derive(Debug, Clone)]
pub struct QisControlEngineProgram {
    interface: QisInterface,
}

impl QisControlEngineProgram {
    /// Create a new program from a QisInterface
    pub fn new(interface: QisInterface) -> Self {
        Self { interface }
    }

    /// Create a program from anything that can be converted to QisInterface
    ///
    /// # Errors
    /// Returns an error if the conversion fails
    pub fn from_program<P: IntoQisInterface>(program: P) -> Result<Self, PecosError> {
        let interface = program.into_qis_interface()?;
        Ok(Self::new(interface))
    }

    /// Get the underlying QisInterface
    pub fn into_interface(self) -> QisInterface {
        self.interface
    }

    /// Get a reference to the underlying QisInterface
    pub fn interface(&self) -> &QisInterface {
        &self.interface
    }
}

impl IntoQisInterface for QisControlEngineProgram {
    fn into_qis_interface(self) -> Result<QisInterface, PecosError> {
        Ok(self.interface)
    }
}

impl From<QisInterface> for QisControlEngineProgram {
    fn from(interface: QisInterface) -> Self {
        Self::new(interface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qis_interface_identity_conversion() {
        let interface = QisInterface::new();
        let result = interface.clone().into_qis_interface().unwrap();
        // Basic check that conversion preserves structure
        assert_eq!(result.allocated_qubits, interface.allocated_qubits);
    }

    #[test]
    fn test_qis_control_engine_program_wrapper() {
        let interface = QisInterface::new();
        let program = QisControlEngineProgram::new(interface.clone());

        let back = program.into_interface();
        assert_eq!(back.allocated_qubits, interface.allocated_qubits);
    }

    #[test]
    fn test_qis_program_conversion_basic() {
        // Test with a simple Bell state QIS program using explicit JIT interface for testing
        let bell_llvm = r#"
            define void @main() {
                call void @__quantum__qis__h__body(i64 0)
                call void @__quantum__qis__cx__body(i64 0, i64 1)
                %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
                %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
                ret void
            }

            declare void @__quantum__qis__h__body(i64)
            declare void @__quantum__qis__cx__body(i64, i64)
            declare i32 @__quantum__qis__m__body(i64, i64)
        "#;

        let qis_program = QisProgram::from_string(bell_llvm);
        // Use explicit JIT interface for reliable testing (no external dependencies)
        let ir = qis_program.ir().expect("Should have IR content").to_string();
        let mut jit_provider = QisJitInterface::from_llvm_ir(ir);
        let result = jit_provider.get_interface();
        assert!(result.is_ok(), "Conversion should succeed: {:?}", result);

        let interface = result.unwrap();
        assert_eq!(interface.allocated_qubits.len(), 2, "Should have 2 qubits");
        assert_eq!(interface.allocated_results.len(), 2, "Should have 2 result slots");
        assert_eq!(interface.operations.len(), 4, "Should have 4 operations");
    }

    #[test]
    fn test_qis_program_conversion_empty() {
        let qis_program = QisProgram::from_string("define void @main() { ret void }");
        // Use explicit JIT interface for reliable testing (no external dependencies)
        let ir = qis_program.ir().expect("Should have IR content").to_string();
        let mut jit_provider = QisJitInterface::from_llvm_ir(ir);
        let result = jit_provider.get_interface();
        assert!(result.is_ok());

        let interface = result.unwrap();
        assert_eq!(interface.allocated_qubits.len(), 0);
        assert_eq!(interface.operations.len(), 0);
    }
}


/// Compile HUGR bytes using Selene's compiler
///
/// This uses Selene's proven HUGR→LLVM compiler, ensuring proper qubit ID
/// management and QIS function generation. Returns explicit error if Selene is not available.
fn compile_hugr_with_selene(hugr_bytes: &[u8]) -> Result<String, PecosError> {
    log::info!("Compiling HUGR with Selene compiler (required)");

    // Use Selene's Python compiler - no fallbacks
    compile_hugr_with_selene_python(hugr_bytes)
        .map_err(|e| {
            PecosError::Generic(format!(
                "Selene Helios compilation failed: {}\n\n\
                To use Helios interface, ensure Selene is installed and available:\n\
                1. Ensure Selene repository is at ../selene or ../../../selene\n\
                2. Build Selene compilers: 'cargo build --release' in Selene directory\n\
                3. Or use explicit JIT interface: qis_control_engine().interface(qis_jit_interface()).program()",
                e
            ))
        })
}

/// Compile HUGR using Selene's Python compiler
fn compile_hugr_with_selene_python(hugr_bytes: &[u8]) -> Result<String, PecosError> {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Write HUGR bytes to a temporary file
    let mut hugr_file = NamedTempFile::new()
        .map_err(|e| PecosError::Generic(format!("Failed to create temp file: {}", e)))?;
    hugr_file.write_all(hugr_bytes)
        .map_err(|e| PecosError::Generic(format!("Failed to write HUGR bytes: {}", e)))?;

    // Call Selene's compiler using Python
    let output = Command::new("python3")
        .arg("-c")
        .arg(format!(r#"
import sys
sys.path.insert(0, '{}/selene-compilers/hugr_qis/python')
from selene_hugr_qis_compiler import compile_to_llvm_ir

with open('{}', 'rb') as f:
    hugr_bytes = f.read()

llvm_ir = compile_to_llvm_ir(hugr_bytes, opt_level=2, target_triple='native')
print(llvm_ir)
"#,
            "/home/ciaranra/Repos/cl_projects/gup/selene",
            hugr_file.path().display()))
        .output()
        .map_err(|e| PecosError::Generic(format!("Failed to run Selene compiler: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PecosError::Generic(format!("Selene compiler failed: {}", stderr)));
    }

    let llvm_ir = String::from_utf8(output.stdout)
        .map_err(|e| PecosError::Generic(format!("Invalid UTF-8 output: {}", e)))?;

    log::debug!("Successfully compiled HUGR using Selene compiler");
    Ok(llvm_ir)
}


