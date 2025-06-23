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
