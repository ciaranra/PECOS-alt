//! Thin PyO3 bindings that directly mirror the unified Rust QASM simulation API

use pecos_engines::shot_results::ShotVec;
use pecos_qasm::{qasm_engine, QasmEngineBuilder};
use pecos_engines::{ClassicalControlEngineBuilder};
use pecos_engines::noise::{
    DepolarizingNoiseModelBuilder, BiasedDepolarizingNoiseModelBuilder, 
    GeneralNoiseModelBuilder
};
use pecos_programs::QasmProgram;
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::collections::HashMap;

/// Convert `PecosError` to `PyErr`
fn pecos_error_to_pyerr(err: pecos_core::errors::PecosError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// Convert ShotVec to HashMap with binary string values
fn convert_shot_vec_to_binary_dict(shot_vec: ShotVec) -> HashMap<String, Vec<String>> {
    let shot_map = match shot_vec.try_as_shot_map() {
        Ok(map) => map,
        Err(_) => {
            // Fallback for empty results
            return HashMap::new();
        }
    };
    
    let mut result = HashMap::new();
    
    // Iterate over all registers
    for name in shot_map.register_names() {
        // Try to get binary strings for BitVec registers
        if let Ok(binary_strings) = shot_map.try_bits_as_binary(name) {
            result.insert(name.to_string(), binary_strings);
        }
        // For non-BitVec registers, try to convert to string representations
        else if let Ok(u32_values) = shot_map.try_u32s(name) {
            result.insert(name.to_string(), u32_values.into_iter().map(|v| v.to_string()).collect());
        }
        else if let Ok(i64_values) = shot_map.try_i64s(name) {
            result.insert(name.to_string(), i64_values.into_iter().map(|v| v.to_string()).collect());
        }
        // Skip registers we can't convert
    }
    
    result
}

/// Python wrapper for `GeneralNoiseModelBuilder`
#[pyclass(name = "GeneralNoiseModelBuilder", module = "pecos_rslib._pecos_rslib")]
#[derive(Debug, Clone)]
pub struct PyGeneralNoiseModelBuilder {
    inner: GeneralNoiseModelBuilder,
}

#[pymethods]
impl PyQasmSimBuilder {
    /// Set the random seed
    #[pyo3(text_signature = "($self, seed)")]
    fn seed(mut self_: PyRefMut<'_, Self>, seed: u64) -> PyResult<()> {
        if let Some(builder) = self_.inner.take() {
            self_.inner = Some(builder.seed(seed));
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    // Global parameter setters
    /// Mark a specific gate type as noiseless.
    ///
    /// Args:
    ///     gate: Gate name (e.g., "H", "X", "CX", "MEASURE")
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If gate type is unknown
    #[pyo3(text_signature = "($self, gate)")]
    fn with_noiseless_gate(&self, gate: &str) -> PyResult<Self> {
        let mut new_self = self.clone();
        if let Some(gate_type) = parse_gate_type_from_string(gate) {
            new_self.inner = new_self.inner.with_noiseless_gate(gate_type);
            Ok(new_self)
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Set the random number generator seed for reproducible noise.
    ///
    /// Args:
    ///     seed: Random seed value (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    #[pyo3(text_signature = "($self, seed)")]
    fn with_seed(&self, seed: u64) -> Self {
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_seed(seed);
        new_self
    }

    /// Set global scaling factor for all error rates.
    ///
    /// This multiplies all error probabilities by the given factor,
    /// useful for studying noise threshold behavior.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("scale must be non-negative"));
        }
    }

    /// Set the leakage vs depolarizing ratio.
    ///
    /// Controls how much of the error budget goes to leakage (qubit
    /// leaving computational subspace) vs depolarizing errors.
    ///
    /// Args:
    ///     scale: Leakage scale between 0.0 (no leakage) and 1.0 (all leakage)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is not between 0 and 1
    #[pyo3(text_signature = "($self, scale)")]
    fn with_leakage_scale(&self, scale: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&scale) {
            return Err(PyValueError::new_err(
                "leakage_scale must be between 0 and 1",
            ));
        }
    }

    /// Set scaling factor for spontaneous emission errors.
    ///
    /// Args:
    ///     scale: Emission scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_emission_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("emission_scale must be non-negative"));
        }
    }

    /// Set the global seepage probability for leaked qubits.
    ///
    /// This sets the seepage probability for both single-qubit and two-qubit gates.
    ///
    /// Args:
    ///     prob: Seepage probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If prob is not between 0 and 1
    #[pyo3(text_signature = "($self, prob)")]
    fn with_seepage_prob(&self, prob: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&prob) {
            return Err(PyValueError::new_err(
                "seepage_prob must be between 0 and 1",
            ));
        }
    }

    // Idle noise setters
    /// Set whether to use coherent vs incoherent dephasing.
    ///
    /// Args:
    ///     `use_coherent`: If True, use coherent dephasing. If False, use incoherent.
    ///
    /// Returns:
    ///     Self for method chaining
    #[pyo3(text_signature = "($self, use_coherent)")]
    fn with_p_idle_coherent(&self, use_coherent: bool) -> Self {
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p_idle_coherent(use_coherent);
        new_self
    }

    /// Set the idle noise linear rate.
    ///
    /// Args:
    ///     rate: Linear rate (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If rate is negative
    #[pyo3(text_signature = "($self, rate)")]
    fn with_p_idle_linear_rate(&self, rate: f64) -> PyResult<Self> {
        if rate < 0.0 {
            return Err(PyValueError::new_err(
                "p_idle_linear_rate must be non-negative",
            ));
        }
    }

    /// Set the average idle noise linear rate.
    ///
    /// Args:
    ///     rate: Average linear rate (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If rate is negative
    #[pyo3(text_signature = "($self, rate)")]
    fn with_average_p_idle_linear_rate(&self, rate: f64) -> PyResult<Self> {
        if rate < 0.0 {
            return Err(PyValueError::new_err(
                "p_average_idle_linear_rate must be non-negative",
            ));
        }
    }
}

