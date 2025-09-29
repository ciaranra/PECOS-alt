//! Builder for `SeleneExecutableEngine`
//!
//! This builder configures and creates `SeleneExecutableEngine` instances,
//! using Selene's `build()` API and the `PecosSeleneBridgeSimulator`.

use crate::selene_executable_engine::SeleneExecutableEngine;
use pecos_core::prelude::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_programs::{HugrProgram, Program, QisProgram, SeleneInterfaceProgram};
use std::path::PathBuf;

/// Builder for creating `SeleneExecutableEngine` instances
#[derive(Clone)]
pub struct SeleneExecutableEngineBuilder {
    /// The program to execute
    program: Option<SeleneInterfaceProgram>,

    /// QIS program (Selene QIS format LLVM IR)
    qis_program: Option<QisProgram>,

    /// HUGR program (will be compiled to LLVM IR)
    hugr_program: Option<HugrProgram>,

    /// Number of qubits
    num_qubits: Option<usize>,

    /// Working directory for temporary files
    working_dir: Option<PathBuf>,

    /// Whether to enable verbose output
    verbose: bool,

    /// Path to the bridge simulator plugin (auto-detected if not specified)
    plugin_path: Option<PathBuf>,

    /// HUGR compiler to use ("selene" or "pecos", defaults to "selene")
    hugr_compiler: String,
}

impl SeleneExecutableEngineBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: None,
            qis_program: None,
            hugr_program: None,
            num_qubits: None,
            working_dir: None,
            verbose: false,
            plugin_path: None,
            hugr_compiler: "selene".to_string(),
        }
    }

    /// Set the HUGR compiler to use
    ///
    /// Options:
    /// - "selene": Use Selene's hugr-qis compiler (requires Python environment)
    /// - "pecos": Use PECOS's Rust HUGR compiler
    ///
    /// Default is "selene"
    #[must_use]
    pub fn hugr_compiler(mut self, compiler: impl Into<String>) -> Self {
        self.hugr_compiler = compiler.into();
        self
    }

    /// Set the program to execute
    #[must_use]
    pub fn program(mut self, program: impl Into<Program>) -> Self {
        match program.into() {
            Program::SeleneInterface(selene_prog) => {
                self.program = Some(selene_prog);
            }
            Program::Qis(qis_prog) => {
                // QIS is Selene QIS format LLVM IR
                log::info!("QIS program provided");
                self.qis_program = Some(qis_prog);
            }
            Program::Hugr(hugr_prog) => {
                // Store HUGR program for compilation during build
                log::info!("HUGR program will be compiled to LLVM IR during build");
                self.hugr_program = Some(hugr_prog);
            }
            _ => {
                log::warn!(
                    "SeleneExecutableEngine only supports SeleneInterfaceProgram, LlvmProgram, QisProgram, and HugrProgram"
                );
            }
        }
        self
    }

    /// Set a `SeleneInterfaceProgram` directly
    #[must_use]
    pub fn selene_interface_program(mut self, program: SeleneInterfaceProgram) -> Self {
        self.program = Some(program);
        self
    }

    /// Set a HUGR program directly
    #[must_use]
    pub fn hugr(mut self, hugr: impl Into<HugrProgram>) -> Self {
        self.hugr_program = Some(hugr.into());
        self
    }

    /// Set the number of qubits
    #[must_use]
    pub fn qubits(mut self, n: usize) -> Self {
        self.num_qubits = Some(n);
        self
    }

    /// Alias for qubits
    #[must_use]
    pub fn num_qubits(self, n: usize) -> Self {
        self.qubits(n)
    }

    /// Set the working directory
    #[must_use]
    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Enable verbose output
    #[must_use]
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set optimization flag (for API compatibility - currently ignored)
    #[must_use]
    pub fn optimize(self, _optimize: bool) -> Self {
        // Note: This method is provided for API compatibility with existing code.
        // The SeleneExecutableEngine doesn't have configurable optimization settings,
        // so this parameter is ignored.
        self
    }

    /// Set the plugin path explicitly (for testing or custom plugins)
    #[must_use]
    pub fn plugin(mut self, path: impl Into<PathBuf>) -> Self {
        self.plugin_path = Some(path.into());
        self
    }
}

impl Default for SeleneExecutableEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassicalControlEngineBuilder for SeleneExecutableEngineBuilder {
    type Engine = SeleneExecutableEngine;

    fn build(self) -> Result<Self::Engine, PecosError> {
        // Check if we have a program - this is required
        if self.program.is_none() && self.qis_program.is_none() && self.hugr_program.is_none() {
            return Err(PecosError::Input(
                "No program specified. Use .program() to set a SeleneInterface, LLVM, or HUGR program.".to_string()
            ));
        }

        let num_qubits = self.num_qubits.unwrap_or(10);
        log::debug!("SeleneExecutableEngineBuilder.build() called with {num_qubits} qubits");

        let mut engine = SeleneExecutableEngine::new(num_qubits)?.with_verbose(self.verbose);

        if let Some(working_dir) = self.working_dir {
            engine = engine.with_working_dir(working_dir);
        }

        if let Some(plugin_path) = self.plugin_path {
            engine = engine.with_plugin_path(plugin_path);
        }

        if let Some(program) = self.program {
            engine = engine.with_program(program);
        } else if let Some(hugr_prog) = self.hugr_program {
            // Pass HUGR directly to the engine - it will handle compilation
            log::info!("Passing HUGR program directly to SeleneExecutableEngine");
            engine = engine.with_hugr_program(hugr_prog);
        } else if let Some(qis_prog) = self.qis_program {
            // QIS program
            engine = engine.with_qis_program(qis_prog);
        }

        Ok(engine)
    }
}

/// Create a new `SeleneExecutableEngineBuilder`
#[must_use]
pub fn selene_executable() -> SeleneExecutableEngineBuilder {
    SeleneExecutableEngineBuilder::new()
}
