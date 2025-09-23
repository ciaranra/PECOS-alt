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
//! // Run simulation using the new API
//! let results = sim(QasmProgram::from_string(qasm_str))
//!     .seed(42)
//!     .run(1000)
//!     .unwrap();
//!
//! // Results contains measurement outcomes for each shot
//! println!("Simulation results: {:?}", results);
//! ```
//!
//! ## Features
//!
//! PECOS supports a variety of noise models and quantum simulators. Check the documentation
//! for the simulation builders and noise models for more details on the available options.

pub mod engine_type;
pub mod prelude;
pub mod program;
pub mod unified_sim;

pub use engine_type::{DynamicEngineBuilder, EngineType, sim_dynamic};
pub use pecos_engines::{
    DepolarizingNoise, GeneralNoiseModelBuilder, PassThroughNoiseModel, SimInput, sim_builder,
    sparse_stabilizer, state_vector,
};
pub use pecos_llvm_runtime::LlvmEngineConfig;
pub use pecos_qasm::run_qasm;
pub use unified_sim::{ProgrammedSimBuilder, SimBuilderExt, sim};

// Re-export program types from pecos-programs
pub use pecos_programs::{HugrProgram, LlvmProgram, Program, QasmProgram};

// Re-export engine builders from individual crates
#[cfg(feature = "qasm")]
pub use pecos_qasm::qasm_engine;

#[cfg(feature = "llvm")]
pub use pecos_llvm_sim::llvm_engine;

#[cfg(feature = "selene")]
pub use pecos_selene_engine::selene_executable_builder::selene_executable;

#[cfg(feature = "phir")]
pub use pecos_phir_json::phir_json_engine;

use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngine;
use std::path::Path;

/// Set up a generic LLVM engine for executing quantum programs
///
/// This function creates a mock engine for Selene QIS LLVM IR.
/// The LLVM IR should use the Selene QIS format (with functions like
/// ___qalloc, ___rzz, ___h, etc.)
///
/// Note: This is a simplified implementation that creates a mock engine
/// for testing purposes. Full Selene QIS execution is not yet implemented.
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
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    log::debug!("Setting up LLVM engine for: {}", llvm_ir_path.display());

    // Read the LLVM IR to detect number of qubits
    let llvm_ir = std::fs::read_to_string(llvm_ir_path)
        .map_err(|e| PecosError::IO(e))?;

    // Check for qubit usage - both Selene style (___qalloc) and QIR style (direct indices)
    let selene_qubits = llvm_ir.matches("___qalloc").count();
    let has_qir_gates = llvm_ir.contains("__quantum__qis__");

    if selene_qubits == 0 && !has_qir_gates {
        return Err(PecosError::Generic(
            "No quantum operations found in LLVM IR".to_string()
        ));
    }

    let num_qubits_hint = if selene_qubits > 0 {
        log::debug!("Detected {} qubits from ___qalloc calls", selene_qubits);
        Some(selene_qubits)
    } else {
        log::debug!("Detected QIR-style program with implicit qubit allocation");
        // For QIR programs, scan for the maximum qubit index used
        let mut max_index = 0;

        // Simple pattern matching for QIR gate calls
        // Look for patterns like: __quantum__qis__*__body(... i64 N ...)
        for line in llvm_ir.lines() {
            if line.contains("__quantum__qis__") && line.contains("__body") {
                // Find i64 arguments
                if let Some(args_start) = line.find('(') {
                    let args = &line[args_start..];
                    // Split by commas and look for i64 values
                    for arg in args.split(',') {
                        if let Some(i64_pos) = arg.find("i64 ") {
                            let num_str = &arg[i64_pos + 4..].trim_end_matches(')').trim();
                            if let Ok(idx) = num_str.parse::<usize>() {
                                max_index = max_index.max(idx);
                            }
                        }
                    }
                }
            }
        }

        // Need at least max_index + 1 qubits
        let needed_qubits = max_index + 1;
        log::debug!("QIR program uses qubits 0 to {}, need {} qubits", max_index, needed_qubits);
        Some(needed_qubits.max(2)) // At least 2 qubits for safety
    };

    // Use the actual LLVM engine with Selene QIS runtime
    setup_llvm_engine_with_config(llvm_ir_path, shots, num_qubits_hint)
}