/// Python wrapper for DepolarizingNoiseModelBuilder
#[pyclass(name = "DepolarizingNoiseModelBuilder", module = "pecos_rslib._pecos_rslib")]
#[derive(Clone)]
pub struct PyDepolarizingNoiseModelBuilder {
    pub(crate) inner: DepolarizingNoiseModelBuilder,
}

    /// Set the idle noise quadratic rate.
    ///
    /// Args:
    ///     rate: Quadratic rate (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If rate is negative
    #[pyo3(text_signature = "($self, rate)")]
    fn with_p_idle_quadratic_rate(&self, rate: f64) -> PyResult<Self> {
        if rate < 0.0 {
            return Err(PyValueError::new_err(
                "p_idle_quadratic_rate must be non-negative",
            ));
        }
    }

    /// Set the average idle noise quadratic rate.
    ///
    /// Args:
    ///     rate: Average quadratic rate (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If rate is negative
    #[pyo3(text_signature = "($self, rate)")]
    fn with_average_p_idle_quadratic_rate(&self, rate: f64) -> PyResult<Self> {
        if rate < 0.0 {
            return Err(PyValueError::new_err(
                "p_average_idle_quadratic_rate must be non-negative",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_average_p_idle_quadratic_rate(rate);
        Ok(new_self)
    }

    /// Set the coherent to incoherent conversion factor.
    ///
    /// Args:
    ///     factor: Conversion factor (must be positive)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If factor is not positive
    #[pyo3(text_signature = "($self, factor)")]
    fn with_p_idle_coherent_to_incoherent_factor(&self, factor: f64) -> PyResult<Self> {
        if factor <= 0.0 {
            return Err(PyValueError::new_err(
                "p_idle_coherent_to_incoherent_factor must be positive",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self
            .inner
            .with_p_idle_coherent_to_incoherent_factor(factor);
        Ok(new_self)
    }

    /// Set the idle noise scaling factor.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_idle_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("idle_scale must be non-negative"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_idle_scale(scale);
        Ok(new_self)
    }

    // Preparation noise setters
    /// Set error probability during qubit state preparation.
    ///
    /// Args:
    ///     p: Error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }

    /// Set the preparation leakage ratio.
    ///
    /// Args:
    ///     ratio: Fraction of preparation errors that result in leakage (0.0 to 1.0)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If ratio is not between 0 and 1
    #[pyo3(text_signature = "($self, ratio)")]
    fn with_prep_leak_ratio(&self, ratio: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&ratio) {
            return Err(PyValueError::new_err(
                "prep_leak_ratio must be between 0 and 1",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_prep_leak_ratio(ratio);
        Ok(new_self)
    }

    /// Set the preparation crosstalk probability.
    ///
    /// Args:
    ///     p: Crosstalk probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_meas_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_probability(p)
        })
    }

    /// Set the preparation error scaling factor.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_prep_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("prep_scale must be non-negative"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_prep_scale(scale);
        Ok(new_self)
    }

    /// Set the preparation crosstalk scaling factor.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_p_prep_crosstalk_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err(
                "p_prep_crosstalk_scale must be non-negative",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p_prep_crosstalk_scale(scale);
        Ok(new_self)
    }

    // Single-qubit gate noise setters
    /// Set total error probability after single-qubit gates.
    ///
    /// This is the total probability of any error occurring after
    /// a single-qubit gate operation.
    ///
    /// Args:
    ///     p: Total error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_p1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p1_probability(p)
        })
    }

    /// Set average error probability for single-qubit gates.
    ///
    /// This sets the average gate infidelity, which is automatically
    /// converted to total error probability (multiplied by 1.5).
    ///
    /// Args:
    ///     p: Average error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_average_p1_probability(&self, p: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&p) {
            return Err(PyValueError::new_err("p1 must be between 0 and 1"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_average_p1_probability(p);
        Ok(new_self)
    }

    /// Set the emission ratio for single-qubit gate errors.
    ///
    /// Args:
    ///     ratio: Fraction of errors that are emission errors (0.0 to 1.0)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If ratio is not between 0 and 1
    #[pyo3(text_signature = "($self, ratio)")]
    fn with_p1_emission_ratio(&self, ratio: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&ratio) {
            return Err(PyValueError::new_err(
                "p1_emission_ratio must be between 0 and 1",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p1_emission_ratio(ratio);
        Ok(new_self)
    }

    /// Set the emission error model for single-qubit gates.
    ///
    /// Args:
    ///     model: Dictionary mapping Pauli operators to probabilities
    ///
    /// Returns:
    ///     Self for method chaining
    #[pyo3(text_signature = "($self, model)")]
    fn with_p1_emission_model(&self, model: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut btree_model = BTreeMap::new();
        for (key, value) in model.iter() {
            let key_str: String = key.extract()?;
            let value_f64: f64 = value.extract()?;
            btree_model.insert(key_str, value_f64);
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p1_emission_model(&btree_model);
        Ok(new_self)
    }

    /// Set the seepage probability for single-qubit gates.
    ///
    /// Args:
    ///     prob: Probability of seeping leaked qubits (0.0 to 1.0)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If prob is not between 0 and 1
    #[pyo3(text_signature = "($self, prob)")]
    fn with_p1_seepage_prob(&self, prob: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&prob) {
            return Err(PyValueError::new_err(
                "p1_seepage_prob must be between 0 and 1",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p1_seepage_prob(prob);
        Ok(new_self)
    }

    /// Set the distribution of Pauli errors for single-qubit gates.
    ///
    /// Specifies how single-qubit errors are distributed among
    /// X, Y, and Z Pauli errors. Values should sum to 1.0.
    ///
    /// Args:
    ///     model: Dictionary mapping Pauli operators to probabilities
    ///            e.g., {"X": 0.5, "Y": 0.3, "Z": 0.2}
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Example:
    ///     >>> `builder.with_p1_pauli_model`({
    ///     ...     "X": 0.5,  # 50% X errors (bit flips)
    ///     ...     "Y": 0.3,  # 30% Y errors
    ///     ...     "Z": 0.2   # 20% Z errors (phase flips)
    ///     ... })
    #[pyo3(text_signature = "($self, model)")]
    fn with_p1_pauli_model(&self, model: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut btree_model = BTreeMap::new();
        for (key, value) in model.iter() {
            let key_str: String = key.extract()?;
            let value_f64: f64 = value.extract()?;
            btree_model.insert(key_str, value_f64);
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p1_pauli_model(&btree_model);
        Ok(new_self)
    }

    /// Set the scaling factor for single-qubit gate errors.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_p1_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("p1_scale must be non-negative"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p1_scale(scale);
        Ok(new_self)
    }

    // Two-qubit gate noise setters
    /// Set total error probability after two-qubit gates.
    ///
    /// This is the total probability of any error occurring after
    /// a two-qubit gate operation (e.g., CX, CZ).
    ///
    /// Args:
    ///     p: Total error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_p2_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p2_probability(p)
        })
    }
}

    /// Set average error probability for two-qubit gates.
    ///
    /// This sets the average gate infidelity, which is automatically
    /// converted to total error probability (multiplied by 1.25).
    ///
    /// Args:
    ///     p: Average error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_average_p2_probability(&self, p: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&p) {
            return Err(PyValueError::new_err("p2 must be between 0 and 1"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_average_p2_probability(p);
        Ok(new_self)
    }

    /// Set RZZ angle-dependent error parameters.
    ///
    /// The error rate depends on the rotation angle θ according to:
    /// - For θ < 0: (a × |θ/π|^power + b) × p2
    /// - For θ > 0: (c × |θ/π|^power + d) × p2
    /// - For θ = 0: (b + d) × 0.5 × p2
    ///
    /// Args:
    ///     params: Tuple of (a, b, c, d) parameters
    ///
    /// Returns:
    ///     Self for method chaining
    #[pyo3(text_signature = "($self, params)")]
    fn with_p2_angle_params(&self, params: (f64, f64, f64, f64)) -> Self {
        let mut new_self = self.clone();
        new_self.inner = new_self
            .inner
            .with_p2_angle_params(params.0, params.1, params.2, params.3);
        new_self
    }

    /// Set the power parameter for RZZ angle-dependent errors.
    ///
    /// Args:
    ///     power: Power parameter (must be positive)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If power is not positive
    #[pyo3(text_signature = "($self, power)")]
    fn with_p2_angle_power(&self, power: f64) -> PyResult<Self> {
        if power <= 0.0 {
            return Err(PyValueError::new_err("p2_angle_power must be positive"));
        }
    }

    /// Set the emission ratio for two-qubit gate errors.
    ///
    /// Args:
    ///     ratio: Fraction of errors that are emission errors (0.0 to 1.0)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If ratio is not between 0 and 1
    #[pyo3(text_signature = "($self, ratio)")]
    fn with_p2_emission_ratio(&self, ratio: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&ratio) {
            return Err(PyValueError::new_err(
                "p2_emission_ratio must be between 0 and 1",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p2_emission_ratio(ratio);
        Ok(new_self)
    }

    /// Set the emission error model for two-qubit gates.
    ///
    /// Args:
    ///     model: Dictionary mapping two-qubit Pauli operators to probabilities
    ///
    /// Returns:
    ///     Self for method chaining
    #[pyo3(text_signature = "($self, model)")]
    fn with_p2_emission_model(&self, model: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut btree_model = BTreeMap::new();
        for (key, value) in model.iter() {
            let key_str: String = key.extract()?;
            let value_f64: f64 = value.extract()?;
            btree_model.insert(key_str, value_f64);
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p2_emission_model(&btree_model);
        Ok(new_self)
    }

    /// Set the seepage probability for two-qubit gates.
    ///
    /// Args:
    ///     prob: Probability of seeping leaked qubits (0.0 to 1.0)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If prob is not between 0 and 1
    #[pyo3(text_signature = "($self, prob)")]
    fn with_p2_seepage_prob(&self, prob: f64) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&prob) {
            return Err(PyValueError::new_err(
                "p2_seepage_prob must be between 0 and 1",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p2_seepage_prob(prob);
        Ok(new_self)
    }

    /// Set the distribution of Pauli errors for two-qubit gates.
    ///
    /// Specifies how two-qubit errors are distributed among
    /// two-qubit Pauli operators.
    ///
    /// Args:
    ///     model: Dictionary mapping two-qubit Pauli strings to probabilities
    ///            e.g., {"IX": 0.25, "XI": 0.25, "XX": 0.5}
    ///
    /// Returns:
    ///     Self for method chaining
    #[pyo3(text_signature = "($self, model)")]
    fn with_p2_pauli_model(&self, model: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut btree_model = BTreeMap::new();
        for (key, value) in model.iter() {
            let key_str: String = key.extract()?;
            let value_f64: f64 = value.extract()?;
            btree_model.insert(key_str, value_f64);
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p2_pauli_model(&btree_model);
        Ok(new_self)
    }

    /// Set the idle noise probability after two-qubit gates.
    ///
    /// Args:
    ///     p: Idle noise probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }

    /// Set the scaling factor for two-qubit gate errors.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_p2_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("p2_scale must be non-negative"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p2_scale(scale);
        Ok(new_self)
    }

    // Measurement noise setters
    /// Set probability of measurement bit flip from |0> to |1>.
    ///
    /// This is the probability that a qubit in state |0> is incorrectly
    /// measured as |1>.
    ///
    /// Args:
    ///     p: Error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_meas_0_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_0_probability(p)
        })
    }

    /// Set probability of measurement bit flip from |1> to |0>.
    ///
    /// This is the probability that a qubit in state |1> is incorrectly
    /// measured as |0>.
    ///
    /// Args:
    ///     p: Error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_meas_1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_1_probability(p)
        })
    }

    /// Set symmetric measurement error probability.
    ///
    /// Sets both 0->1 and 1->0 measurement error probabilities to the same value.
    ///
    /// Args:
    ///     p: Error probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_p1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p1_probability(p)
        })
    }

    /// Set probability of crosstalk during measurement operations.
    ///
    /// Args:
    ///     p: Crosstalk probability between 0.0 and 1.0
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If p is not between 0 and 1
    #[pyo3(text_signature = "($self, p)")]
    fn with_p2_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p2_probability(p)
        })
    }

    /// Set the scaling factor for measurement errors.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_meas_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err("meas_scale must be non-negative"));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_meas_scale(scale);
        Ok(new_self)
    }

    /// Set the scaling factor for measurement crosstalk probability.
    ///
    /// Args:
    ///     scale: Scaling factor (must be non-negative)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     `ValueError`: If scale is negative
    #[pyo3(text_signature = "($self, scale)")]
    fn with_p_meas_crosstalk_scale(&self, scale: f64) -> PyResult<Self> {
        if scale < 0.0 {
            return Err(PyValueError::new_err(
                "p_meas_crosstalk_scale must be non-negative",
            ));
        }
        let mut new_self = self.clone();
        new_self.inner = new_self.inner.with_p_meas_crosstalk_scale(scale);
        Ok(new_self)
    }

    /// Internal method to get the underlying Rust builder
    #[pyo3(text_signature = "($self)")]
    fn _get_builder(&self) -> Self {
        self.clone()
    }

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "GeneralNoiseModelBuilder()".to_string()
    }
}

