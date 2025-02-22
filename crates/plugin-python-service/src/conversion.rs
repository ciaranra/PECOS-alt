use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList};
use serde_json::Value;

#[allow(dead_code)]
pub fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => {
            let bound = b.into_pyobject(py)?;
            Ok(Py::from(
                <pyo3::Bound<'_, PyBool> as Clone>::clone(&bound).unbind(),
            ))
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let bound = i.into_pyobject(py)?;
                Ok(Py::from(bound.unbind()))
            } else if let Some(f) = n.as_f64() {
                let bound = f.into_pyobject(py)?;
                Ok(Py::from(bound.unbind()))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid number type",
                ))
            }
        }
        Value::String(s) => {
            let bound = s.into_pyobject(py)?;
            Ok(Py::from(bound.unbind()))
        }
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(Py::from(list.into_pyobject(py)?.unbind()))
        }
        Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (key, val) in obj {
                dict.set_item(key, value_to_py(py, val)?)?;
            }
            Ok(Py::from(dict.into_pyobject(py)?.unbind()))
        }
    }
}

pub fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }

    if let Ok(val) = obj.extract::<bool>() {
        return Ok(Value::Bool(val));
    }

    if let Ok(val) = obj.extract::<i64>() {
        return Ok(Value::Number(val.into()));
    }

    if let Ok(val) = obj.extract::<f64>() {
        return Ok(Value::Number(
            serde_json::Number::from_f64(val).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid float value")
            })?,
        ));
    }

    if let Ok(val) = obj.extract::<String>() {
        return Ok(Value::String(val));
    }

    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_to_value(&item)?);
        }
        return Ok(Value::Array(arr));
    }

    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::with_capacity(dict.len());
        for (key, val) in dict.iter() {
            let key = key.extract::<String>()?;
            map.insert(key, py_to_value(&val)?);
        }
        return Ok(Value::Object(map));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Unsupported type",
    ))
}
