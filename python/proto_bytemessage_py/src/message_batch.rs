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

    fn iter<'py>(&self, py: Python<'py>) -> PyResult<Py<PyList>> {
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
    fn _into_message_batch(&self) -> PyResult<PyMessageBatch> {
        Ok(Self {
            batch: self.batch.clone(),
        })
    }

    /// Create from a MessageBatch
    #[classmethod]
    fn from_message_batch(_cls: &Bound<'_, PyType>, py_batch: PyRef<'_, PyMessageBatch>) -> PyResult<Self> {
        Ok(Self {
            batch: py_batch.batch.clone(),
        })
    }

    /// Get all messages as a list of (type, payload) tuples
    pub fn get_messages<'py>(&self, _py: Python<'py>) -> PyResult<Vec<(u8, &[u8])>> {
        Ok(self.batch.iter().map(|(h, p)| (h.msg_type, p)).collect())
    }
}