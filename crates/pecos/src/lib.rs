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
//! - `pecos_phir_json`: PECOS High-level Intermediate Representation
//! - `pecos_llvm_runtime`: Support for Quantum Intermediate Representation
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
/// A boxed `ClassicalEngine` ready for execution
///
/// # Errors
/// Returns `PecosError` if engine creation fails
pub fn setup_llvm_engine(
    llvm_ir_path: &Path,
    shots: Option<usize>,
) -> Result<Box<dyn ClassicalEngine>, PecosError> {
    log::debug!("Setting up LLVM engine for: {}", llvm_ir_path.display());

    // Create a generic LLVM engine from the path
    let mut engine = pecos_llvm_runtime::LlvmEngine::new(llvm_ir_path.to_path_buf());

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
/// 2. `pecos-llvm-runtime` executes LLVM IR (pure execution, no HUGR dependencies)
/// 3. `pecos` orchestrates: HUGR → LLVM IR → Execution
pub mod hugr {
    use pecos_core::errors::PecosError;
    use pecos_engines::ClassicalEngine;
    use std::path::Path;

    /// Compile and run a HUGR file with default settings
    ///
    /// This is a convenience function that:
    /// 1. Compiles HUGR to LLVM IR using `pecos-hugr-llvm`
    /// 2. Creates an LLVM engine from the IR using `pecos-llvm-runtime`
    /// 3. Returns the configured engine ready for execution
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file
    /// * `shots` - Optional number of shots to assign to the engine
    ///
    /// # Returns
    /// A boxed `ClassicalEngine` ready for execution
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
    /// A boxed `ClassicalEngine` ready for execution
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
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Write IR to temporary file - use persist to keep it around
        let mut temp_file = NamedTempFile::new().map_err(|e| {
            PecosError::with_context(e, "Failed to create temporary file for LLVM IR")
        })?;

        temp_file.write_all(llvm_ir.as_bytes()).map_err(|e| {
            PecosError::with_context(e, "Failed to write LLVM IR to temporary file")
        })?;

        // Persist the file to keep it around after this function returns
        // This is necessary because the LlvmEngine needs to access the file in worker threads
        let (_, path) = temp_file
            .keep()
            .map_err(|e| PecosError::with_context(e, "Failed to persist temporary LLVM IR file"))?;

        // Create engine from temporary file
        create_llvm_engine_from_ir(&path, shots)
    }
}

/// PHIR (PECOS High-level Intermediate Representation) Integration
///
/// This module provides thin orchestration functions that combine HUGR compilation
/// through the PHIR pipeline with LLVM execution. The architecture is:
///
/// 1. `pecos-phir` compiles HUGR → PHIR → LLVM IR (direct parsing to PHIR, no separate AST)
/// 2. `pecos-llvm-runtime` executes LLVM IR (pure execution, no HUGR/PHIR dependencies)
/// 3. `pecos` orchestrates: HUGR → PHIR → LLVM IR → Execution
///
/// PHIR provides an alternative compilation path to HUGR-LLVM that goes through
/// MLIR-based optimizations and transformations.
pub mod phir {
    use pecos_core::errors::PecosError;
    use pecos_engines::ClassicalEngine;
    pub use pecos_phir::PhirConfig;
    use pecos_phir::{compile_hugr_bytes_via_phir, compile_hugr_via_phir as compile_hugr_phir};
    use std::path::Path;

    /// Compile and run a HUGR file via PHIR with default settings
    ///
    /// This is a convenience function that:
    /// 1. Compiles HUGR to LLVM IR via PHIR using `pecos-phir`
    /// 2. Creates an LLVM engine from the IR using `pecos-llvm-runtime`
    /// 3. Returns the configured engine ready for execution
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file
    /// * `shots` - Optional number of shots to assign to the engine
    /// * `config` - Optional PHIR configuration (uses defaults if None)
    ///
    /// # Returns
    /// A boxed `ClassicalEngine` ready for execution
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation via PHIR or engine creation fails
    pub fn run_phir_llvm<P: AsRef<Path>>(
        hugr_path: P,
        shots: Option<usize>,
        config: Option<PhirConfig>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        // Read HUGR file (could be binary or JSON format)
        let hugr_bytes = std::fs::read(hugr_path.as_ref()).map_err(|e| {
            PecosError::with_context(
                e,
                format!("Failed to read HUGR file: {}", hugr_path.as_ref().display()),
            )
        })?;

        // Use provided config or default
        let config = config.unwrap_or_default();

        // Compile via PHIR (handles both binary and JSON formats)
        let llvm_ir =
            compile_hugr_bytes_via_phir(&hugr_bytes, &config).map_err(convert_phir_error)?;

        // Create LLVM engine from the IR string
        super::hugr::create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }

