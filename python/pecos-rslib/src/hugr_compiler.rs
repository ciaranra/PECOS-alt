//! HUGR to LLVM compilation Python bindings

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

/// Compile HUGR bytes to LLVM IR string
#[pyfunction]
#[pyo3(signature = (hugr_bytes, output_path=None))]
pub fn compile_hugr_to_llvm_rust(
    hugr_bytes: &[u8],
    output_path: Option<String>
) -> PyResult<String> {
    // Use the pecos-selene HUGR 0.13 compiler instead of pecos-hugr
    use pecos_selene::hugr_to_llvm::GuppylangCompiler;

    let mut compiler = GuppylangCompiler::new();

    match compiler.compile_hugr_json(hugr_bytes) {
        Ok(llvm_ir) => {
            // If output path is provided, also write to file
            if let Some(path) = output_path {
                std::fs::write(&path, &llvm_ir)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to write LLVM IR to file: {}", e)))?;
            }
            Ok(llvm_ir)
        }
        Err(e) => Err(PyRuntimeError::new_err(format!("Failed to compile HUGR: {}", e)))
    }
}

/// Check if Rust HUGR backend is available
#[pyfunction]
pub fn check_rust_hugr_availability() -> bool {
    true
}

/// Module containing HUGR compilation functions
pub fn register_hugr_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_hugr_to_llvm_rust, m)?)?;
    m.add_function(wrap_pyfunction!(check_rust_hugr_availability, m)?)?;
    m.add("RUST_HUGR_AVAILABLE", true)?;
    Ok(())
}