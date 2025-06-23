// Copyright 2025 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! Context-based QIR execution with proper isolation for parallel execution

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pecos_qir::{setup_qir_engine, engine::QirEngine};
use pecos_engines::{run_sim, NoiseModel, ClassicalEngine};
use pecos_engines::noise::DepolarizingNoiseModel;
use pecos_engines::shot_results;
use pecos_core::rng::RngManageable;
use std::path::PathBuf;
use std::sync::Arc;

/// Execute QIR with proper context isolation
/// 
/// This implementation ensures each QIR execution has its own isolated context,
/// allowing true parallel execution without global state conflicts.
pub fn execute_qir_isolated(
    qir_path: &std::path::Path,
    shots: usize,
    seed: Option<u64>,
    noise_probability: Option<f64>,
    workers: Option<usize>,
) -> Result<shot_results::ShotVec, pecos_core::errors::PecosError> {
    // Create a new QIR engine with its own isolated state
    let mut qir_engine = QirEngine::new(qir_path.to_path_buf());
    
    // Pre-compile if needed
    qir_engine.pre_compile()?;
    
    // Each engine maintains its own LLVM context and runtime state
    let classical_engine: Box<dyn ClassicalEngine> = Box::new(qir_engine);
    
    // Create noise model
    let noise_model: Box<dyn NoiseModel> = if let Some(prob) = noise_probability {
        let mut model = DepolarizingNoiseModel::new_uniform(prob);
        if let Some(s) = seed {
            model.set_seed(s)?;
        }
        Box::new(model)
    } else {
        Box::new(pecos_engines::noise::PassThroughNoiseModel)
    };
    
    // Execute simulation - each execution is fully isolated
    run_sim(
        classical_engine,
        shots,
        noise_model,
        workers,
        seed,
        1,
        false,
    )
}

/// Python binding for isolated QIR execution
#[pyfunction(name = "execute_qir_isolated")]
#[pyo3(signature = (qir_path, shots, seed=None, noise_probability=None, workers=None))]
pub fn py_execute_qir_isolated(
    py: Python<'_>,
    qir_path: &str,
    shots: usize,
    seed: Option<u64>,
    noise_probability: Option<f64>,
    workers: Option<usize>,
) -> PyResult<PyObject> {
    // Validate QIR file path
    let path = std::path::PathBuf::from(qir_path);
    if !path.exists() {
        return Err(PyRuntimeError::new_err(format!(
            "QIR file not found: {}",
            qir_path
        )));
    }
    
    // Execute with isolated context
    let results = execute_qir_isolated(&path, shots, seed, noise_probability, workers)
        .map_err(|e| PyRuntimeError::new_err(format!("QIR execution failed: {:?}", e)))?;
    
    // Convert results to Python format
    use pyo3::types::{PyDict, PyList};
    let py_results = PyList::empty_bound(py);
    
    for result in results.iter() {
        let shot_dict = PyDict::new_bound(py);
        
        // Handle different result types
        match result {
            shot_results::ShotResult::ClassicalResult(shot) => {
                for (key, value) in &shot.data {
                    match value {
                        shot_results::Data::I64(v) => {
                            shot_dict.set_item(key, v)?;
                        }
                        shot_results::Data::Bool(v) => {
                            shot_dict.set_item(key, *v)?;
                        }
                    }
                }
            }
            shot_results::ShotResult::Counts(counts) => {
                shot_dict.set_item("counts", counts)?;
            }
        }
        
        py_results.append(shot_dict)?;
    }
    
    // Create metadata
    let result_dict = PyDict::new_bound(py);
    result_dict.set_item("results", py_results)?;
    result_dict.set_item("shots", shots)?;
    result_dict.set_item("execution_type", "isolated")?;
    
    Ok(result_dict.into())
}

/// Register isolated QIR execution module
pub fn register_isolated_qir_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_execute_qir_isolated, m)?)?;
    Ok(())
}