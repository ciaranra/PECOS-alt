//! Builder for SeleneExecutableEngine
//!
//! This builder configures and creates SeleneExecutableEngine instances,
//! using Selene's build() API and the PecosSeleneBridgeSimulator.

use crate::selene_executable_engine::SeleneExecutableEngine;
use pecos_core::prelude::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_programs::{Program, SeleneInterfaceProgram, LlvmProgram};
use std::path::PathBuf;

/// Builder for creating SeleneExecutableEngine instances
#[derive(Clone)]
pub struct SeleneExecutableEngineBuilder {
    /// The program to execute
    program: Option<SeleneInterfaceProgram>,
    
    /// LLVM program (for backwards compatibility)
    llvm_program: Option<LlvmProgram>,
    
    /// Number of qubits
    num_qubits: Option<usize>,
    
    /// Working directory for temporary files
    working_dir: Option<PathBuf>,
    
    /// Whether to enable verbose output
    verbose: bool,
    
    /// Path to the bridge simulator plugin (auto-detected if not specified)
    plugin_path: Option<PathBuf>,
}

impl SeleneExecutableEngineBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            program: None,
            llvm_program: None,
            num_qubits: None,
            working_dir: None,
            verbose: false,
            plugin_path: None,
        }
    }
    
    /// Set the program to execute
    pub fn program(mut self, program: impl Into<Program>) -> Self {
        match program.into() {
            Program::SeleneInterface(selene_prog) => {
                self.program = Some(selene_prog);
            }
            Program::Llvm(llvm_prog) => {
                // Store LLVM program for later processing
                self.llvm_program = Some(llvm_prog);
            }
            _ => {
                log::warn!("SeleneExecutableEngine only supports SeleneInterfaceProgram and LlvmProgram");
            }
        }
        self
    }
    
    /// Set a SeleneInterfaceProgram directly
    pub fn selene_interface_program(mut self, program: SeleneInterfaceProgram) -> Self {
        self.program = Some(program);
        self
    }
    
    /// Set the number of qubits
    pub fn qubits(mut self, n: usize) -> Self {
        self.num_qubits = Some(n);
        self
    }
    
    /// Alias for qubits
    pub fn num_qubits(self, n: usize) -> Self {
        self.qubits(n)
    }
    
    /// Set the working directory
    pub fn working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }
    
    /// Enable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
    
    /// Set optimization flag (for API compatibility - currently ignored)
    pub fn optimize(self, _optimize: bool) -> Self {
        // Note: This method is provided for API compatibility with existing code.
        // The SeleneExecutableEngine doesn't have configurable optimization settings,
        // so this parameter is ignored.
        self
    }
    
    /// Set the plugin path explicitly (for testing or custom plugins)
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
        let num_qubits = self.num_qubits.unwrap_or(10);
        println!("*** BUILDER: SeleneExecutableEngineBuilder.build() called with {} qubits ***", num_qubits);
        
        let mut engine = SeleneExecutableEngine::new(num_qubits)?
            .with_verbose(self.verbose);
        
        if let Some(working_dir) = self.working_dir {
            engine = engine.with_working_dir(working_dir);
        }
        
        if let Some(plugin_path) = self.plugin_path {
            engine = engine.with_plugin_path(plugin_path);
        }
        
        if let Some(program) = self.program {
            engine = engine.with_program(program);
        } else if let Some(llvm_prog) = self.llvm_program {
            // Handle LLVM program by storing it directly in the engine
            engine = engine.with_llvm_program(llvm_prog);
        }
        
        Ok(engine)
    }
}

/// Create a new SeleneExecutableEngineBuilder  
pub fn selene_executable() -> SeleneExecutableEngineBuilder {
    SeleneExecutableEngineBuilder::new()
}