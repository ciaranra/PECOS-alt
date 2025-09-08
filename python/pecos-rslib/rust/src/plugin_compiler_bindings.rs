//! Python bindings for HUGR/LLVM compilation

use pyo3::prelude::*;

// Plugin compilation functionality has been removed.
// Selene uses native executables, not plugins.

/// Compile HUGR 0.13 to LLVM IR
#[cfg(feature = "hugr-013")]
#[pyfunction]
pub fn compile_hugr_to_llvm(hugr_bytes: &[u8]) -> PyResult<String> {
    use pecos_selene::hugr_to_llvm::compile_hugr_to_llvm as rust_compile_hugr_to_llvm;

    rust_compile_hugr_to_llvm(hugr_bytes).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to compile HUGR to LLVM: {e}"))
    })
}

/// Register the compiler module
pub fn register_plugin_compiler_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[cfg(feature = "hugr-013")]
    m.add_function(wrap_pyfunction!(compile_hugr_to_llvm, m)?)?;

    Ok(())
}
