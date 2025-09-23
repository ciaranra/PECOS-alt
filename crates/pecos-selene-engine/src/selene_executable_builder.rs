//! Builder for `SeleneExecutableEngine`
//!
//! This builder configures and creates `SeleneExecutableEngine` instances,
//! using Selene's `build()` API and the `PecosSeleneBridgeSimulator`.

use crate::selene_executable_engine::SeleneExecutableEngine;
use pecos_core::prelude::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_programs::{HugrProgram, LlvmProgram, Program, SeleneInterfaceProgram};
use std::path::PathBuf;

/// Builder for creating `SeleneExecutableEngine` instances
#[derive(Clone)]
pub struct SeleneExecutableEngineBuilder {
    /// The program to execute
    program: Option<SeleneInterfaceProgram>,

    /// LLVM program (for backwards compatibility)
    llvm_program: Option<LlvmProgram>,

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
            llvm_program: None,
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
            Program::Llvm(llvm_prog) => {
                // Store LLVM program for later processing
                self.llvm_program = Some(llvm_prog);
            }
            Program::Qis(qis_prog) => {
                // QIS is Selene QIS format LLVM IR, treat it as LLVM
                log::info!("QIS program provided, treating as LLVM IR");
                self.llvm_program = Some(LlvmProgram::from_string(qis_prog.source().to_string()));
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
        if self.program.is_none() && self.llvm_program.is_none() && self.hugr_program.is_none() {
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
            // Compile HUGR to LLVM IR using selected compiler
            let llvm_ir = match self.hugr_compiler.as_str() {
                "selene" => {
                    // Try to use Selene's compiler through Python
                    // This would require Python interop, so for pure Rust usage, we error
                    return Err(PecosError::Input(
                        "Selene's HUGR compiler requires Python environment. \
                         Use .hugr_compiler(\"pecos\") for pure Rust compilation, \
                         or compile HUGR to LLVM in Python before passing to Rust.".to_string()
                    ));
                }
                "pecos" => {
                    pecos_hugr_qis::compile_hugr_bytes_to_string(hugr_prog.bytes())
                        .map_err(|e| PecosError::Input(format!("Failed to compile HUGR with PECOS compiler: {e}")))?
                }
                other => {
                    return Err(PecosError::Input(
                        format!("Invalid HUGR compiler '{}'. Use 'selene' or 'pecos'.", other)
                    ));
                }
            };

            log::info!("Successfully compiled HUGR to LLVM IR using {} compiler", self.hugr_compiler);
            engine = engine.with_llvm_program(LlvmProgram::from_ir(llvm_ir));
        } else if let Some(llvm_prog) = self.llvm_program {
            // Regular LLVM program
            engine = engine.with_llvm_program(llvm_prog);
        }

        Ok(engine)
    }
}

/// Create a new `SeleneExecutableEngineBuilder`
#[must_use]
pub fn selene_executable() -> SeleneExecutableEngineBuilder {
    SeleneExecutableEngineBuilder::new()
}
