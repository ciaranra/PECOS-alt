use crate::base::{ProcessorBase, StatefulProcessorBase};
use crate::{CoProcessorBase, DrivingProcessorBase};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

// Message types for JSON serialization
#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum OutgoingMessage {
    Execute {
        operation: String,
        style: String,
        args: Vec<i32>,
    },
    #[allow(dead_code)]
    ProcessBatch {
        operation: String,
        style: String,
        batch: Value,
    },
    ListPlugins,
    Shutdown,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    Result { value: i32 },
    BatchResult { value: Value },
    Error { message: String },
    PluginList { plugins: Vec<PythonPluginInfo> },
}

#[derive(Deserialize, Debug)]
pub struct PythonPluginInfo {
    name: String,
    style: String,
    description: String,
}

// Python interface with message handling
#[pyclass]
pub struct PluginMessage {
    #[pyo3(get)]
    operation: String,
    #[pyo3(get)]
    style: String,
    #[pyo3(get)]
    args: Vec<i32>,
}

type PluginListResult = (i32, String, Option<Vec<(String, String, String)>>);

#[pymethods]
impl PluginMessage {
    #[new]
    fn new(operation: String, style: String, args: Vec<i32>) -> Self {
        Self {
            operation,
            style,
            args,
        }
    }

    fn to_json(&self) -> PyResult<String> {
        let msg = OutgoingMessage::Execute {
            operation: self.operation.clone(),
            style: self.style.clone(),
            args: self.args.clone(),
        };
        serde_json::to_string(&msg)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<PluginListResult> {
        let msg: IncomingMessage = serde_json::from_str(json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        match msg {
            IncomingMessage::Result { value } => Ok((value, String::new(), None)),
            IncomingMessage::Error { message } => Ok((0, message, None)),
            IncomingMessage::PluginList { plugins } => {
                let plugin_list = plugins
                    .into_iter()
                    .map(|p| (p.name, p.style, p.description))
                    .collect();
                Ok((0, String::new(), Some(plugin_list)))
            }
            IncomingMessage::BatchResult { value } => Ok((0, value.to_string(), None)),
        }
    }

    #[staticmethod]
    fn list_plugins() -> PyResult<String> {
        serde_json::to_string(&OutgoingMessage::ListPlugins {})
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    #[staticmethod]
    fn shutdown() -> PyResult<String> {
        serde_json::to_string(&OutgoingMessage::Shutdown {})
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

// Internal plugin info for PythonService
#[derive(Debug)]
struct PluginInfo {
    name: String,
    description: String,
    implementation: Py<PyAny>,
}

// The PyO3 PythonService implementation that Python uses
#[pyclass]
pub struct PythonService {
    loaded_plugin_names: HashSet<String>,
    function_plugins: Vec<PluginInfo>,
    class_plugins: Vec<PluginInfo>,
    stateful_plugins: Vec<PluginInfo>,
    coprocessor_plugins: Vec<PluginInfo>,
    driving_processor_plugins: Vec<PluginInfo>,
}

#[pymethods]
impl PythonService {
    #[new]
    fn new() -> Self {
        Self {
            loaded_plugin_names: HashSet::new(),
            function_plugins: Vec::new(),
            class_plugins: Vec::new(),
            stateful_plugins: Vec::new(),
            coprocessor_plugins: Vec::new(),
            driving_processor_plugins: Vec::new(),
        }
    }

    fn load_plugin(&mut self, py: Python<'_>, path: &str) -> PyResult<()> {
        let code = std::fs::read_to_string(path)?;
        let code = std::ffi::CString::new(code)?;
        let empty = std::ffi::CString::new("")?;

        let module = PyModule::from_code(py, &code, &empty, &empty)?;

        for item in module.dir()? {
            let name: String = item.extract()?;
            if name.starts_with('_') {
                continue;
            }

            if let Ok(obj) = module.getattr(&name) {
                // Handle function-style python-plugins
                if obj.is_callable() {
                    if let Ok(doc) = obj.getattr("__doc__") {
                        if !self.loaded_plugin_names.contains(&name) {
                            let description = doc.extract::<String>()?;
                            let implementation = obj.extract::<PyObject>()?;
                            self.function_plugins.push(PluginInfo {
                                name: name.clone(),
                                description,
                                implementation: implementation.into_bound(py).into(),
                            });
                            self.loaded_plugin_names.insert(name);
                        }
                    }
                }
                // Handle class-style python-plugins
                else if let Ok(cls) = obj.downcast::<PyType>() {
                    if cls.is_subclass_of::<ProcessorBase>()? {
                        // Skip base classes
                        let class_name = cls.getattr("__name__")?.extract::<String>()?;
                        if class_name == "ProcessorBase" {
                            continue;
                        }

                        let instance = cls.call0()?;
                        let name = instance.getattr("name")?.extract::<String>()?;
                        let description = instance.getattr("description")?.extract::<String>()?;

                        if !name.is_empty()
                            && !description.is_empty()
                            && !self.loaded_plugin_names.contains(&name)
                        {
                            let type_obj = cls.extract::<PyObject>()?;
                            let class_impl = type_obj.into_bound(py).into();
                            self.class_plugins.push(PluginInfo {
                                name: name.clone(),
                                description,
                                implementation: class_impl,
                            });
                            self.loaded_plugin_names.insert(name);
                        }
                    }
                    // Handle stateful python-plugins
                    else if cls.is_subclass_of::<StatefulProcessorBase>()? {
                        // Skip base classes
                        let class_name = cls.getattr("__name__")?.extract::<String>()?;
                        if class_name == "StatefulProcessorBase" {
                            continue;
                        }

                        let instance = cls.call0()?;
                        let name = instance.getattr("name")?.extract::<String>()?;
                        let description = instance.getattr("description")?.extract::<String>()?;

                        if !name.is_empty()
                            && !description.is_empty()
                            && !self.loaded_plugin_names.contains(&name)
                        {
                            let type_obj = cls.extract::<PyObject>()?;
                            let class_impl = type_obj.into_bound(py).into();
                            self.stateful_plugins.push(PluginInfo {
                                name: name.clone(),
                                description,
                                implementation: class_impl,
                            });
                            self.loaded_plugin_names.insert(name);
                        }
                    }
                    // Handle coprocessor python-plugins
                    else if cls.is_subclass_of::<CoProcessorBase>()? {
                        // Skip base classes
                        let class_name = cls.getattr("__name__")?.extract::<String>()?;
                        if class_name == "CoProcessorBase" {
                            continue;
                        }

                        let instance = cls.call0()?;
                        let name = instance.getattr("name")?.extract::<String>()?;
                        let description = instance.getattr("description")?.extract::<String>()?;

                        if !name.is_empty()
                            && !description.is_empty()
                            && !self.loaded_plugin_names.contains(&name)
                        {
                            let type_obj = cls.extract::<PyObject>()?;
                            let class_impl = type_obj.into_bound(py).into();
                            self.coprocessor_plugins.push(PluginInfo {
                                name: name.clone(),
                                description,
                                implementation: class_impl,
                            });
                            self.loaded_plugin_names.insert(name);
                        }
                    }
                    // Handle driving processor python-plugins
                    else if cls.is_subclass_of::<DrivingProcessorBase>()? {
                        // Skip base classes
                        let class_name = cls.getattr("__name__")?.extract::<String>()?;
                        if class_name == "DrivingProcessorBase" {
                            continue;
                        }

                        let instance = cls.call0()?;
                        let name = instance.getattr("name")?.extract::<String>()?;
                        let description = instance.getattr("description")?.extract::<String>()?;

                        if !name.is_empty()
                            && !description.is_empty()
                            && !self.loaded_plugin_names.contains(&name)
                        {
                            let type_obj = cls.extract::<PyObject>()?;
                            let class_impl = type_obj.into_bound(py).into();
                            self.driving_processor_plugins.push(PluginInfo {
                                name: name.clone(),
                                description,
                                implementation: class_impl,
                            });
                            self.loaded_plugin_names.insert(name);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn execute_function(&self, py: Python<'_>, operation: &str, args: Vec<i32>) -> PyResult<i32> {
        for plugin in &self.function_plugins {
            if plugin.name == operation {
                let bound_func = plugin.implementation.bind(py);
                let result = bound_func.call1((args,))?;
                return result.extract();
            }
        }
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "No function plugin handles operation: {}",
            operation
        )))
    }

    fn execute_class(&self, py: Python<'_>, operation: &str, a: u32, b: u32) -> PyResult<u32> {
        for plugin in &self.class_plugins {
            if plugin.name == operation {
                let cls = plugin.implementation.bind(py);
                let instance = cls.call0()?;
                let result = instance.call_method1("process", (a, b))?;
                return result.extract();
            }
        }
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "No class plugin handles operation: {}",
            operation
        )))
    }

    fn get_plugin_class(&self, py: Python<'_>, operation: &str) -> PyResult<Py<PyAny>> {
        for plugin in &self.stateful_plugins {
            if plugin.name == operation {
                return Ok(plugin.implementation.clone_ref(py));
            }
        }
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "No stateful plugin found: {}",
            operation
        )))
    }

    fn list_plugins<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let list = PyList::empty(py);

        // Add function python-plugins
        for plugin in &self.function_plugins {
            if !plugin.name.is_empty() && !plugin.description.is_empty() {
                list.append((&plugin.name, "function", &plugin.description))?;
            }
        }

        // Add class python-plugins
        for plugin in &self.class_plugins {
            if !plugin.name.is_empty() && !plugin.description.is_empty() {
                list.append((&plugin.name, "class", &plugin.description))?;
            }
        }

        // Add stateful python-plugins
        for plugin in &self.stateful_plugins {
            if !plugin.name.is_empty() && !plugin.description.is_empty() {
                list.append((&plugin.name, "stateful", &plugin.description))?;
            }
        }

        // Add coprocessor python-plugins
        for plugin in &self.coprocessor_plugins {
            if !plugin.name.is_empty() && !plugin.description.is_empty() {
                list.append((&plugin.name, "coprocessor", &plugin.description))?;
            }
        }

        // Add driving processor python-plugins
        for plugin in &self.driving_processor_plugins {
            if !plugin.name.is_empty() && !plugin.description.is_empty() {
                list.append((&plugin.name, "driving_processor", &plugin.description))?;
            }
        }

        Ok(list)
    }
}