/// Python wrapper for GeneralNoiseModelBuilder  
#[pyclass(name = "GeneralNoiseModelBuilder", module = "pecos_rslib._pecos_rslib")]
#[derive(Clone)]
pub struct PyGeneralNoiseModelBuilder {
    pub(crate) inner: GeneralNoiseModelBuilder,
}

#[pymethods]
impl PyGeneralNoiseModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: GeneralNoiseModelBuilder::new()
        }
    }

    /// Set single-qubit gate error probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_p1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p1_probability(p)
        })
    }

    /// Set two-qubit gate error probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_p2_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_p2_probability(p)
        })
    }

    // Add more methods as needed...
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
        NoiseModelType::PassThrough(Box::new(PassThroughNoiseModel::builder()))
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
    shot_vec_to_columnar_py(py, &shot_vec, BitVecFormat::BigUint)
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

/// Python wrapper for `QasmSimulation`
#[pyclass(name = "QasmSimulation", module = "pecos_rslib._pecos_rslib")]
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

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "QasmSimulation(<compiled>)".to_string()
    }
}

/// Python wrapper for `QasmSimulationBuilder`
#[pyclass(name = "QasmSimulationBuilder", module = "pecos_rslib._pecos_rslib")]
#[derive(Clone)]
pub struct PyQasmSimulationBuilder {
    qasm: String,
    seed: Option<u64>,
    workers: usize,
    noise_model: NoiseModelType,
    quantum_engine: QuantumEngineType,
    bit_format: BitVecFormat,
    #[cfg(feature = "wasm")]
    wasm_path: Option<String>,
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

