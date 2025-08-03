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

/// Python wrapper for the unified SimBuilder<QasmEngineBuilder>
/// 
/// This directly mirrors the Rust SimBuilder API
#[pyclass(name = "QasmSimBuilder", module = "pecos_rslib._pecos_rslib")]
pub struct PyQasmSimBuilder {
    inner: Option<pecos_engines::SimBuilder<QasmEngineBuilder>>,
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

    /// Set the number of worker threads
    #[pyo3(text_signature = "($self, workers)")]
    fn workers(mut self_: PyRefMut<'_, Self>, workers: usize) -> PyResult<()> {
        if let Some(builder) = self_.inner.take() {
            self_.inner = Some(builder.workers(workers));
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Use automatic worker count based on available CPUs
    #[pyo3(text_signature = "($self)")]
    fn auto_workers(mut self_: PyRefMut<'_, Self>) -> PyResult<()> {
        if let Some(builder) = self_.inner.take() {
            self_.inner = Some(builder.auto_workers());
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Set the quantum engine type
    #[pyo3(text_signature = "($self, engine_type)")]
    fn quantum(mut self_: PyRefMut<'_, Self>, engine_type: &str) -> PyResult<()> {
        use pecos_engines::quantum_engine_builder::{state_vector, sparse_stab};
        
        eprintln!("DEBUG PyQasmSimBuilder::quantum called with engine_type={}", engine_type);
        if let Some(builder) = self_.inner.take() {
            match engine_type.to_lowercase().as_str() {
                "statevector" | "state_vector" => {
                    eprintln!("DEBUG PyQasmSimBuilder: Setting state_vector engine");
                    self_.inner = Some(builder.quantum(state_vector()));
                }
                "sparsestabilizer" | "sparse_stabilizer" => {
                    eprintln!("DEBUG PyQasmSimBuilder: Setting sparse_stabilizer engine");
                    self_.inner = Some(builder.quantum(sparse_stab()));
                }
                _ => {
                    self_.inner = Some(builder); // Put it back
                    return Err(PyRuntimeError::new_err(format!("Unknown quantum engine type: {}", engine_type)));
                }
            }
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Set noise from a DepolarizingNoiseModelBuilder
    #[pyo3(text_signature = "($self, noise_builder)")]
    fn noise_depolarizing(mut self_: PyRefMut<'_, Self>, noise_builder: &PyDepolarizingNoiseModelBuilder) -> PyResult<()> {
        if let Some(builder) = self_.inner.take() {
            self_.inner = Some(builder.noise(noise_builder.inner.clone()));
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Set noise from a BiasedDepolarizingNoiseModelBuilder  
    #[pyo3(text_signature = "($self, noise_builder)")]
    fn noise_biased_depolarizing(mut self_: PyRefMut<'_, Self>, noise_builder: &PyBiasedDepolarizingNoiseModelBuilder) -> PyResult<()> {
        if let Some(builder) = self_.inner.take() {
            self_.inner = Some(builder.noise(noise_builder.inner.clone()));
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Set noise from a GeneralNoiseModelBuilder
    #[pyo3(text_signature = "($self, noise_builder)")]
    fn noise_general(mut self_: PyRefMut<'_, Self>, noise_builder: &PyGeneralNoiseModelBuilder) -> PyResult<()> {
        if let Some(builder) = self_.inner.take() {
            self_.inner = Some(builder.noise(noise_builder.inner.clone()));
            Ok(())
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }

    /// Run the simulation with the specified number of shots
    #[pyo3(text_signature = "($self, shots)")]
    fn run(mut self_: PyRefMut<'_, Self>, shots: usize) -> PyResult<HashMap<String, Vec<String>>> {
        if let Some(builder) = self_.inner.take() {
            let results = builder.run(shots).map_err(pecos_error_to_pyerr)?;
            Ok(convert_shot_vec_to_binary_dict(results))
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }
}

/// Python wrapper for DepolarizingNoiseModelBuilder
#[pyclass(name = "DepolarizingNoiseModelBuilder", module = "pecos_rslib._pecos_rslib")]
#[derive(Clone)]
pub struct PyDepolarizingNoiseModelBuilder {
    pub(crate) inner: DepolarizingNoiseModelBuilder,
}

#[pymethods]
impl PyDepolarizingNoiseModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: DepolarizingNoiseModelBuilder::new()
        }
    }

    /// Set preparation error probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }

    /// Set measurement error probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_meas_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_probability(p)
        })
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
}

/// Python wrapper for BiasedDepolarizingNoiseModelBuilder
#[pyclass(name = "BiasedDepolarizingNoiseModelBuilder", module = "pecos_rslib._pecos_rslib")]
#[derive(Clone)]
pub struct PyBiasedDepolarizingNoiseModelBuilder {
    pub(crate) inner: BiasedDepolarizingNoiseModelBuilder,
}

#[pymethods]
impl PyBiasedDepolarizingNoiseModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: BiasedDepolarizingNoiseModelBuilder::new()
        }
    }

    /// Set preparation error probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_prep_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_prep_probability(p)
        })
    }

    /// Set measurement 0->1 flip probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_meas_0_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_0_probability(p)
        })
    }

    /// Set measurement 1->0 flip probability
    #[pyo3(text_signature = "($self, p)")]
    fn with_meas_1_probability(&self, p: f64) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().with_meas_1_probability(p)
        })
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

/// Create a new QASM simulation builder (thin wrapper around Rust qasm_engine().program().to_sim())
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