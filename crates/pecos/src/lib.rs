// Copyright 2024 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! # PECOS: Practical Error Correction Optimizing Simulator
//!
//! PECOS is a quantum error correction simulation framework that provides tools for
//! designing, testing, and evaluating quantum error correction codes and protocols.
//!
//! ## Crate Structure
//!
//! PECOS is organized as a meta-crate that brings together several component crates:
//!
//! - `pecos_core`: Core types, traits, and utilities used across PECOS
//! - `pecos_engines`: Simulation engines for quantum and classical processing
//! - `pecos_qasm`: Support for `OpenQASM` language for quantum circuit description
//! - `pecos_qsim`: Quantum simulation implementations
//! - `pecos_phir`: PECOS High-level Intermediate Representation
//! - `pecos_qir`: Support for Quantum Intermediate Representation
//!
//! This meta-crate unifies the API and re-exports the most commonly used types and
//! functions from the component crates to provide a simplified interface.
//!
//! ## Using the Prelude
//!
//! PECOS provides a prelude module that re-exports the most commonly used types and traits.
//! To use it, add the following import to your code:
//!
//! ```rust
//! use pecos::prelude::*;
//! ```
//!
//! This will bring all the essential PECOS types and traits into scope, making it easier to
//! write PECOS code without numerous import statements.
//!
//! ### Component Crate Preludes
//!
//! When writing tests or documentation for the individual component crates, you should
//! import from the component's own prelude to avoid circular dependencies:
//!
//! ```
//! // In pecos-qasm tests or examples:
//! use pecos_qasm::prelude::*;
//! ```
//!
//! ## Example Usage
//!
//! Here's a simple example of running a quantum circuit simulation using PECOS:
//!
//! ```rust,no_run
//! use pecos::prelude::*;
//!
//! // Bell state in OpenQASM
//! let qasm_str = r#"
//! OPENQASM 2.0;
//! include "qelib1.inc";
//! qreg q[2];
//! creg c[2];
//! h q[0];
//! cx q[0], q[1];
//! measure q -> c;
//! "#;
//!
//! // Run simulation with default settings (no noise, state vector simulator)
//! let program = QASMProgram::from_str(qasm_str).unwrap();
//! let results = run_sim(program.into_engine_box(), 1000, Some(42), None, None, None).unwrap();
//!
//! // Results contains measurement outcomes for each shot
//! println!("Simulation results: {:?}", results);
//! ```
//!
//! ## Features
//!
//! PECOS supports a variety of noise models and quantum simulators. Check the documentation
//! for `run_qasm_with_options` and `NoiseModelType` for more details on the available options.

pub mod prelude;
pub mod program;

pub use pecos_qasm::run_qasm_sim;

use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::path::Path;

/// Set up a generic LLVM engine for executing quantum programs
///
/// This function creates an LLVM engine from an LLVM IR file.
/// It's format-agnostic and can execute any valid LLVM IR that follows
/// quantum runtime conventions.
///
/// # Arguments
/// * `llvm_ir_path` - Path to the LLVM IR file
/// * `shots` - Optional number of shots to assign to the engine
///
/// # Returns
/// A boxed ClassicalEngine ready for execution
///
/// # Errors
/// Returns `PecosError` if engine creation fails
pub fn setup_llvm_engine(
    llvm_ir_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    log::debug!("Setting up LLVM engine for: {}", llvm_ir_path.display());

    // Create a generic LLVM engine from the path
    let mut engine = pecos_qir::LlvmEngine::new(llvm_ir_path.to_path_buf());

    // Set the number of shots if specified
    if let Some(num_shots) = shots {
        engine.set_assigned_shots(num_shots);
    }

    // Pre-compile the LLVM library for efficient execution
    engine.pre_compile()?;

    Ok(Box::new(engine))
}

/// HUGR-LLVM Integration
///
/// This module provides thin orchestration functions that combine HUGR compilation
/// with LLVM execution. The architecture is:
///
/// 1. `pecos-hugr-llvm` compiles HUGR → LLVM IR (pure compilation, no engine dependencies)
/// 2. `pecos-qir` executes LLVM IR (pure execution, no HUGR dependencies)
/// 3. `pecos` orchestrates: HUGR → LLVM IR → Execution
pub mod hugr {
    use pecos_core::errors::PecosError;
    use pecos_engines::ClassicalEngine;
    use std::path::Path;

    /// Compile and run a HUGR file with default settings
    ///
    /// This is a convenience function that:
    /// 1. Compiles HUGR to LLVM IR using `pecos-hugr-llvm`
    /// 2. Creates an LLVM engine from the IR using `pecos-qir`
    /// 3. Returns the configured engine ready for execution
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file
    /// * `shots` - Optional number of shots to assign to the engine
    ///
    /// # Returns
    /// A boxed ClassicalEngine ready for execution
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation or engine creation fails
    pub fn run_hugr_llvm<P: AsRef<Path>>(
        hugr_path: P,
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        // Step 1: Compile HUGR to LLVM IR
        let llvm_ir_path = pecos_hugr_llvm::compile_hugr_to_llvm(hugr_path, None)?;
        
        // Step 2: Create LLVM engine from the generated IR
        create_llvm_engine_from_ir(&llvm_ir_path, shots)
    }

    /// Compile HUGR bytes to LLVM IR and create an engine
    ///
    /// # Arguments
    /// * `hugr_bytes` - HUGR data as bytes
    /// * `shots` - Optional number of shots to assign to the engine
    ///
    /// # Returns
    /// A boxed ClassicalEngine ready for execution
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation or engine creation fails
    pub fn run_hugr_llvm_from_bytes(
        hugr_bytes: &[u8],
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        // Step 1: Compile HUGR bytes to LLVM IR string
        let llvm_ir = pecos_hugr_llvm::compile_hugr_bytes_to_string(hugr_bytes)?;
        
        // Step 2: Create LLVM engine from the IR string
        create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }

