// Python bindings for HUGR to LLVM compilation
use pecos::prelude::*;

use pyo3::prelude::*;

/// Compile HUGR to LLVM IR
///
/// This function takes HUGR bytes (envelope format) and compiles them to LLVM IR
/// using the PECOS HUGR compiler that generates QIS-compatible output.
///
/// Args:
///     `hugr_bytes`: HUGR program as envelope bytes
///
/// Returns:
///     LLVM IR as a string
#[pyfunction(name = "compile_hugr_to_llvm")]
pub fn py_compile_hugr_to_llvm(hugr_bytes: &[u8]) -> PyResult<String> {
    compile_hugr_bytes_to_string(hugr_bytes)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

/// Register HUGR compilation functions with the Python module
pub fn register_hugr_compilation_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_compile_hugr_to_llvm, m)?)?;
    Ok(())
}
