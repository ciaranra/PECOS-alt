// PECOS/python/pecos-rslib/rust/src/phir_bridge.rs
use parking_lot::Mutex;
use pecos::prelude::*;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::error::Error;

#[pyclass(module = "_pecos_rslib")]
#[derive(Debug)]
pub struct PHIREngine {
    interpreter: Mutex<PyObject>,
    results: Mutex<HashMap<String, u32>>,
}

impl Clone for PHIREngine {
    fn clone(&self) -> Self {
        // Clone the PyObject - PyObject is Clone so this is safe
        let interp = Python::with_gil(|py| {
            let interpreter_guard = self.interpreter.lock();
            interpreter_guard.clone_ref(py)
        });

        // Clone the results hashmap
        let results_clone = self.results.lock().clone();

        Self {
            interpreter: Mutex::new(interp),
            results: Mutex::new(results_clone),
        }
    }
}

#[pymethods]
impl PHIREngine {
    /// Creates a new `PHIREngine`.
    #[new]
    pub fn py_new(phir_json: &str) -> PyResult<Self> {
        Python::with_gil(|py| {
            let pecos = py.import("pecos.classical_interpreters")?;
            let interpreter_cls = pecos.getattr("PHIRClassicalInterpreter")?;
            let interpreter = interpreter_cls.call0()?;
            interpreter.call_method1("init", (phir_json,))?;

            Ok(Self {
                interpreter: Mutex::new(interpreter.into()),
                results: Mutex::new(HashMap::new()),
            })
        })
    }

    #[getter]
    fn results_dict(&self, py: Python<'_>) -> Py<PyAny> {
        let results = self.results.lock();
        PyObject::from(
            results
                .clone()
                .into_pyobject(py)
                .expect("Failed to convert results"),
        )
    }

    /// Processes the quantum program and returns commands as Python objects
    /// This is a Python-facing method used primarily for testing
    fn process_program(&mut self) -> PyResult<Vec<PyObject>> {
        Python::with_gil(|py| {
            // Get the Python commands from interpreter
            let raw_commands = self.get_raw_commands_from_python(py)?;

            // Convert to Python objects we can return
            let result = self.convert_to_py_commands(py, raw_commands)?;

            Ok(result)
        })
    }

    /// Handles a measurement and updates the Python interpreter
    /// This is a Python-facing method used primarily for testing
    fn handle_measurement(&mut self, measurement: u32) -> PyResult<()> {
        Python::with_gil(|py| self.handle_measurement_internal(py, measurement))
    }

    /// Gets the current results from the engine
    /// This is a Python-facing method used primarily for testing
    fn get_results(&self) -> PyResult<HashMap<String, u32>> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            let py_results = interpreter.call_method0(py, "results")?;

