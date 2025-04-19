use crate::conversion::py_to_value;
use pyo3::prelude::*;

#[pyclass(subclass)]
pub struct ProcessorBase {
    #[pyo3(get, set)]
    pub(crate) name: String,
    #[pyo3(get, set)]
    pub(crate) description: String,
}

#[pymethods]
impl ProcessorBase {
    #[new]
    fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
        }
    }

    #[pyo3(name = "init")]
    fn init(&mut self, name: String, description: String) -> PyResult<()> {
        self.name = name;
        self.description = description;
        Ok(())
    }

    fn process(&self, _a: u32, _b: u32) -> PyResult<u32> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "process() must be implemented",
        ))
    }
}

#[pyclass(subclass)]
pub struct StatefulProcessorBase {
    #[pyo3(get, set)]
    pub(crate) name: String,
    #[pyo3(get, set)]
    pub(crate) description: String,
}

#[pymethods]
impl StatefulProcessorBase {
    #[new]
    fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
        }
    }

    #[pyo3(name = "init")]
    fn init(&mut self, name: String, description: String) -> PyResult<()> {
        self.name = name;
        self.description = description;
        Ok(())
    }

    fn get_state(&self) -> PyResult<i32> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "get_state() must be implemented",
        ))
    }

    fn process(&mut self, _value: i32) -> PyResult<i32> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "process() must be implemented",
        ))
    }
}

#[pyclass(subclass)]
pub struct CoProcessorBase {
    #[pyo3(get, set)]
    pub(crate) name: String,
    #[pyo3(get, set)]
    pub(crate) description: String,
}

#[pymethods]
impl CoProcessorBase {
    #[new]
    fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
        }
    }

    #[pyo3(name = "init")]
    fn init(&mut self, name: String, description: String) -> PyResult<()> {
        self.name = name;
        self.description = description;
        Ok(())
    }

    fn process(&self, _py: Python<'_>, input_data: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let _value = py_to_value(input_data)?;
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "process() must be implemented",
        ))
    }
}

#[pyclass(subclass)]
pub struct DrivingProcessorBase {
    #[pyo3(get, set)]
    pub(crate) name: String,
    #[pyo3(get, set)]
    pub(crate) description: String,
}

#[pymethods]
impl DrivingProcessorBase {
    #[new]
    fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
        }
    }

    #[pyo3(name = "init")]
    fn init(&mut self, name: String, description: String) -> PyResult<()> {
        self.name = name;
        self.description = description;
        Ok(())
    }

    fn start(
        &self,
        _py: Python<'_>,
        input_data: &Bound<'_, PyAny>,
    ) -> PyResult<(String, PyObject)> {
        let _value = py_to_value(input_data)?;
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "start() must be implemented",
        ))
    }

    fn continue_processing(
        &self,
        _py: Python<'_>,
        coprocessor_result: &Bound<'_, PyAny>,
    ) -> PyResult<(String, PyObject)> {
        let _value = py_to_value(coprocessor_result)?;
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "continue_processing() must be implemented",
        ))
    }
}
