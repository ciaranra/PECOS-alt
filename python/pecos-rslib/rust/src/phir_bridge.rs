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
    ///
    /// # Arguments
    ///
    /// * `phir_json` - A JSON string representing the initial configuration of the PHIREngine.
    ///
    /// # Returns
    ///
    /// Returns an instance of `PHIREngine` wrapped in a `PyResult`.
    ///
    /// # Errors
    ///
    /// This function will return a `PyErr` if:
    /// - The `pecos.classical_interpreters` module cannot be imported.
    /// - The `PHIRClassicalInterpreter` class is not found.
    /// - Any method call on the Python interpreter fails.
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

    fn process_program(&mut self) -> PyResult<Vec<PyObject>> {
        Python::with_gil(|py| {
            // Call our implementation that returns CommandBatch
            let batch = ClassicalEngine::process_program(self)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

            let mut py_commands = Vec::with_capacity(batch.len());

            // Convert each command to a Python dict
            for cmd in batch.commands() {
                let py_dict = PyDict::new(py);

                // Create a dict for parameters
                let params_dict = PyDict::new(py);

                // Convert gate type and parameters
                match &cmd.gate {
                    GateType::Measure { result_id } => {
                        py_dict.set_item("gate_type", "Measure")?;
                        params_dict.set_item("result_id", result_id)?;
                    }
                    GateType::RZ { theta } => {
                        py_dict.set_item("gate_type", "RZ")?;
                        params_dict.set_item("theta", theta)?;
                    }
                    GateType::R1XY { phi, theta } => {
                        py_dict.set_item("gate_type", "R1XY")?;
                        let angles = vec![phi, theta];
                        params_dict.set_item("angles", angles)?;
                    }
                    GateType::SZZ => {
                        py_dict.set_item("gate_type", "SZZ")?;
                    }
                    GateType::H => {
                        py_dict.set_item("gate_type", "H")?;
                    }
                    GateType::CX => {
                        py_dict.set_item("gate_type", "CX")?;
                    }
                }

                py_dict.set_item("params", params_dict)?;
                py_dict.set_item("qubits", &cmd.qubits)?;

                // Convert to PyObject
                let py_obj: PyObject = py_dict.into_any().into();
                py_commands.push(py_obj);
            }
            Ok(py_commands)
        })
    }

    fn handle_measurement(&mut self, measurement: u32) -> PyResult<()> {
        ClassicalEngine::handle_measurement(self, measurement)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn get_results(&self) -> PyResult<HashMap<String, u32>> {
        ClassicalEngine::get_results(self)
            .map(|shot_result| shot_result.measurements)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

impl ClassicalEngine for PHIREngine {
    fn num_qubits(&self) -> usize {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            match interpreter.call_method0(py, "num_qubits") {
                Ok(result) => result.extract(py).unwrap_or(0),
                Err(_) => {
                    // Fallback if Python-side doesn't implement num_qubits
                    // Try to get program and count quantum variables
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

    fn process_program(&mut self) -> Result<CommandBatch, QueueError> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            let program = interpreter
                .getattr(py, "program")
                .map_err(|e| py_err_to_queue_error(&e))?;
            let ops = program
                .getattr(py, "ops")
                .map_err(|e| py_err_to_queue_error(&e))?;
            let result = interpreter
                .call_method1(py, "execute", (ops,))
                .map_err(|e| py_err_to_queue_error(&e))?;

            match result.call_method0(py, "__next__") {
                Ok(commands) if commands.is_none(py) => Ok(CommandBatch::new()),
                Ok(commands) => {
                    // Use closure to handle DowncastError properly
                    let py_list = commands
                        .downcast_bound::<PyList>(py)
                        .map_err(to_queue_error)?;

                    let mut batch = CommandBatch::new();
                    for py_cmd in py_list.iter() {
                        let (gate, qubits) = convert_gate(&py_cmd).map_err(to_queue_error)?;
                        batch.add_command(QuantumCommand { gate, qubits });
                    }
                    Ok(batch)
                }
                Err(e) => {
                    // Only convert StopIteration to empty batch, propagate other errors
                    if e.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                        Ok(CommandBatch::new())
                    } else {
                        Err(to_queue_error(e))
                    }
                }
            }
        })
    }

    fn handle_measurement(&mut self, measurement: Message) -> Result<(), QueueError> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            let dict = PyDict::new(py);
            dict.set_item("measurement", measurement)
                .map_err(to_queue_error)?;

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

            interpreter
                .call_method1(py, "receive_results", (dict_list,))
                .map_err(|e| py_err_to_queue_error(&e))?;

            Ok(())
        })
    }

    fn get_results(&self) -> Result<ShotResult, QueueError> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            let py_results = interpreter
                .call_method0(py, "results")
                .map_err(|e| py_err_to_queue_error(&e))?;

            let results: HashMap<String, u32> = py_results.extract(py).map_err(to_queue_error)?;

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
}

