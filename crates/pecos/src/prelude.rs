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

//! A prelude for PECOS users.
//!
//! This prelude re-exports the most commonly used types, traits, and functions
//! from all PECOS component crates. By importing this prelude with
//! `use pecos::prelude::*;`, you get access to the complete PECOS API without
//! having to manually import from each component crate.
//!
//! ## Contents
//!
//! This prelude includes re-exports from:
//!
//! * `pecos_core`: Core types, traits, and error handling
//! * `pecos_engines`: Simulation engines for quantum and classical processing
//! * `pecos_phir_json`: PECOS High-level Intermediate Representation
//! * `pecos_qasm`: `OpenQASM` language support
//! * `pecos_qis_core`: QIS control engine with multiple runtime support
//! * `pecos_qsim`: Quantum simulation implementations
//!
//! It also includes key functionality from the top-level PECOS crate:
//!
//! * Simulation functions (`sim`, `sim_builder`)
//! * Engine setup functions (`setup_qasm_engine`, `setup_llvm_engine`)
//! * Program type detection and handling
//!
//! ## Usage
//!
//! ```rust
//! use pecos::prelude::*;
//!
//! // Now you can use all common PECOS types and functions without additional imports
//! ```

// Re-export preludes from component crates
pub use pecos_core::prelude::*;
pub use pecos_engines::prelude::*;
pub use pecos_phir_json::prelude::*;
pub use pecos_qasm::prelude::*;
// Re-export pecos_qis_core selectively to avoid conflicts with pecos_engines
// The main Shot type users should use is from pecos_engines (more feature-rich)
// The QIS Shot is an internal implementation detail
pub use pecos_qis_core::{
    qis_control_engine, QisControlEngine, QisEngineBuilder,
    QisInterface, QisInterfaceBuilder, QisRuntime,
    ProgramFormat, InterfaceError, RuntimeError,
    ClassicalState,
    // Note: Shot and Value from pecos_qis_core are NOT exported to avoid ambiguity
    // Use pecos_engines::Shot and pecos_qis_core::runtime::Value if needed
};
// pecos_qis_sim removed - using pecos_qis_core instead
pub use pecos_qsim::prelude::*;

// Re-export QIS interface implementations when features are enabled
#[cfg(feature = "jit")]
pub use pecos_qis_jit::{JitInterfaceBuilder, QisJitInterface, jit_interface_builder};
#[cfg(feature = "selene")]
pub use pecos_qis_selene::{HeliosInterfaceBuilder, QisHeliosInterface, helios_interface_builder};

// Re-export native runtime
pub use pecos_qis_native::native_runtime;

// Re-export ShotVec directly from pecos_engines for easier access
pub use pecos_engines::shot_results::ShotVec;

// Re-export crate-specific utilities
pub use crate::program::{
    ProgramType, detect_program_type, get_program_path, setup_engine_for_program,
};

// Re-export program types from pecos-programs
pub use pecos_programs::{HugrProgram, Program, QasmProgram, QisProgram};

// Re-export setup functions from format-specific crates
pub use pecos_phir_json::setup_phir_json_engine;
pub use pecos_qasm::setup_qasm_engine;

// Re-export ClassicalControlEngine
pub use pecos_engines::ClassicalControlEngine;

// Re-export unified simulation API
pub use crate::unified_sim::{SimBuilderExt, sim};
pub use pecos_engines::sim_builder;

// Re-export PCG RNG functions
pub use pecos_rng::rng_pcg;
