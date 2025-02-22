use pyo3::prelude::*;

pub mod base;
pub mod conversion;
pub mod service;

pub use base::{CoProcessorBase, DrivingProcessorBase, ProcessorBase, StatefulProcessorBase};
pub use service::{PluginMessage, PythonService};

#[pymodule]
fn plugin_python_service(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ProcessorBase>()?;
    m.add_class::<StatefulProcessorBase>()?;
    m.add_class::<CoProcessorBase>()?;
    m.add_class::<DrivingProcessorBase>()?;
    m.add_class::<PythonService>()?;
    m.add_class::<PluginMessage>()?;
    Ok(())
}
