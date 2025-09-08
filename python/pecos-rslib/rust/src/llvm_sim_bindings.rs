//! Thin PyO3 bindings that directly mirror the unified Rust LLVM simulation API

use pecos_engines::shot_results::ShotVec;
use pecos_llvm_sim::{llvm_engine, LlvmEngineBuilder};
use pecos_engines::{ClassicalControlEngineBuilder};
use pecos_engines::noise::{
    DepolarizingNoiseModelBuilder, BiasedDepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder, PassThroughNoiseModelBuilder, IntoNoiseModel
};
use pecos_engines::quantum_engine_builder::{state_vector, sparse_stab};
use pecos_programs::LlvmProgram;
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::collections::HashMap;

/// Convert `PecosError` to `PyErr`
fn pecos_error_to_pyerr(err: pecos_core::errors::PecosError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// Convert ShotVec to HashMap with binary string values
fn convert_shot_vec_to_binary_dict(shot_vec: ShotVec) -> HashMap<String, Vec<String>> {
    let shot_map = shot_vec.try_as_shot_map().unwrap_or_else(|_| {
        // Fallback for empty results
        HashMap::new().into()
    });

    shot_map.registers().iter().map(|(name, bit_vectors)| {
        let binary_strings: Vec<String> = bit_vectors.iter()
            .map(|bv| bv.iter().map(|bit| if *bit { '1' } else { '0' }).collect())
            .collect();
        (name.clone(), binary_strings)
    }).collect()
}

/// Python wrapper for the unified SimBuilder<LlvmEngineBuilder>
///
/// This directly mirrors the Rust SimBuilder API
#[pyclass(name = "LlvmSimBuilder", module = "pecos_rslib._pecos_rslib")]
pub struct PyLlvmSimBuilder {
    inner: Option<pecos_engines::SimBuilder<LlvmEngineBuilder>>,
}

#[pymethods]
impl PyLlvmSimBuilder {
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
        if let Some(builder) = self_.inner.take() {
            match engine_type.to_lowercase().as_str() {
                "statevector" | "state_vector" => {
                    self_.inner = Some(builder.quantum(state_vector()));
                }
                "sparsestabilizer" | "sparse_stabilizer" => {
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
    fn noise_depolarizing(mut self_: PyRefMut<'_, Self>, noise_builder: &crate::qasm_sim_bindings::PyDepolarizingNoiseModelBuilder) -> PyResult<()> {
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

/// Create a new LLVM simulation builder (thin wrapper around Rust llvm_engine().program().to_sim())
#[pyfunction(name = "llvm_sim")]
pub fn py_llvm_sim(llvm_ir: &str) -> PyResult<PyLlvmSimBuilder> {
    let sim_builder = llvm_engine()
        .program(LlvmProgram::from_string(llvm_ir))
        .to_sim();

    Ok(PyLlvmSimBuilder {
        inner: Some(sim_builder)
    })
}

/// Register the new unified LLVM simulation module
pub fn register_llvm_sim_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyLlvmSimBuilder>()?;
    module.add_function(wrap_pyfunction!(py_llvm_sim, module)?)?;
    Ok(())
}