    /// Compile HUGR JSON string to LLVM IR via PHIR and create an engine
    ///
    /// # Arguments
    /// * `hugr_json` - HUGR data as JSON string
    /// * `shots` - Optional number of shots to assign to the engine
    /// * `config` - Optional PHIR configuration (uses defaults if None)
    ///
    /// # Returns
    /// A boxed `ClassicalEngine` ready for execution
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation via PHIR or engine creation fails
    pub fn run_phir_llvm_from_string(
        hugr_json: &str,
        shots: Option<usize>,
        config: Option<PhirConfig>,
    ) -> Result<Box<dyn ClassicalEngine>, PecosError> {
        // Use provided config or default
        let config = config.unwrap_or_default();

        // Step 1: Compile HUGR to LLVM IR via PHIR
        let llvm_ir = compile_hugr_phir(hugr_json, &config).map_err(convert_phir_error)?;

        // Step 2: Create LLVM engine from the IR string
        super::hugr::create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }

    /// Compile HUGR file to LLVM IR via PHIR (without creating an engine)
    ///
    /// This function only performs compilation and returns the LLVM IR string.
    /// Useful when you need the compiled output but don't want to create an engine.
    ///
    /// # Arguments
    /// * `hugr_path` - Path to the HUGR file
    /// * `config` - Optional PHIR configuration (uses defaults if None)
    ///
    /// # Returns
    /// LLVM IR as a string
    ///
    /// # Errors
    /// Returns `PecosError` if HUGR compilation via PHIR fails
    pub fn compile_hugr_file_via_phir<P: AsRef<Path>>(
        hugr_path: P,
        config: Option<PhirConfig>,
    ) -> Result<String, PecosError> {
        // Read HUGR file (could be binary or JSON format)
        let hugr_bytes = std::fs::read(hugr_path.as_ref()).map_err(|e| {
            PecosError::with_context(
                e,
                format!("Failed to read HUGR file: {}", hugr_path.as_ref().display()),
            )
        })?;

        // Use provided config or default
        let config = config.unwrap_or_default();

        // Compile via PHIR (handles both binary and JSON formats)
        let llvm_ir =
            compile_hugr_bytes_via_phir(&hugr_bytes, &config).map_err(convert_phir_error)?;
        Ok(llvm_ir)
    }

    // Re-export types for convenience (PhirConfig already imported above)
    pub use pecos_phir::hugr_to_phir_mlir;

    // Error conversion helper function
    fn convert_phir_error(error: pecos_phir::PhirError) -> PecosError {
        // Convert PhirError to PecosError using appropriate category
        match error {
            pecos_phir::PhirError::Parse(_) => PecosError::ParseSyntax {
                language: "PHIR".to_string(),
                message: error.to_string(),
            },
            pecos_phir::PhirError::Type(_) | pecos_phir::PhirError::Validation(_) => {
                PecosError::ValidationInvalidCircuitStructure(error.to_string())
            }
            pecos_phir::PhirError::Runtime(_) => PecosError::Processing(error.to_string()),
            pecos_phir::PhirError::Compilation(_) => PecosError::Compilation(error.to_string()),
            pecos_phir::PhirError::IO(msg) => PecosError::IO(std::io::Error::other(msg)),
            pecos_phir::PhirError::Internal(msg) => {
                PecosError::Generic(format!("Internal PHIR error: {msg}"))
            }
        }
    }
}