    /// Set the noise model using a `GeneralNoiseModelBuilder` or other noise types
    pub fn noise(&self, noise_model: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut new = self.clone();

        // Check if it's a GeneralNoiseModelBuilder directly
        if let Ok(builder) = noise_model.downcast::<PyGeneralNoiseModelBuilder>() {
            let py_builder: PyGeneralNoiseModelBuilder = builder.extract()?;
            new.noise_model = NoiseModelType::General(Box::new(py_builder.get_inner_builder()));
            return Ok(new);
        }

        // Otherwise parse as other noise model types
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

    /// Set the path to a WebAssembly file (.wasm or .wat) for foreign function calls
    #[cfg(feature = "wasm")]
    pub fn wasm(&self, wasm_path: String) -> Self {
        let mut new = self.clone();
        new.wasm_path = Some(wasm_path);
        new
    }

    /// Configure the simulation using a dictionary
    pub fn config(&self, py: Python<'_>, config: &Bound<'_, PyDict>) -> PyResult<Self> {
        let mut new = self.clone();

        // Handle seed
        if let Some(seed_val) = config.get_item("seed")?
            && !seed_val.is_none()
        {
            let seed: u64 = seed_val.extract()?;
            new.seed = Some(seed);
        }

        // Handle workers
        if let Some(workers_val) = config.get_item("workers")?
            && !workers_val.is_none()
        {
            // Check if it's the string "auto"
            if let Ok(workers_str) = workers_val.extract::<String>() {
                if workers_str == "auto" {
                    new.workers = std::thread::available_parallelism()
                        .map(std::num::NonZero::get)
                        .unwrap_or(4);
                } else {
                    return Err(PyValueError::new_err(format!(
                        "Invalid workers value: {workers_str}"
                    )));
                }
            } else {
                // Try to extract as integer
                let workers: usize = workers_val.extract()?;
                new.workers = workers;
            }
        }

        // Handle noise
        if let Some(noise_val) = config.get_item("noise")? {
            if noise_val.is_none() {
                // Explicitly null - use PassThrough
                new.noise_model =
                    NoiseModelType::PassThrough(Box::new(PassThroughNoiseModel::builder()));
            } else if let Ok(noise_dict) = noise_val.downcast::<PyDict>() {
                // It's a dictionary with noise configuration
                new.noise_model = parse_noise_config(py, noise_dict)?;
            } else {
                return Err(PyValueError::new_err("noise must be a dictionary or null"));
            }
        }

        // Handle quantum_engine
        if let Some(engine_val) = config.get_item("quantum_engine")?
            && !engine_val.is_none()
        {
            let engine_str: String = engine_val.extract()?;
            match engine_str.as_str() {
                "StateVector" => new.quantum_engine = QuantumEngineType::StateVector,
                "SparseStabilizer" => new.quantum_engine = QuantumEngineType::SparseStabilizer,
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "Unknown quantum engine: {engine_str}"
                    )));
                }
            }
        }

        // Handle binary_string_format
        if let Some(format_val) = config.get_item("binary_string_format")?
            && !format_val.is_none()
        {
            let use_binary: bool = format_val.extract()?;
            if use_binary {
                new.bit_format = BitVecFormat::BinaryString;
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

        #[cfg(feature = "wasm")]
        if let Some(ref wasm_path) = self.wasm_path {
            builder = builder.wasm(wasm_path);
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

        #[cfg(feature = "wasm")]
        if let Some(ref wasm_path) = self.wasm_path {
            builder = builder.wasm(wasm_path);
        }

        let shot_vec = builder.run(shots).map_err(|e| pecos_error_to_pyerr(&e))?;
        shot_vec_to_columnar_py(py, &shot_vec, self.bit_format)
    }

    fn __repr__(&self) -> String {
        let noise_str = match &self.noise_model {
            NoiseModelType::PassThrough(_) => "PassThrough",
            NoiseModelType::Depolarizing(_) => "Depolarizing",
            NoiseModelType::BiasedDepolarizing(_) => "BiasedDepolarizing",
            NoiseModelType::General(_) => "General",
        };
        let engine_str = match self.quantum_engine {
            QuantumEngineType::StateVector => "StateVector",
            QuantumEngineType::SparseStabilizer => "SparseStabilizer",
        };
        format!(
            "QasmSimulationBuilder(noise={}, engine={}, workers={})",
            noise_str, engine_str, self.workers
        )
    }

    /// Get the current number of workers
    #[getter]
    fn get_workers(&self) -> usize {
        self.workers
    }

    /// Get the current random seed if set
    #[getter]
    fn get_seed(&self) -> Option<u64> {
        self.seed
    }

    /// Check if binary string format is enabled
    #[getter]
    fn is_binary_string_format(&self) -> bool {
        self.bit_format == BitVecFormat::BinaryString
    }
}

/// Create a QASM simulation builder
#[pyfunction(name = "qasm_sim")]
pub fn py_qasm_sim(qasm: &str) -> PyResult<PyQasmSimBuilder> {
    let sim_builder = qasm_engine()
        .program(QasmProgram::from_string(qasm))
        .to_sim();
    
    Ok(PyQasmSimBuilder {
        inner: Some(sim_builder)
    })
}

/// Register the new unified QASM simulation module
pub fn register_qasm_sim_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyQasmSimBuilder>()?;
    module.add_class::<PyDepolarizingNoiseModelBuilder>()?;
    module.add_class::<PyBiasedDepolarizingNoiseModelBuilder>()?;
    module.add_class::<PyGeneralNoiseModelBuilder>()?;
    module.add_function(wrap_pyfunction!(py_qasm_sim, module)?)?;
    Ok(())
}