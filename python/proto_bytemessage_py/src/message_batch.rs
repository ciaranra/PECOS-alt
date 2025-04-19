use proto_bytemessage::message::{BatchBuilder, MessageBatch};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyType};

#[pyclass]
#[derive(Clone)]
pub struct PyMessageBatch {
    pub(crate) batch: MessageBatch,
}

impl PyMessageBatch {
    pub fn new_with_batch(batch: MessageBatch) -> Self {
        Self { batch }
    }
}

#[pymethods]
impl PyMessageBatch {
    #[new]
    fn py_new() -> Self {
        Self {
            batch: BatchBuilder::new().build(),
        }
    }

    /// Convert the batch to a Python list of (type, bytes) tuples
    fn to_list(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let messages: Vec<(u8, Py<PyBytes>)> = self
            .batch
            .iter()
            .map(|(header, payload)| {
                let bytes = PyBytes::new(py, payload);
                (header.msg_type, bytes.into())
            })
            .collect();

        let list = PyList::new(py, messages)?;
        Ok(list.into())
    }

    /// Convert to a MessageBatch clone
    #[pyo3(name = "into_message_batch")]
    fn _into_message_batch(&self) -> PyMessageBatch {
        Self {
            batch: self.batch.clone(),
        }
    }

    /// Create from a MessageBatch
    #[allow(clippy::needless_pass_by_value)]
    #[classmethod]
    fn from_message_batch(_cls: &Bound<'_, PyType>, py_batch: PyRef<'_, PyMessageBatch>) -> Self {
        Self {
            batch: py_batch.batch.clone(),
        }
    }

    /// Get all messages as a list of (type, payload) tuples
    pub fn get_messages(&self, _py: Python<'_>) -> Vec<(u8, &[u8])> {
        self.batch.iter().map(|(h, p)| (h.msg_type, p)).collect()
    }
}
