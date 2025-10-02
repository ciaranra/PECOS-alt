//! # PECOS - Performance Estimator of Codes On Surfaces
//!
//! PECOS is a framework for simulating and evaluating quantum error correction codes.
//! It provides a comprehensive set of tools for quantum simulation, noise modeling,
//! and error correction analysis.
//!
//! ## Quick Start
//!
//! The easiest way to use PECOS is through the unified simulation API:
//!
//! ```rust,no_run
//! use pecos::{sim, QasmProgram};
//! use pecos::sparse_stabilizer;
//!
//! // Create a QASM program
//! let qasm_code = r#"
//!     OPENQASM 2.0;
//!     include "qelib1.inc";
//!     qreg q[2];
//!     creg c[2];
//!     h q[0];
//!     cx q[0], q[1];
//!     measure q -> c;
//! "#;
//!
//! let program = QasmProgram::from_string(qasm_code);
//!
//! // Run simulation
//! let results = sim(program)
//!     .quantum(sparse_stabilizer())
//!     .seed(42)
//!     .run(1000)?;
//!
//! println!("Got {} shots", results.len());
//! # Ok::<(), pecos_core::errors::PecosError>(())
//! ```
//!
//! ## Program Types
//!
//! PECOS supports multiple quantum program formats:
//! - QASM (OpenQASM 2.0)
//! - QIS (Quantum Instruction Set - LLVM IR)
//! - HUGR (Hierarchical Unified Graph Representation)
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
pub use pecos_qasm::run_qasm;
pub use unified_sim::{ProgrammedSimBuilder, SimBuilderExt, sim};

// Re-export program types from pecos-programs
pub use pecos_programs::{HugrProgram, Program, QasmProgram, QisProgram};

// Re-export engine builders from individual crates
#[cfg(feature = "qasm")]
pub use pecos_qasm::qasm_engine;

// QIS/LLVM engine functionality now provided by pecos_qis_ccengine

#[cfg(feature = "phir")]
pub use pecos_phir_json::phir_json_engine;

// Re-export qis_control_engine and related functions
pub use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface, native_runtime, selene_simple_runtime};