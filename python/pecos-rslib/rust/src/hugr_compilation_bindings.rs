// Python bindings for HUGR to LLVM compilation

use pyo3::prelude::*;

/// Compile HUGR to LLVM IR
///
/// This function takes HUGR bytes (envelope format) and compiles them to LLVM IR
/// using the PECOS HUGR compiler that generates Selene QIS-compatible output.
///
/// Args:
///     `hugr_bytes`: HUGR program as envelope bytes
///
/// Returns:
///     LLVM IR as a string
#[pyfunction]
pub fn compile_hugr_to_llvm(hugr_bytes: &[u8]) -> PyResult<String> {
    #[cfg(feature = "hugr-llvm-pipeline")]
    {
        use pecos_hugr_qis::compile_hugr_bytes_to_string;

        compile_hugr_bytes_to_string(hugr_bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[cfg(not(feature = "hugr-llvm-pipeline"))]
    {
        Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
            "compile_hugr_to_llvm requires pecos-rslib to be compiled with hugr-llvm-pipeline feature",
        ))
    }
}

/// Register HUGR compilation functions with the Python module
pub fn register_hugr_compilation_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_hugr_to_llvm, m)?)?;
    Ok(())
}