    /// Create an LLVM engine from LLVM IR file
    fn create_llvm_engine_from_ir(
        llvm_ir_path: &Path,
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        crate::setup_llvm_engine(llvm_ir_path, shots)
    }

    /// Create an LLVM engine from LLVM IR string
    pub(crate) fn create_llvm_engine_from_ir_string(
        llvm_ir: &str,
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        // Write IR to temporary file - use persist to keep it around
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| PecosError::with_context(e, "Failed to create temporary file for LLVM IR"))?;
        
        temp_file.write_all(llvm_ir.as_bytes())
            .map_err(|e| PecosError::with_context(e, "Failed to write LLVM IR to temporary file"))?;
        
        // Persist the file to keep it around after this function returns
        // This is necessary because the LlvmEngine needs to access the file in worker threads
        let (_, path) = temp_file.keep()
            .map_err(|e| PecosError::with_context(e, "Failed to persist temporary LLVM IR file"))?;
        
        // Create engine from temporary file
        create_llvm_engine_from_ir(&path, shots)
    }
}

/// PMIR (PECOS MLIR) Integration
///
/// This module provides thin orchestration functions that combine HUGR compilation
/// through the PMIR pipeline with LLVM execution. The architecture is:
///
/// 1. `pecos-pmir` compiles HUGR → PAST → PMIR → LLVM IR (pure compilation, no engine dependencies)
/// 2. `pecos-qir` executes LLVM IR (pure execution, no HUGR/PMIR dependencies)
/// 3. `pecos` orchestrates: HUGR → PMIR → LLVM IR → Execution
///
/// PMIR provides an alternative compilation path to HUGR-LLVM that goes through
/// MLIR-based optimizations and transformations.
#[cfg(feature = "pmir-pipeline")]
pub mod pmir {
    use pecos_core::errors::PecosError;
    use pecos_engines::ClassicalEngine;
    use pecos_pmir::{compile_hugr_via_pmir as compile_hugr_pmir, compile_hugr_bytes_via_pmir};
    pub use pecos_pmir::PmirConfig;
    use std::path::Path;

    /// Compile and run a HUGR file via PMIR with default settings
    ///
    /// This is a convenience function that:
    /// 1. Compiles HUGR to LLVM IR via PMIR using `pecos-pmir`
    /// 2. Creates an LLVM engine from the IR using `pecos-qir`
    /// 3. Returns the configured engine ready for execution
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file
    /// * `shots` - Optional number of shots to assign to the engine
    /// * `config` - Optional PMIR configuration (uses defaults if None)
    ///
    /// # Returns
    /// A boxed ClassicalEngine ready for execution
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation via PMIR or engine creation fails
    pub fn run_pmir_llvm<P: AsRef<Path>>(
        hugr_path: P,
        shots: Option<usize>,
        config: Option<PmirConfig>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        // Read HUGR file (could be binary or JSON format)
        let hugr_bytes = std::fs::read(hugr_path.as_ref())
            .map_err(|e| PecosError::with_context(e, 
                format!("Failed to read HUGR file: {}", hugr_path.as_ref().display())))?;
        
        // Use provided config or default
        let config = config.unwrap_or_default();
        
        // Compile via PMIR (handles both binary and JSON formats)
        let llvm_ir = compile_hugr_bytes_via_pmir(&hugr_bytes, &config)?;
        
        // Create LLVM engine from the IR string
        super::hugr::create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }

    /// Compile HUGR JSON string to LLVM IR via PMIR and create an engine
    ///
    /// # Arguments
    /// * `hugr_json` - HUGR data as JSON string
    /// * `shots` - Optional number of shots to assign to the engine
    /// * `config` - Optional PMIR configuration (uses defaults if None)
    ///
    /// # Returns
    /// A boxed ClassicalEngine ready for execution
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation via PMIR or engine creation fails
    pub fn run_pmir_llvm_from_string(
        hugr_json: &str,
        shots: Option<usize>,
        config: Option<PmirConfig>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        // Use provided config or default
        let config = config.unwrap_or_default();
        
        // Step 1: Compile HUGR to LLVM IR via PMIR
        let llvm_ir = compile_hugr_pmir(hugr_json, &config)?;
        
        // Step 2: Create LLVM engine from the IR string
        super::hugr::create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }

    /// Compile HUGR file to LLVM IR via PMIR (without creating an engine)
    ///
    /// This function only performs compilation and returns the LLVM IR string.
    /// Useful when you need the compiled output but don't want to create an engine.
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file
    /// * `config` - Optional PMIR configuration (uses defaults if None)
    ///
    /// # Returns
    /// LLVM IR as a string
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation via PMIR fails
    pub fn compile_hugr_file_via_pmir<P: AsRef<Path>>(
        hugr_path: P,
        config: Option<PmirConfig>,
    ) -> Result<String, PecosError> {
        // Read HUGR file (could be binary or JSON format)
        let hugr_bytes = std::fs::read(hugr_path.as_ref())
            .map_err(|e| PecosError::with_context(e, 
                format!("Failed to read HUGR file: {}", hugr_path.as_ref().display())))?;
        
        // Use provided config or default
        let config = config.unwrap_or_default();
        
        // Compile via PMIR (handles both binary and JSON formats)
        compile_hugr_bytes_via_pmir(&hugr_bytes, &config)
    }

    // Re-export types for convenience (PmirConfig already imported above)
    pub use pecos_pmir::{hugr_to_past_ron, hugr_to_pmir_mlir, past_ron_to_pmir_mlir};
}
