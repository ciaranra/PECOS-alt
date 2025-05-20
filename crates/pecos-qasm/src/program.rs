// Copyright 2025 The PECOS Developers
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

//! QASM program representation and parsing utilities.

use crate::engine::QASMEngine;
use crate::parser::Program;
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalEngine;
use std::fs::read_to_string;
use std::path::Path;
use std::str::FromStr;

/// A parsed QASM program.
///
/// This type represents a parsed QASM program that can be used to create a `QASMEngine`
/// or inspected directly. It provides a more type-safe way to handle QASM programs
/// compared to using raw strings.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use pecos_qasm::QASMProgram;
/// use std::str::FromStr;
///
/// // Parse a QASM program from a string
/// let qasm = r#"
///     OPENQASM 2.0;
///     include "qelib1.inc";
///     qreg q[2];
///     h q[0];
/// "#;
///
/// let program = QASMProgram::from_str(qasm).unwrap();
///
/// // Get information about the program
/// println!("Total qubits: {}", program.num_qubits());
///
/// // Convert to a QASMEngine for execution
/// let engine = program.into_engine();
/// ```
///
/// Using with the PECOS simulation API:
///
/// ```no_run
/// use pecos_qasm::QASMProgram;
/// use std::str::FromStr;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Parse a QASM program
/// let qasm = r#"
///     OPENQASM 2.0;
///     include "qelib1.inc";
///     qreg q[2];
///     h q[0];
///     cx q[0], q[1];
///     measure q -> c;
/// "#;
///
/// let program = QASMProgram::from_str(qasm)?;
///
/// // Convert directly to a boxed engine ready for simulation
/// // This is more concise than `Box::new(program.into_engine())`
/// let engine_box = program.into_engine_box();
///
/// // Use with pecos::run_sim (not actually run in this example)
/// // let results = pecos::run_sim(engine_box, 1000, Some(42), None, None, None)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct QASMProgram {
    /// The parsed QASM program
    program: Program,
    /// The original QASM code
    source: String,
}

impl QASMProgram {
    /// Creates a new `QASMProgram` from a parsed Program and source code.
    ///
    /// This is generally used internally - users should prefer `from_str` or `from_file`.
    #[must_use]
    pub fn new(program: Program, source: String) -> Self {
        Self { program, source }
    }

    /// Get the number of qubits in the program.
    #[must_use]
    pub fn num_qubits(&self) -> usize {
        self.program.total_qubits
    }

    /// Get a reference to the internal Program AST.
    #[must_use]
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Get the original source code of the program.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Convert this program into a `QASMEngine` for execution.
    #[must_use]
    pub fn into_engine(self) -> QASMEngine {
        QASMEngine::new(self)
    }

    /// Convert this program into a boxed `QASMEngine` ready for simulation.
    ///
    /// This is particularly convenient when using the `run_sim` function from the
    /// pecos crate, which takes a `Box<dyn ClassicalEngine>`.
    #[must_use]
    pub fn into_engine_box(self) -> Box<dyn ClassicalEngine> {
        Box::new(self.into_engine())
    }

    /// Parse a QASM program from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the QASM code is invalid.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, PecosError> {
        let source = read_to_string(path).map_err(|e| PecosError::Input(e.to_string()))?;
        Self::from_str(&source)
    }
}

impl FromStr for QASMProgram {
    type Err = PecosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let program = crate::parser::QASMParser::parse_str(s)?;
        Ok(Self::new(program, s.to_string()))
    }
}

impl std::fmt::Display for QASMProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)
    }
}