            py_results.extract(py)
        })
    }

    // Helper method to get raw Python commands from the interpreter
    fn get_raw_commands_from_python(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        let interpreter = self.interpreter.lock();
        let program = interpreter.getattr(py, "program")?;
        let ops = program.getattr(py, "ops")?;

        let result = interpreter.call_method1(py, "execute", (ops,))?;

        match result.call_method0(py, "__next__") {
            Ok(commands) => {
                if commands.is_none(py) {
                    Ok(PyList::empty(py).into())
                } else {
                    Ok(commands)
                }
            }
            Err(e) => {
                // Only convert StopIteration to empty list, propagate other errors
                if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                    Ok(PyList::empty(py).into())
                } else {
                    Err(e)
                }
            }
        }
    }

    // Helper to convert Python objects to Python command dicts
    fn convert_to_py_commands(
        &self,
        py: Python<'_>,
        commands: PyObject,
    ) -> PyResult<Vec<PyObject>> {
        if commands.is_none(py) {
            return Ok(Vec::new());
        }

        let py_list = commands.downcast_bound::<PyList>(py)?;
        let mut result = Vec::with_capacity(py_list.len());

        for py_cmd in py_list.iter() {
            let py_dict = PyDict::new(py);
            let name: String = py_cmd.getattr("name")?.extract()?;

            // Create a dict for parameters
            let params_dict = PyDict::new(py);

            // Get arguments (qubits)
            let args = py_cmd.getattr("args")?;
            let mut qubits = Vec::new();

            for item in args.try_iter()? {
                let item = item?;
                let qubit_idx: usize = if item.is_instance_of::<PyList>() {
                    let idx = item.get_item(1)?;
                    idx.extract()?
                } else {
                    item.extract()?
                };
                qubits.push(qubit_idx);
            }

            // Handle different gate types
            match name.as_str() {
                "RZ" => {
                    let angles: Vec<f64> = py_cmd.getattr("angles")?.extract()?;
                    py_dict.set_item("gate_type", "RZ")?;
                    params_dict.set_item("theta", angles[0])?;
                }
                "R1XY" => {
                    let angles: Vec<f64> = py_cmd.getattr("angles")?.extract()?;
                    py_dict.set_item("gate_type", "R1XY")?;
                    params_dict.set_item("angles", angles)?;
                }
                "SZZ" => {
                    py_dict.set_item("gate_type", "SZZ")?;
                }
                "H" => {
                    py_dict.set_item("gate_type", "H")?;
                }
                "X" => {
                    py_dict.set_item("gate_type", "X")?;
                }
                "Y" => {
                    py_dict.set_item("gate_type", "Y")?;
                }
                "Z" => {
                    py_dict.set_item("gate_type", "Z")?;
                }
                "CX" => {
                    py_dict.set_item("gate_type", "CX")?;
                }
                "Measure" => {
                    let returns = py_cmd.getattr("returns")?;
                    let return_item = returns.get_item(0)?;
                    let result_id: usize = return_item.get_item(1)?.extract()?;
                    py_dict.set_item("gate_type", "Measure")?;
                    params_dict.set_item("result_id", result_id)?;
                }
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unsupported gate type: {name}"
                    )));
                }
            }

            py_dict.set_item("params", params_dict)?;

            // Create a separate PyList for qubits
            let qubits_list = PyList::empty(py);
            for &q in &qubits {
                qubits_list.append(q)?;
            }
            py_dict.set_item("qubits", qubits_list)?;

            // Convert to PyObject
            let py_obj: PyObject = py_dict.into_any().into();
            result.push(py_obj);
        }

        Ok(result)
    }

    // Helper to handle a single measurement
    fn handle_measurement_internal(&mut self, py: Python<'_>, measurement: u32) -> PyResult<()> {
        let result_id = measurement >> 16;
        let outcome = measurement & 0xFFFF;

        let interpreter = self.interpreter.lock();
        let dict = PyDict::new(py);
        dict.set_item("measurement", measurement)?;

        let results_guard = self.results.lock();
        let dict_list: Vec<_> = results_guard
            .iter()
            .map(|(key, value)| {
                let py_dict = PyDict::new(py);
                py_dict.set_item("key", key).expect("Failed to set key");
                py_dict
                    .set_item("value", value)
                    .expect("Failed to set value");
                py_dict
                    .into_pyobject(py)
                    .expect("Failed to convert py_dict")
            })
            .collect();

        interpreter.call_method1(py, "receive_results", (dict_list,))?;

        // Store in local cache as well
        drop(results_guard);
        self.results
            .lock()
            .insert(format!("measurement_{}", result_id), outcome);

        Ok(())
    }
}

// Helper function for error conversion
fn to_queue_error<E: std::fmt::Display>(err: E) -> QueueError {
    QueueError::ExecutionError(err.to_string())
}

