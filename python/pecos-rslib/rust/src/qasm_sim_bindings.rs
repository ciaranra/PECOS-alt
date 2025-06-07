//! `PyO3` bindings for QASM simulation with enhanced API

use pecos::prelude::*;
use pecos_qasm::simulation::{
    BiasedDepolarizingNoise, BiasedMeasurementNoise, DepolarizingCustomNoise, DepolarizingNoise,
    GeneralNoise, PassThroughNoise,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Convert `PecosError` to `PyErr`
fn pecos_error_to_pyerr(err: &PecosError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// Python-exposed noise model types
#[pyclass(name = "NoiseModel")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyNoiseModelType {
    /// No noise (ideal simulation)
    PassThrough,
    /// Standard depolarizing noise with uniform probability
    Depolarizing,
    /// Depolarizing noise with custom probabilities
    DepolarizingCustom,
    /// Biased depolarizing noise
    BiasedDepolarizing,
    /// Biased measurement noise
    BiasedMeasurement,
    /// General noise model
    General,
}

#[pymethods]
impl PyNoiseModelType {
    #[new]
    fn new(model_type: &str) -> PyResult<Self> {
        match model_type.to_lowercase().replace('_', "").as_str() {
            "passthrough" | "none" => Ok(Self::PassThrough),
            "depolarizing" => Ok(Self::Depolarizing),
            "depolarizingcustom" => Ok(Self::DepolarizingCustom),
            "biaseddepolarizing" => Ok(Self::BiasedDepolarizing),
            "biasedmeasurement" => Ok(Self::BiasedMeasurement),
            "general" => Ok(Self::General),
            _ => Err(PyValueError::new_err(format!(
                "Unknown noise model type: {model_type}"
            ))),
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __str__(&self) -> &'static str {
        match self {
            Self::PassThrough => "PassThrough",
            Self::Depolarizing => "Depolarizing",
            Self::DepolarizingCustom => "DepolarizingCustom",
            Self::BiasedDepolarizing => "BiasedDepolarizing",
            Self::BiasedMeasurement => "BiasedMeasurement",
            Self::General => "General",
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __repr__(&self) -> String {
        format!("NoiseModel.{}", self.__str__())
    }
}

/// Python-exposed quantum engine types
#[pyclass(name = "QuantumEngine")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PyQuantumEngineType {
    /// State vector simulator
    StateVector,
    /// Sparse stabilizer simulator
    SparseStabilizer,
}

impl From<PyQuantumEngineType> for QuantumEngineType {
    fn from(py_engine: PyQuantumEngineType) -> Self {
        match py_engine {
            PyQuantumEngineType::StateVector => QuantumEngineType::StateVector,
            PyQuantumEngineType::SparseStabilizer => QuantumEngineType::SparseStabilizer,
        }
    }
}

#[pymethods]
impl PyQuantumEngineType {
    #[new]
    fn new(engine_type: &str) -> PyResult<Self> {
        match engine_type.to_lowercase().as_str() {
            "statevector" | "state_vector" | "sv" => Ok(Self::StateVector),
            "sparsestabilizer" | "sparse_stabilizer" | "stab" => Ok(Self::SparseStabilizer),
            _ => Err(PyValueError::new_err(format!(
                "Unknown quantum engine type: {engine_type}"
            ))),
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __str__(&self) -> &'static str {
        match self {
            Self::StateVector => "StateVector",
            Self::SparseStabilizer => "SparseStabilizer",
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __repr__(&self) -> String {
        format!("QuantumEngine.{}", self.__str__())
    }
}

/// Convert `ShotVec` to columnar format using `ShotMap`
fn shot_vec_to_columnar_py(py: Python<'_>, shot_vec: &ShotVec) -> PyResult<PyObject> {
    use pyo3::types::PyBytes;

    // Convert to ShotMap for efficient columnar access
    let shot_map = shot_vec
        .try_as_shot_map()
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let py_dict = PyDict::new(py);

    // Get all register names
    let register_names = shot_map.register_names();

    for reg_name in register_names {
        let py_list = PyList::empty(py);

        // Check if this is a BitVec register
        if let Ok(biguint_values) = shot_map.try_bits_as_biguint(reg_name) {
            // Convert BigUint values to Python big integers
            for val in biguint_values {
                // Convert BigUint to Python integer via bytes
                let bytes = val.to_bytes_le();
                let py_int: PyObject = if bytes.is_empty() {
                    0u32.into_pyobject(py)?.into()
                } else {
                    // Create Python int from bytes using int.from_bytes
                    let py_bytes = PyBytes::new(py, &bytes);
                    let int_type = py.import("builtins")?.getattr("int")?;
                    int_type
                        .call_method1("from_bytes", (py_bytes, "little"))?
                        .into()
                };
                py_list.append(py_int)?;
            }
            py_dict.set_item(reg_name, py_list)?;
        } else if let Ok(f64_values) = shot_map.try_f64s(reg_name) {
            // Handle float registers
            for val in f64_values {
                py_list.append(val)?;
            }
            py_dict.set_item(reg_name, py_list)?;
        } else if let Ok(bool_values) = shot_map.try_bools(reg_name) {
            // Handle boolean registers
            for val in bool_values {
                py_list.append(val)?;
            }
            py_dict.set_item(reg_name, py_list)?;
        } else if let Ok(u32_values) = shot_map.try_u32s(reg_name) {
            // Handle u32 registers
            for val in u32_values {
                py_list.append(val)?;
            }
            py_dict.set_item(reg_name, py_list)?;
        }
        // Skip any registers we can't handle
    }

    Ok(py_dict.into())
}

/// Run QASM simulation with a more Pythonic interface
#[pyfunction(name = "run_qasm")]
#[pyo3(signature = (qasm, shots, noise_model=None, engine=None, workers=None, seed=None))]
pub fn py_run_qasm(
    py: Python<'_>,
    qasm: &str,
    shots: usize,
    noise_model: Option<&Bound<'_, PyAny>>,
    engine: Option<PyQuantumEngineType>,
    workers: Option<usize>,
    seed: Option<u64>,
) -> PyResult<PyObject> {
    // Build config directly
    let noise_type = if let Some(nm) = noise_model {
        parse_noise_model(nm)?
    } else {
        NoiseModelType::PassThrough(PassThroughNoise)
    };

    let mut builder = qasm_sim(qasm).noise(noise_type).quantum_engine(
        engine
            .unwrap_or(PyQuantumEngineType::SparseStabilizer)
            .into(),
    );

    if let Some(w) = workers {
        builder = builder.workers(w);
    }

    if let Some(s) = seed {
        builder = builder.seed(s);
    }

    let shot_vec = builder.run(shots).map_err(|e| pecos_error_to_pyerr(&e))?;
    shot_vec_to_columnar_py(py, &shot_vec)
}

/// Get available noise models
#[pyfunction(name = "get_noise_models")]
pub fn py_get_noise_models() -> Vec<&'static str> {
    vec![
        "PassThrough",
        "Depolarizing",
        "DepolarizingCustom",
        "BiasedDepolarizing",
        "BiasedMeasurement",
        "General",
    ]
}

/// Get available quantum engines
#[pyfunction(name = "get_quantum_engines")]
pub fn py_get_quantum_engines() -> Vec<&'static str> {
    vec!["StateVector", "SparseStabilizer"]
}

/// Python wrapper for QasmSimulation
#[pyclass(name = "QasmSimulation")]
pub struct PyQasmSimulation {
    inner: QasmSimulation,
}

#[pymethods]
impl PyQasmSimulation {
    /// Run the simulation with the specified number of shots
    pub fn run(&self, py: Python<'_>, shots: usize) -> PyResult<PyObject> {
        let shot_vec = self
            .inner
            .run(shots)
            .map_err(|e| pecos_error_to_pyerr(&e))?;
        shot_vec_to_columnar_py(py, &shot_vec)
    }
}

/// Python wrapper for QasmSimulationBuilder
#[pyclass(name = "QasmSimulationBuilder")]
#[derive(Clone)]
pub struct PyQasmSimulationBuilder {
    qasm: String,
    seed: Option<u64>,
    workers: usize,
    noise_model: NoiseModelType,
    quantum_engine: QuantumEngineType,
}

#[pymethods]
impl PyQasmSimulationBuilder {
    /// Set the random seed
    pub fn seed(&self, seed: u64) -> Self {
        let mut new = self.clone();
        new.seed = Some(seed);
        new
    }

    /// Set the number of workers
    pub fn workers(&self, workers: usize) -> Self {
        let mut new = self.clone();
        new.workers = workers;
        new
    }

    /// Automatically set workers based on CPU cores
    pub fn auto_workers(&self) -> Self {
        let mut new = self.clone();
        new.workers = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        new
    }

    /// Set the noise model
    pub fn noise(&self, noise_model: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new = self.clone();
        new.noise_model = parse_noise_model(noise_model)?;
        Ok(new)
    }

    /// Set the quantum engine
    pub fn quantum_engine(&self, engine: PyQuantumEngineType) -> Self {
        let mut new = self.clone();
        new.quantum_engine = engine.into();
        new
    }

    /// Build the simulation for repeated execution
    pub fn build(&self) -> PyResult<PyQasmSimulation> {
        let mut builder = qasm_sim(&self.qasm)
            .workers(self.workers)
            .quantum_engine(self.quantum_engine)
            .noise(self.noise_model.clone());

        if let Some(s) = self.seed {
            builder = builder.seed(s);
        }

        let sim = builder.build().map_err(|e| pecos_error_to_pyerr(&e))?;
        Ok(PyQasmSimulation { inner: sim })
    }

    /// Run the simulation directly
    pub fn run(&self, py: Python<'_>, shots: usize) -> PyResult<PyObject> {
        let mut builder = qasm_sim(&self.qasm)
            .workers(self.workers)
            .quantum_engine(self.quantum_engine)
            .noise(self.noise_model.clone());

        if let Some(s) = self.seed {
            builder = builder.seed(s);
        }

        let shot_vec = builder.run(shots).map_err(|e| pecos_error_to_pyerr(&e))?;
        shot_vec_to_columnar_py(py, &shot_vec)
    }
}

/// Create a QASM simulation builder
#[pyfunction(name = "qasm_sim")]
pub fn py_qasm_sim(qasm: &str) -> PyQasmSimulationBuilder {
    PyQasmSimulationBuilder {
        qasm: qasm.to_string(),
        seed: None,
        workers: 1,
        noise_model: NoiseModelType::PassThrough(PassThroughNoise),
        quantum_engine: QuantumEngineType::SparseStabilizer,
    }
}

/// Helper function to parse noise model from Python object
fn parse_noise_model(nm: &Bound<'_, PyAny>) -> PyResult<NoiseModelType> {
    if let Ok(model_type) = nm.extract::<PyNoiseModelType>() {
        // Simple enum variant
        match model_type {
            PyNoiseModelType::PassThrough => Ok(NoiseModelType::PassThrough(PassThroughNoise)),
            PyNoiseModelType::General => Ok(NoiseModelType::General(GeneralNoise)),
            _ => Err(PyValueError::new_err(
                "Enum noise model requires parameters to be specified via noise model classes",
            )),
        }
    } else {
        // Try to extract from Python noise model classes
        let class_name: String = nm.get_type().name()?.extract()?;
        match class_name.as_str() {
            "PassThroughNoise" => Ok(NoiseModelType::PassThrough(PassThroughNoise)),
            "DepolarizingNoise" => {
                let p: f64 = nm.getattr("p")?.extract()?;
                Ok(NoiseModelType::Depolarizing(DepolarizingNoise { p }))
            }
            "DepolarizingCustomNoise" => {
                let p_prep: f64 = nm.getattr("p_prep")?.extract()?;
                let p_meas: f64 = nm.getattr("p_meas")?.extract()?;
                let p1: f64 = nm.getattr("p1")?.extract()?;
                let p2: f64 = nm.getattr("p2")?.extract()?;
                Ok(NoiseModelType::DepolarizingCustom(
                    DepolarizingCustomNoise {
                        p_prep,
                        p_meas,
                        p1,
                        p2,
                    },
                ))
            }
            "BiasedDepolarizingNoise" => {
                let p: f64 = nm.getattr("p")?.extract()?;
                Ok(NoiseModelType::BiasedDepolarizing(
                    BiasedDepolarizingNoise { p },
                ))
            }
            "BiasedMeasurementNoise" => {
                let p0: f64 = nm.getattr("p0")?.extract()?;
                let p1: f64 = nm.getattr("p1")?.extract()?;
                Ok(NoiseModelType::BiasedMeasurement(BiasedMeasurementNoise {
                    p0,
                    p1,
                }))
            }
            "GeneralNoise" => Ok(NoiseModelType::General(GeneralNoise)),
            _ => Err(PyValueError::new_err(format!(
                "Unknown noise model type: {class_name}"
            ))),
        }
    }
}

/// Register all QASM simulation functions with the module
pub fn register_qasm_sim_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyNoiseModelType>()?;
    module.add_class::<PyQuantumEngineType>()?;
    module.add_class::<PyQasmSimulation>()?;
    module.add_class::<PyQasmSimulationBuilder>()?;
    module.add_function(wrap_pyfunction!(py_run_qasm, module)?)?;
    module.add_function(wrap_pyfunction!(py_qasm_sim, module)?)?;
    module.add_function(wrap_pyfunction!(py_get_noise_models, module)?)?;
    module.add_function(wrap_pyfunction!(py_get_quantum_engines, module)?)?;
    Ok(())
}
