use crate::qir_compiler::QirCompiler as InternalQirCompiler;
use pecos_core::errors::PecosError;
use std::path::{Path, PathBuf};

/// Compiles a QIR program to a dynamically loadable library
///
/// This is the main public interface for compiling QIR programs.
/// It delegates to the internal `QirCompiler` for the actual compilation.
pub struct QirCompiler;

impl QirCompiler {
    /// Compile a QIR program to a dynamically loadable library
    ///
    /// This method compiles a QIR (Quantum Intermediate Representation) file into a
    /// dynamically loadable library that can be executed by the QIR engine.
    ///
    /// # Arguments
    ///
    /// * `qir_file` - Path to the QIR file to compile
    /// * `output_dir` - Optional output directory for the compiled library
    ///
    /// # Returns
    ///
    /// * `Result<PathBuf, PecosError>` - Path to the compiled library if successful
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// * `PecosError::ResourceError` - If the QIR file does not exist or is empty
    /// * `PecosError::IO` - If the QIR file cannot be read
    /// * `PecosError::CompilationError` - If the compilation process fails
    /// * `PecosError::IO` - If the temporary directory cannot be created
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pecos_qir::compiler::QirCompiler;
    ///
    /// let library = QirCompiler::compile("program.ll", None)?;
    /// println!("Compiled library: {:?}", library);
    /// # Ok::<(), pecos_core::errors::PecosError>(())
    /// ```
    pub fn compile<P: AsRef<Path>>(
        qir_file: P,
        output_dir: Option<P>,
    ) -> Result<PathBuf, PecosError> {
        InternalQirCompiler::compile(qir_file, output_dir)
    }
}
