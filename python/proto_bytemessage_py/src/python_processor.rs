use std::sync::Arc;
use crate::message_batch::PyMessageBatch;
use proto_bytemessage::message::MessageBatch;
use proto_bytemessage::process::CoProcessor;
use pyo3::prelude::*;

// Rust-side wrapper for Python processors
pub struct PythonProcessor {
    py_processor: Arc<PyObject>,
}

impl PythonProcessor {
    pub fn new(module_name: &str, class_name: &str) -> PyResult<Self> {
        Python::with_gil(|py| {
            let module = PyModule::import(py, module_name)?;
            let processor = module.getattr(class_name)?.call0()?;
            Ok(Self {
                py_processor: Arc::new(processor.into()),
            })
        })
    }
}

impl Clone for PythonProcessor {
    fn clone(&self) -> Self {
        Self {
            py_processor: self.py_processor.clone(),
        }
    }
}

impl CoProcessor for PythonProcessor {
    fn process(&mut self, input: MessageBatch) -> MessageBatch {
        Python::with_gil(|py| -> PyResult<MessageBatch> {
            // Wrap input in PyMessageBatch
            let py_batch = PyMessageBatch { batch: input };

            // Call Python process method
            let result: PyRef<PyMessageBatch> = self
                .py_processor
                .call_method1(py, "process", (py_batch,))?
                .extract(py)?;

            // Extract MessageBatch back out
            Ok(result.batch.clone())
        })
            .expect("Python processing failed")
    }
}
