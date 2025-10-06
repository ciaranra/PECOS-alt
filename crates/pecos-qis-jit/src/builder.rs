//! JIT Interface Builder
//!
//! This module provides the builder pattern for creating JIT-based QisInterfaces.
//! It implements the QisInterfaceBuilder trait from pecos-qis-core.

use crate::JitExecutor;
use pecos_core::errors::PecosError;
use pecos_programs::{QisProgram, HugrProgram};
use pecos_qis_core::program::QisInterfaceBuilder;
use pecos_qis_ffi::OperationCollector;

/// JIT-based interface builder
///
/// This builder creates JIT executor instances from various program formats.
/// Uses JitExecutor which properly supports Selene-style LLVM IR symbols.
#[derive(Debug, Clone)]
pub struct JitInterfaceBuilder;

impl JitInterfaceBuilder {
    /// Create a new JIT interface builder
    pub fn new() -> Self {
        Self
    }
}

impl Default for JitInterfaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl QisInterfaceBuilder for JitInterfaceBuilder {
    fn build_from_qis_program(&self, program: QisProgram) -> Result<OperationCollector, PecosError> {
        let mut executor = JitExecutor::new();

        // Execute the program using JIT executor
        let llvm_ir = match &program.content {
            pecos_programs::QisContent::Ir(ir_text) => ir_text.clone(),
            pecos_programs::QisContent::Bitcode(_bitcode) => {
                return Err(PecosError::Processing(
                    "JIT executor currently only supports LLVM IR text.\n\
                    Convert bitcode to text using llvm-dis first.".to_string()
                ));
            }
        };

        // Execute LLVM IR and collect operations
        executor.execute_llvm_ir(&llvm_ir)
            .map_err(|e| PecosError::Processing(format!("Failed to execute QIS program with JIT: {}", e)))
    }

    fn build_from_hugr_program(&self, program: HugrProgram) -> Result<OperationCollector, PecosError> {
        #[cfg(feature = "hugr")]
        {
            // Compile HUGR to LLVM IR using pecos-hugr-qis
            let llvm_ir = pecos_hugr_qis::compile_hugr_bytes_to_string(&program.hugr)
                .map_err(|e| PecosError::Processing(format!("Failed to compile HUGR to LLVM: {}", e)))?;

            // Create a QIS program from the compiled LLVM IR
            let qis_program = pecos_programs::QisProgram::from_string(&llvm_ir);

            // Use the existing QIS program builder
            self.build_from_qis_program(qis_program)
        }
        #[cfg(not(feature = "hugr"))]
        {
            let _ = program; // Suppress unused variable warning
            Err(PecosError::Processing(
                "JIT interface requires the 'hugr' feature to compile HUGR programs.\n\
                Please enable the 'hugr' feature in pecos-qis-jit to use HUGR compilation.".to_string()
            ))
        }
    }

    fn build_from_interface(&self, interface: OperationCollector) -> Result<OperationCollector, PecosError> {
        // Already an OperationCollector, just return it
        Ok(interface)
    }

    fn name(&self) -> &'static str {
        "JitInterfaceBuilder"
    }
}

/// Convenience function to create a JIT interface builder
pub fn jit_interface_builder() -> JitInterfaceBuilder {
    JitInterfaceBuilder::new()
}