fn convert_gate(py_cmd: &Bound<'_, PyAny>) -> Result<(GateType, Vec<usize>), PyErr> {
    let name: String = py_cmd.getattr("name")?.extract()?;
    let args = py_cmd.getattr("args")?;

    let mut qubits = Vec::new();
    for item in args.try_iter()? {
        let item = item?;
        let qubit_idx = if item.is_instance_of::<PyList>() {
            item.get_item(1)?.extract()?
        } else {
            item.extract()?
        };
        qubits.push(qubit_idx);
    }

    let gate = match name.as_str() {
        "RZ" => {
            let angles: Vec<f64> = py_cmd.getattr("angles")?.extract()?;
            GateType::RZ { theta: angles[0] }
        }
        "R1XY" => {
            let angles: Vec<f64> = py_cmd.getattr("angles")?.extract()?;
            GateType::R1XY {
                phi: angles[0],
                theta: angles[1],
            }
        }
        "SZZ" => GateType::SZZ,
        "H" => GateType::H,
        "CX" => GateType::CX,
        "Measure" => {
            let returns = py_cmd.getattr("returns")?;
            let return_item = returns.get_item(0)?;
            let result_id: usize = return_item.get_item(1)?.extract()?;
            GateType::Measure { result_id }
        }
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unsupported gate type: {name}"
            )));
        }
    };

    Ok((gate, qubits))
}

// Generic error conversion function
fn to_queue_error<E: std::fmt::Display>(err: E) -> QueueError {
    QueueError::ExecutionError(err.to_string())
}

// PyErr specific conversion - take a reference instead of by value
fn py_err_to_queue_error(err: &PyErr) -> QueueError {
    QueueError::ExecutionError(err.to_string())
}

impl ControlEngine for PHIREngine {
    type Input = ();
    type Output = ShotResult;
    type EngineInput = CommandBatch;
    type EngineOutput = Vec<Message>;

    fn reset(&mut self) -> Result<(), QueueError> {
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            interpreter
                .call_method0(py, "reset")
                .map_err(|e| py_err_to_queue_error(&e))?;
            (*self.results.lock()).clear();
            Ok(())
        })
    }

    fn start(&mut self, _input: ()) -> Result<EngineStage<CommandBatch, ShotResult>, QueueError> {
        // Reset state to ensure clean start
        Python::with_gil(|py| {
            let interpreter = self.interpreter.lock();
            interpreter
                .call_method0(py, "reset")
                .map_err(|e| py_err_to_queue_error(&e))?;
            (*self.results.lock()).clear();
            Ok::<(), QueueError>(())
        })?;

        // Get commands to process using the ClassicalEngine implementation
        let commands = ClassicalEngine::process_program(self)?;

        if commands.is_empty() {
            let results = ClassicalEngine::get_results(self)?;
            Ok(EngineStage::Complete(results))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn continue_processing(
        &mut self,
        measurements: Vec<Message>,
    ) -> Result<EngineStage<CommandBatch, ShotResult>, QueueError> {
        // Handle received measurements
        for measurement in measurements {
            ClassicalEngine::handle_measurement(self, measurement)?;
        }

        // Get next batch of commands
        let commands = ClassicalEngine::process_program(self)?;

        if commands.is_empty() {
            let results = ClassicalEngine::get_results(self)?;
            Ok(EngineStage::Complete(results))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }
}
