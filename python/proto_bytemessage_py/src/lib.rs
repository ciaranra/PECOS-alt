mod message_batch;
mod python_processor;

use message_batch::PyMessageBatch;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
pub use python_processor::PythonProcessor;

#[pyfunction]
fn bytes_to_u32(_py: Python, bytes: &[u8]) -> PyResult<u32> {
    bytemuck::try_from_bytes(bytes)
        .map(|n: &u32| *n)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

#[pyfunction]
fn u32_to_bytes(py: Python, n: u32) -> Py<PyBytes> {
    PyBytes::new(py, bytemuck::bytes_of(&n)).into()
}

/// Protopr Python bindings
#[pymodule]
fn proto_bytemessage_py(_py: Python<'_>, m: &'_ Bound<'_, PyModule>) -> PyResult<()> {
    m.add("PyMessageBatch", _py.get_type::<PyMessageBatch>())?;
    m.add("bytes_to_u32", wrap_pyfunction!(bytes_to_u32, _py)?)?;
    m.add("u32_to_bytes", wrap_pyfunction!(u32_to_bytes, _py)?)?;
    Ok(())
}