impl ClassicalEngine for PHIREngine {
    fn num_qubits(&self) -> usize {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            match interpreter.call_method0(py, "num_qubits") {
                Ok(result) => result.extract(py).unwrap_or(0),
                Err(_) => {
                    // Fallback if Python-side doesn't implement num_qubits
                    match interpreter.getattr(py, "program") {
                        Ok(program) => {
                            if let Ok(qvars) = program.getattr(py, "quantum_variables") {
                                if let Ok(total) = qvars.call_method0(py, "total_qubits") {
                                    return total.extract(py).unwrap_or(0);
                                }
                            }
                            0 // Default if we can't get the information
                        }
                        Err(_) => 0,
                    }
                }
            }
        })
    }

    // Generate a CommandBatch from Python commands
    fn process_program(&mut self) -> Result<CommandBatch, QueueError> {
        // Create a CommandBatch
        let mut batch = CommandBatch::new();

        // Fill it with commands from Python
        Python::with_gil(|py| -> Result<(), QueueError> {
            // Get Python commands
            let raw_commands = match self.get_raw_commands_from_python(py) {
                Ok(cmds) => cmds,
                Err(e) => return Err(to_queue_error(e)),
            };

            // Check if empty
            if raw_commands.is_none(py) {
                return Ok(()); // Empty batch
            }

            // Convert to list
            let py_list = match raw_commands.downcast_bound::<PyList>(py) {
                Ok(list) => list,
                Err(e) => return Err(to_queue_error(e)),
            };

            // Process each command
            for py_cmd in py_list.iter() {
                // Get command name
                let name = match py_cmd.getattr("name") {
                    Ok(n) => match n.extract::<String>() {
                        Ok(s) => s,
                        Err(e) => return Err(to_queue_error(e)),
                    },
                    Err(e) => return Err(to_queue_error(e)),
                };

                // Get qubits
                let args = match py_cmd.getattr("args") {
                    Ok(a) => a,
                    Err(e) => return Err(to_queue_error(e)),
                };

                let iter = match args.try_iter() {
                    Ok(i) => i,
                    Err(e) => return Err(to_queue_error(e)),
                };

                let mut qubits = Vec::new();
                for item_result in iter {
                    let item = match item_result {
                        Ok(i) => i,
                        Err(e) => return Err(to_queue_error(e)),
                    };

                    let qubit_idx = if item.is_instance_of::<PyList>() {
                        match item.get_item(1) {
                            Ok(idx) => match idx.extract::<usize>() {
                                Ok(i) => i,
                                Err(e) => return Err(to_queue_error(e)),
                            },
                            Err(e) => return Err(to_queue_error(e)),
                        }
                    } else {
                        match item.extract::<usize>() {
                            Ok(i) => i,
                            Err(e) => return Err(to_queue_error(e)),
                        }
                    };

                    qubits.push(qubit_idx);
                }

                // Create gate based on type
                let gate = match name.as_str() {
                    "RZ" => {
                        let angles = match py_cmd.getattr("angles") {
                            Ok(a) => match a.extract::<Vec<f64>>() {
                                Ok(v) => v,
                                Err(e) => return Err(to_queue_error(e)),
                            },
                            Err(e) => return Err(to_queue_error(e)),
                        };

                        GateType::RZ { theta: angles[0] }
                    }
                    "R1XY" => {
                        let angles = match py_cmd.getattr("angles") {
                            Ok(a) => match a.extract::<Vec<f64>>() {
                                Ok(v) => v,
                                Err(e) => return Err(to_queue_error(e)),
                            },
                            Err(e) => return Err(to_queue_error(e)),
                        };

                        GateType::R1XY {
                            phi: angles[0],
                            theta: angles[1],
                        }
                    }
                    "SZZ" => GateType::SZZ,
                    "H" => GateType::H,
                    "X" => GateType::X,
                    "Y" => GateType::Y,
                    "Z" => GateType::Z,
                    "CX" => GateType::CX,
                    "Measure" => {
                        let returns = match py_cmd.getattr("returns") {
                            Ok(r) => r,
                            Err(e) => return Err(to_queue_error(e)),
                        };

                        let return_item = match returns.get_item(0) {
                            Ok(i) => i,
                            Err(e) => return Err(to_queue_error(e)),
                        };

                        let result_id = match return_item.get_item(1) {
                            Ok(id) => match id.extract::<usize>() {
                                Ok(i) => i,
                                Err(e) => return Err(to_queue_error(e)),
                            },
                            Err(e) => return Err(to_queue_error(e)),
                        };

                        GateType::Measure { result_id }
                    }
                    _ => {
                        return Err(QueueError::OperationError(format!(
                            "Unsupported gate type: {name}"
                        )));
                    }
                };

                // Add command to batch
                batch.add_command(QuantumCommand { gate, qubits });
            }

            Ok(())
        })?;

        Ok(batch)
    }

    fn handle_measurement(&mut self, measurement: u32) -> Result<(), QueueError> {
        Python::with_gil(
            |py| match self.handle_measurement_internal(py, measurement) {
                Ok(_) => Ok(()),
                Err(e) => Err(to_queue_error(e)),
            },
        )
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, QueueError> {
        // Use ClassicalEngine::process_program instead of the Python method
        // This uses the trait implementation which returns a CommandBatch
        match ClassicalEngine::process_program(self) {
            Ok(batch) => {
                // Then convert to ByteMessage
                if batch.is_empty() {
                    ByteMessage::create_flush(true)
                } else {
                    ByteMessage::create_quantum_operations(&batch)
                }
            }
            Err(e) => Err(e), // This is now QueueError -> QueueError
        }
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), QueueError> {
        // Parse measurements from the message
        match message.parse_measurements() {
            Ok(measurements) => {
                // Process each measurement
                for measurement in measurements {
                    // Use ClassicalEngine::handle_measurement which returns Result<(), QueueError>
                    ClassicalEngine::handle_measurement(self, measurement)?;
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn get_results(&self) -> Result<ShotResult, QueueError> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();

            let py_results = match interpreter.call_method0(py, "results") {
                Ok(r) => r,
                Err(e) => return Err(to_queue_error(e)),
            };

            let results: HashMap<String, u32> = match py_results.extract(py) {
                Ok(r) => r,
                Err(e) => return Err(to_queue_error(e)),
            };

            (*self.results.lock()).clone_from(&results);

            Ok(ShotResult {
                measurements: results,
            })
        })
    }

    fn compile(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ClassicalEngine> {
        Box::new(self.clone())
    }

    fn reset(&mut self) -> Result<(), QueueError> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            match interpreter.call_method0(py, "reset") {
                Ok(_) => {
                    (*self.results.lock()).clear();
                    Ok(())
                }
                Err(e) => Err(to_queue_error(e)),
            }
        })
    }
}

