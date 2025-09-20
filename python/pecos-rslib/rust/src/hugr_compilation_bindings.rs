// Python bindings for HUGR to LLVM compilation

use pyo3::prelude::*;

/// Compile HUGR to LLVM IR
///
/// This function takes HUGR bytes (JSON format) and compiles them to LLVM IR
/// using the pecos-selene-engine compiler.
///
/// Args:
///     `hugr_bytes`: HUGR program as JSON bytes
///
/// Returns:
///     LLVM IR as a string
#[pyfunction]
pub fn compile_hugr_to_llvm(hugr_bytes: &[u8]) -> PyResult<String> {
    #[cfg(feature = "hugr-013")]
    {
        use pecos_selene_engine::hugr_to_llvm::compile_hugr_to_llvm as rust_compile_hugr_to_llvm;

        rust_compile_hugr_to_llvm(hugr_bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[cfg(not(feature = "hugr-013"))]
    {
        Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
            "compile_hugr_to_llvm requires pecos-rslib to be compiled with hugr-013 feature",
        ))
    }
}

/// Register HUGR compilation functions with the Python module
pub fn register_hugr_compilation_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_hugr_to_llvm, m)?)?;
    Ok(())
}