/// Create a mock engine for Selene QIS testing
#[allow(dead_code)]
fn create_mock_selene_qis_engine(
    num_qubits: usize,
    _shots: Option<usize>,
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    use pecos_engines::{ByteMessage, ClassicalEngine, ControlEngine, Data, Engine, EngineStage, Shot};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Use a global atomic counter to ensure variation across clones
    static GLOBAL_SHOT_COUNT: AtomicUsize = AtomicUsize::new(0);

    /// Mock engine that returns Bell state-like results for testing
    #[derive(Clone)]
    struct MockSeleneQisEngine {
        num_qubits: usize,
    }

    impl Engine for MockSeleneQisEngine {
        type Input = ();
        type Output = Shot;

        fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
            // Return a mock shot with correlated results
            let mut data = BTreeMap::new();

            // Get and increment global shot count for variation
            let shot_id = GLOBAL_SHOT_COUNT.fetch_add(1, Ordering::SeqCst);

            // Use a simple hash-based pseudo-random to get reproducible but varied results
            // This ensures we get roughly 50/50 distribution
            let hash = (shot_id as i64 * 2654435761) & 1;

            let outcome = if self.num_qubits == 1 {
                // For single qubit (Hadamard), return 0 or 1
                hash
            } else {
                // For multiple qubits, simulate entangled state: |00...0> or |11...1>
                if hash == 0 {
                    0i64  // |00...0>
                } else {
                    (1i64 << self.num_qubits) - 1  // |11...1>
                }
            };
            data.insert("result".to_string(), Data::I64(outcome));

            Ok(Shot { data })
        }

        fn reset(&mut self) -> Result<(), PecosError> {
            // No local state to reset, using global counter
            Ok(())
        }
    }

    impl ClassicalEngine for MockSeleneQisEngine {
        fn num_qubits(&self) -> usize {
            self.num_qubits
        }

        fn compile(&self) -> Result<(), PecosError> {
            Ok(())
        }

        fn reset(&mut self) -> Result<(), PecosError> {
            // No local state to reset, using global counter
            Ok(())
        }

        fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
            // Return empty commands for mock engine
            Ok(ByteMessage::builder().build())
        }

        fn handle_measurements(&mut self, _message: ByteMessage) -> Result<(), PecosError> {
            // No-op for mock engine
            Ok(())
        }

        fn get_results(&self) -> Result<Shot, PecosError> {
            // Return a shot with pseudo-random results
            let mut data = BTreeMap::new();

            // Get current global count for consistent results
            let shot_id = GLOBAL_SHOT_COUNT.load(Ordering::SeqCst);
            let hash = (shot_id as i64 * 2654435761) & 1;

            let outcome = if self.num_qubits == 1 {
                hash
            } else {
                if hash == 0 {
                    0i64
                } else {
                    (1i64 << self.num_qubits) - 1
                }
            };
            data.insert("result".to_string(), Data::I64(outcome));
            Ok(Shot { data })
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    impl ControlEngine for MockSeleneQisEngine {
        type Input = ();
        type Output = Shot;
        type EngineInput = ByteMessage;
        type EngineOutput = ByteMessage;

        fn start(&mut self, _input: Self::Input) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
            // Return a completed stage with a mock shot
            let shot = self.process(())?;
            Ok(EngineStage::Complete(shot))
        }

        fn continue_processing(
            &mut self,
            _message: ByteMessage,
        ) -> Result<EngineStage<ByteMessage, Shot>, PecosError> {
            let shot = self.process(())?;
            Ok(EngineStage::Complete(shot))
        }

        fn reset(&mut self) -> Result<(), PecosError> {
            // No local state to reset, using global counter
            Ok(())
        }
    }

    let engine = MockSeleneQisEngine {
        num_qubits,
    };

    Ok(Box::new(engine))
}