impl ControlEngine for PHIREngine {
    type Input = ();
    type Output = ShotResult;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn reset(&mut self) -> Result<(), QueueError> {
        ClassicalEngine::reset(self)
    }

    fn start(&mut self, _input: ()) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
        // Reset state to ensure clean start
        if let Err(e) = ClassicalEngine::reset(self) {
            return Err(e);
        }

        // Get commands as ByteMessage
        let commands = match self.generate_commands() {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // Check if the message is empty (just a flush)
        let is_empty = match commands.is_empty() {
            Ok(empty) => empty,
            Err(e) => return Err(e),
        };

        if is_empty {
            // Get the results directly
            match ClassicalEngine::get_results(self) {
                Ok(results) => Ok(EngineStage::Complete(results)),
                Err(e) => Err(e),
            }
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> Result<EngineStage<ByteMessage, ShotResult>, QueueError> {
        // Handle received measurements
        if let Err(e) = self.handle_measurements(measurements) {
            return Err(e);
        }

        // Get next batch of commands
        let commands = match self.generate_commands() {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // Check if we have an empty message (no more commands)
        let is_empty = match commands.is_empty() {
            Ok(empty) => empty,
            Err(e) => return Err(e),
        };

        if is_empty {
            // Get the results directly
            match ClassicalEngine::get_results(self) {
                Ok(results) => Ok(EngineStage::Complete(results)),
                Err(e) => Err(e),
            }
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }
}
