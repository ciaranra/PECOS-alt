//! `PyO3` bindings for QASM simulation with enhanced API

use crate::noise_helpers::{
    get_optional_bool, get_optional_dict, get_optional_f64, validate_and_convert_seed,
};
use pecos::prelude::*;
use pecos_engines::noise::GeneralNoiseModelBuilder;
use pecos_qasm::config::{parse_gate_type_from_string, NoiseConfig};
use pecos_qasm::simulation::{
    BiasedDepolarizingNoise, BitVecFormat, DepolarizingCustomNoise, DepolarizingNoise,
    PassThroughNoise,
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
fn shot_vec_to_columnar_py(
    py: Python<'_>,
    shot_vec: &ShotVec,
    bit_format: BitVecFormat,
) -> PyResult<PyObject> {
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

        // Check if this is a BitVec register and handle format
        if bit_format == BitVecFormat::BinaryString {
            // Try to get as binary strings
            if let Ok(binary_values) = shot_map.try_bits_as_binary(reg_name) {
                for val in binary_values {
                    py_list.append(val.into_pyobject(py)?)?;
                }
                py_dict.set_item(reg_name, py_list)?;
            }
        } else if let Ok(biguint_values) = shot_map.try_bits_as_biguint(reg_name) {
            // Default BigInt format
            for val in biguint_values {
                let bytes = val.to_bytes_le();
                let py_int: PyObject = if bytes.is_empty() {
                    0u32.into_pyobject(py)?.into()
                } else {
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
    shot_vec_to_columnar_py(py, &shot_vec, BitVecFormat::BigInt)
}

/// Get available noise models
#[pyfunction(name = "get_noise_models")]
pub fn py_get_noise_models() -> Vec<&'static str> {
    vec![
        "PassThrough",
        "Depolarizing",
        "DepolarizingCustom",
        "BiasedDepolarizing",
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
        shot_vec_to_columnar_py(py, &shot_vec, self.inner.bit_format())
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
    bit_format: BitVecFormat,
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

    /// Set the output format to binary strings
    pub fn with_binary_string_format(&self) -> Self {
        let mut new = self.clone();
        new.bit_format = BitVecFormat::BinaryString;
        new
    }

    /// Apply configuration from a dictionary
    pub fn config(&self, py: Python<'_>, config: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut new = self.clone();

        // Convert Python dict to JSON for Rust processing
        let json_str = py
            .import("json")?
            .getattr("dumps")?
            .call1((config,))?
            .extract::<String>()?;
        let json_val: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse config as JSON: {e}")))?;

        // Apply each configuration field
        if let Some(seed_val) = json_val.get("seed") {
            if let Some(seed) = seed_val.as_u64() {
                new.seed = Some(seed);
            } else {
                return Err(PyValueError::new_err("Invalid seed value"));
            }
        }

        if let Some(workers_val) = json_val.get("workers") {
            if let Some(workers_str) = workers_val.as_str() {
                if workers_str == "auto" {
                    new.workers = std::thread::available_parallelism()
                        .map(std::num::NonZero::get)
                        .unwrap_or(4);
                } else {
                    return Err(PyValueError::new_err(format!(
                        "Invalid worker config '{workers_str}', expected 'auto' or a number"
                    )));
                }
            } else if let Some(workers) = workers_val.as_u64() {
                new.workers = usize::try_from(workers)
                    .map_err(|_| PyValueError::new_err("Workers value too large"))?;
            } else {
                return Err(PyValueError::new_err("Invalid workers value"));
            }
        }

        if let Some(noise_val) = json_val.get("noise") {
            // Skip if noise is explicitly null
            if !noise_val.is_null() {
                // Parse noise from JSON config
                let noise_config: NoiseConfig =
                    serde_json::from_value(noise_val.clone()).map_err(|e| {
                        PyValueError::new_err(format!("Invalid noise configuration: {e}"))
                    })?;
                new.noise_model = noise_config.into();
            }
        }

        if let Some(engine_val) = json_val.get("quantum_engine") {
            if let Some(engine_str) = engine_val.as_str() {
                new.quantum_engine = match engine_str {
                    "StateVector" | "state_vector" => PyQuantumEngineType::StateVector,
                    "SparseStabilizer" | "sparse_stabilizer" => {
                        PyQuantumEngineType::SparseStabilizer
                    }
                    _ => {
                        return Err(PyValueError::new_err(format!(
                            "Unknown quantum engine: {engine_str}"
                        )));
                    }
                }
                .into();
            } else {
                return Err(PyValueError::new_err("Invalid quantum_engine value"));
            }
        }

        if let Some(binary_val) = json_val.get("binary_string_format") {
            if let Some(binary) = binary_val.as_bool() {
                if binary {
                    new.bit_format = BitVecFormat::BinaryString;
                }
            } else {
                return Err(PyValueError::new_err("Invalid binary_string_format value"));
            }
        }

        Ok(new)
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

        if self.bit_format == BitVecFormat::BinaryString {
            builder = builder.with_binary_string_format();
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

        if self.bit_format == BitVecFormat::BinaryString {
            builder = builder.with_binary_string_format();
        }

        let shot_vec = builder.run(shots).map_err(|e| pecos_error_to_pyerr(&e))?;
        shot_vec_to_columnar_py(py, &shot_vec, self.bit_format)
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
        bit_format: BitVecFormat::BigInt,
    }
}

/// Helper function to apply global parameters to the builder
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Seed cast is validated
fn apply_global_params(
    nm: &Bound<'_, PyAny>,
    mut builder: GeneralNoiseModelBuilder,
) -> PyResult<GeneralNoiseModelBuilder> {
    // Global parameters
    if let Ok(Some(gates)) = nm.getattr("noiseless_gates").and_then(|v| {
        if v.is_none() {
            Ok(None)
        } else {
            v.extract::<Vec<String>>().map(Some)
        }
    }) {
        for gate_str in gates {
            if let Some(gate_type) = parse_gate_type_from_string(&gate_str) {
                builder = builder.with_noiseless_gate(gate_type);
            }
        }
    }

    if let Some(s) = get_optional_f64(nm, "seed")? {
        let seed = validate_and_convert_seed(s)?;
        builder = builder.with_seed(seed);
    }
    if let Some(s) = get_optional_f64(nm, "scale")? {
        builder = builder.with_scale(s);
    }
    if let Some(s) = get_optional_f64(nm, "leakage_scale")? {
        builder = builder.with_leakage_scale(s);
    }
    if let Some(s) = get_optional_f64(nm, "emission_scale")? {
        builder = builder.with_emission_scale(s);
    }

    Ok(builder)
}

/// Helper function to apply idle noise parameters to the builder
fn apply_idle_params(
    nm: &Bound<'_, PyAny>,
    mut builder: GeneralNoiseModelBuilder,
) -> PyResult<GeneralNoiseModelBuilder> {
    if let Some(v) = get_optional_bool(nm, "p_idle_coherent")? {
        builder = builder.with_p_idle_coherent(v);
    }
    if let Some(v) = get_optional_f64(nm, "p_idle_linear_rate")? {
        builder = builder.with_p_idle_linear_rate(v);
    }
    if let Some(model) = get_optional_dict(nm, "p_idle_linear_model")? {
        builder = builder.with_p_idle_linear_model(&model);
    }
    if let Some(v) = get_optional_f64(nm, "p_idle_quadratic_rate")? {
        builder = builder.with_p_idle_quadratic_rate(v);
    }
    if let Some(v) = get_optional_f64(nm, "p_idle_coherent_to_incoherent_factor")? {
        builder = builder.with_p_idle_coherent_to_incoherent_factor(v);
    }
    if let Some(s) = get_optional_f64(nm, "idle_scale")? {
        builder = builder.with_idle_scale(s);
    }

    Ok(builder)
}

/// Helper function to apply prep noise parameters to the builder
fn apply_prep_params(
    nm: &Bound<'_, PyAny>,
    mut builder: GeneralNoiseModelBuilder,
) -> PyResult<GeneralNoiseModelBuilder> {
    if let Some(v) = get_optional_f64(nm, "p_prep")? {
        builder = builder.with_prep_probability(v);
    }
    if let Some(v) = get_optional_f64(nm, "p_prep_leak_ratio")? {
        builder = builder.with_prep_leak_ratio(v);
    }
    if let Some(v) = get_optional_f64(nm, "p_prep_crosstalk")? {
        builder = builder.with_p_prep_crosstalk(v);
    }
    if let Some(s) = get_optional_f64(nm, "prep_scale")? {
        builder = builder.with_prep_scale(s);
    }
    if let Some(s) = get_optional_f64(nm, "p_prep_crosstalk_scale")? {
        builder = builder.with_p_prep_crosstalk_scale(s);
    }

    Ok(builder)
}

/// Helper function to apply single-qubit gate noise parameters to the builder
fn apply_single_qubit_params(
    nm: &Bound<'_, PyAny>,
    mut builder: GeneralNoiseModelBuilder,
) -> PyResult<GeneralNoiseModelBuilder> {
    if let Some(v) = get_optional_f64(nm, "p1")? {
        builder = builder.with_p1_probability(v);
    }
    if let Some(v) = get_optional_f64(nm, "p1_emission_ratio")? {
        builder = builder.with_p1_emission_ratio(v);
    }
    if let Some(model) = get_optional_dict(nm, "p1_emission_model")? {
        builder = builder.with_p1_emission_model(&model);
    }
    if let Some(v) = get_optional_f64(nm, "p1_seepage_prob")? {
        builder = builder.with_p1_seepage_prob(v);
    }
    if let Some(model) = get_optional_dict(nm, "p1_pauli_model")? {
        builder = builder.with_p1_pauli_model(&model);
    }
    if let Some(s) = get_optional_f64(nm, "p1_scale")? {
        builder = builder.with_p1_scale(s);
    }

    Ok(builder)
}

/// Helper function to apply two-qubit gate noise parameters to the builder
fn apply_two_qubit_params(
    nm: &Bound<'_, PyAny>,
    mut builder: GeneralNoiseModelBuilder,
) -> PyResult<GeneralNoiseModelBuilder> {
    if let Some(v) = get_optional_f64(nm, "p2")? {
        builder = builder.with_p2_probability(v);
    }
    // Handle angle params tuple
    if let Ok(Some(params)) = nm.getattr("p2_angle_params").and_then(|v| {
        if v.is_none() {
            Ok(None)
        } else {
            let tuple = v.extract::<(f64, f64, f64, f64)>()?;
            Ok(Some(tuple))
        }
    }) {
        builder = builder.with_p2_angle_params(params.0, params.1, params.2, params.3);
    }
    if let Some(v) = get_optional_f64(nm, "p2_angle_power")? {
        builder = builder.with_p2_angle_power(v);
    }
    if let Some(v) = get_optional_f64(nm, "p2_emission_ratio")? {
        builder = builder.with_p2_emission_ratio(v);
    }
    if let Some(model) = get_optional_dict(nm, "p2_emission_model")? {
        builder = builder.with_p2_emission_model(&model);
    }
    if let Some(v) = get_optional_f64(nm, "p2_seepage_prob")? {
        builder = builder.with_p2_seepage_prob(v);
    }
    if let Some(model) = get_optional_dict(nm, "p2_pauli_model")? {
        builder = builder.with_p2_pauli_model(&model);
    }
    if let Some(v) = get_optional_f64(nm, "p2_idle_quadratic_rate")? {
        builder = builder.with_p2_idle_quadratic_rate(v);
    }
    if let Some(s) = get_optional_f64(nm, "p2_scale")? {
        builder = builder.with_p2_scale(s);
    }

    Ok(builder)
}

/// Helper function to apply measurement noise parameters to the builder
fn apply_meas_params(
    nm: &Bound<'_, PyAny>,
    mut builder: GeneralNoiseModelBuilder,
) -> PyResult<GeneralNoiseModelBuilder> {
    if let Some(v) = get_optional_f64(nm, "p_meas_0")? {
        builder = builder.with_meas_0_probability(v);
    }
    if let Some(v) = get_optional_f64(nm, "p_meas_1")? {
        builder = builder.with_meas_1_probability(v);
    }
    if let Some(v) = get_optional_f64(nm, "p_meas_crosstalk")? {
        builder = builder.with_p_meas_crosstalk(v);
    }
    if let Some(s) = get_optional_f64(nm, "meas_scale")? {
        builder = builder.with_meas_scale(s);
    }
    if let Some(s) = get_optional_f64(nm, "p_meas_crosstalk_scale")? {
        builder = builder.with_p_meas_crosstalk_scale(s);
    }

    Ok(builder)
}

/// Helper function to parse noise model from Python object
fn parse_noise_model(nm: &Bound<'_, PyAny>) -> PyResult<NoiseModelType> {
    if let Ok(model_type) = nm.extract::<PyNoiseModelType>() {
        // Simple enum variant
        match model_type {
            PyNoiseModelType::PassThrough => Ok(NoiseModelType::PassThrough(PassThroughNoise)),
            PyNoiseModelType::General => {
                // For the enum case, create default general noise
                Ok(NoiseModelType::GeneralFromBuilder(Box::new(
                    GeneralNoiseModelBuilder::new(),
                )))
            }
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
            "GeneralNoise" => {
                // Create builder and apply all parameters
                let mut builder = GeneralNoiseModelBuilder::new();

                // Apply all parameter groups
                builder = apply_global_params(nm, builder)?;
                builder = apply_idle_params(nm, builder)?;
                builder = apply_prep_params(nm, builder)?;
                builder = apply_single_qubit_params(nm, builder)?;
                builder = apply_two_qubit_params(nm, builder)?;
                builder = apply_meas_params(nm, builder)?;

                Ok(NoiseModelType::GeneralFromBuilder(Box::new(builder)))
            }
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