/// Set up a generic LLVM engine with max qubits configuration
///
/// This function creates a mock engine for Selene QIS LLVM IR with
/// the ability to specify the maximum number of qubits.
///
/// Note: This is a simplified implementation that creates a mock engine
/// for testing purposes. Full Selene QIS execution is not yet implemented.
///
/// # Arguments
/// * `llvm_ir_path` - Path to the LLVM IR file
/// * `shots` - Optional number of shots to assign to the engine
/// * `max_qubits` - Optional maximum number of qubits (ignored if qubits can be detected from IR)
///
/// # Returns
/// A boxed `ClassicalEngine` ready for execution
///
/// # Errors
/// Returns `PecosError` if engine creation fails
pub fn setup_llvm_engine_with_config(
    llvm_ir_path: &Path,
    _shots: Option<usize>,
    max_qubits: Option<usize>,
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    log::debug!(
        "Setting up LLVM engine for: {} with max_qubits: {:?}",
        llvm_ir_path.display(),
        max_qubits
    );

    // Read the LLVM IR to detect number of qubits
    let llvm_ir = std::fs::read_to_string(llvm_ir_path)
        .map_err(|e| PecosError::IO(e))?;

    // Count the number of qubits from ___qalloc calls
    let detected_qubits = llvm_ir.matches("___qalloc").count();

    // Use detected qubits if available, otherwise fall back to max_qubits
    let num_qubits = if detected_qubits > 0 {
        detected_qubits
    } else if let Some(max) = max_qubits {
        max
    } else {
        return Err(PecosError::Generic(
            "No qubits detected and max_qubits not specified".to_string()
        ));
    };

    log::debug!("Using {} qubits for engine", num_qubits);

    // Create an actual LLVM engine with the Selene QIS runtime
    use pecos_llvm_runtime::LlvmEngine;
    let engine = LlvmEngine::new(llvm_ir_path.to_path_buf());
    Ok(Box::new(engine) as Box<dyn ClassicalControlEngine>)
}

/// HUGR-LLVM Integration
///
/// This module provides thin orchestration functions that combine HUGR compilation
/// with LLVM execution. The architecture is:
///
/// 1. `pecos-hugr-qis` compiles HUGR → LLVM IR (pure compilation, no engine dependencies)
/// 2. `pecos-llvm-runtime` executes LLVM IR (pure execution, no HUGR dependencies)
/// 3. `pecos` orchestrates: HUGR → LLVM IR → Execution
pub mod hugr {
    use pecos_core::errors::PecosError;
    use pecos_engines::ClassicalControlEngine;
    use std::path::Path;

    /// Compile and run a HUGR file with default settings
    ///
    /// This is a convenience function that:
    /// 1. Creates a SeleneExecutableEngine
    /// 2. Loads the HUGR program into the engine
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
    ) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
        // Read the HUGR file
        let hugr_bytes = std::fs::read(hugr_path.as_ref())
            .map_err(|e| PecosError::IO(e))?;

        // Use the bytes version
        run_hugr_llvm_from_bytes(&hugr_bytes, shots)
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
    ) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
        // Compile HUGR to LLVM IR
        let llvm_ir = pecos_hugr_qis::compile_hugr_bytes_to_string(hugr_bytes)?;

        // Count qubits from the LLVM IR for validation
        let num_qubits = llvm_ir.matches("___qalloc").count();
        if num_qubits == 0 {
            return Err(PecosError::Generic(
                "No qubits allocated in compiled HUGR".to_string()
            ));
        }

        // Create actual LLVM engine from the compiled IR
        create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }

    /// Create an LLVM engine from LLVM IR file
    fn create_llvm_engine_from_ir(
        llvm_ir_path: &Path,
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
        crate::setup_llvm_engine(llvm_ir_path, shots)
    }

    /// Create an LLVM engine from LLVM IR string
    pub(crate) fn create_llvm_engine_from_ir_string(
        llvm_ir: &str,
        shots: Option<usize>,
    ) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
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
    pub use pecos_phir::PhirConfig;
    // PHIR compilation functions temporarily disabled - needs HUGR 0.22 update
    // use pecos_phir::{compile_hugr_bytes_via_phir, compile_hugr_via_phir as compile_hugr_phir};

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
    // PHIR functions temporarily disabled - needs HUGR 0.22 update
    /*
    pub fn run_phir_llvm<P: AsRef<Path>>(
        hugr_path: P,
        shots: Option<usize>,
        config: Option<PhirConfig>,
    ) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
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
    */

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
    /*
    pub fn run_phir_llvm_from_string(
        hugr_json: &str,
        shots: Option<usize>,
        config: Option<PhirConfig>,
    ) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
        // Use provided config or default
        let config = config.unwrap_or_default();

        // Step 1: Compile HUGR to LLVM IR via PHIR
        let llvm_ir = compile_hugr_phir(hugr_json, &config).map_err(convert_phir_error)?;

        // Step 2: Create LLVM engine from the IR string
        super::hugr::create_llvm_engine_from_ir_string(&llvm_ir, shots)
    }
    */

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
    /*
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
    */

    // Re-export types for convenience (PhirConfig already imported above)
    // pub use pecos_phir::hugr_to_phir_mlir; // Temporarily disabled

    // Error conversion helper function
    #[allow(dead_code)]
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
