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

//! Python bindings for the LlvmSim builder interface

use pecos_llvm_sim::{LlvmSim, LlvmSimulation, NoiseModelConfig, QuantumEngineType, DepolarizingNoise, DepolarizingCustomNoise, BiasedDepolarizingNoise};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Python class for noise models (using a class instead of enum for PyO3 compatibility)
#[pyclass(name = "LlvmNoiseModel")]
#[derive(Clone)]
pub struct PyLlvmNoiseModel {
    variant: NoiseModelVariant,
}

#[derive(Clone)]
enum NoiseModelVariant {
    PassThrough,
    Depolarizing { p: f64 },
    DepolarizingCustom { p_prep: f64, p_meas: f64, p1: f64, p2: f64 },
    BiasedDepolarizing { p: f64 },
}

#[pymethods]
impl PyLlvmNoiseModel {
    #[staticmethod]
    #[pyo3(name = "PassThrough")]
    fn pass_through() -> Self {
        Self { variant: NoiseModelVariant::PassThrough }
    }
    
    #[staticmethod]
    #[pyo3(name = "Depolarizing")]
    fn depolarizing(p: f64) -> Self {
        Self { variant: NoiseModelVariant::Depolarizing { p } }
    }
    
    #[staticmethod]
    #[pyo3(name = "DepolarizingCustom")]
    fn depolarizing_custom(p_prep: f64, p_meas: f64, p1: f64, p2: f64) -> Self {
        Self { variant: NoiseModelVariant::DepolarizingCustom { p_prep, p_meas, p1, p2 } }
    }
    
    #[staticmethod]
    #[pyo3(name = "BiasedDepolarizing")]
    fn biased_depolarizing(p: f64) -> Self {
        Self { variant: NoiseModelVariant::BiasedDepolarizing { p } }
    }
}

/// Python class for quantum engines (using a class instead of enum for PyO3 compatibility)
#[pyclass(name = "LlvmQuantumEngine")]
#[derive(Clone)]
pub struct PyLlvmQuantumEngine {
    variant: QuantumEngineVariant,
}

#[derive(Clone)]
enum QuantumEngineVariant {
    StateVector,
    SparseStabilizer,
}

#[pymethods]
impl PyLlvmQuantumEngine {
    #[staticmethod]
    #[pyo3(name = "StateVector")]
    fn state_vector() -> Self {
        Self { variant: QuantumEngineVariant::StateVector }
    }
    
    #[staticmethod]
    #[pyo3(name = "SparseStabilizer")]
    fn sparse_stabilizer() -> Self {
        Self { variant: QuantumEngineVariant::SparseStabilizer }
    }
}

/// Python wrapper for LlvmSim builder
#[pyclass(name = "llvm_sim_builder")]
pub struct PyLlvmSimBuilder {
    builder: LlvmSim,
}

#[pymethods]
impl PyLlvmSimBuilder {
    /// Create a new LlvmSim builder from source (string or file path)
    #[new]
    pub fn new(source: &str) -> PyResult<Self> {
        // Check if it's a file path
        let builder = if std::path::Path::new(source).exists() {
            LlvmSim::new().llvm_file(source)
        } else {
            // Assume it's LLVM IR string
            LlvmSim::new().llvm_ir(source)
        };
        
        Ok(Self { builder })
    }

    /// Set random seed
    pub fn seed(mut slf: PyRefMut<'_, Self>, seed: u64) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().seed(seed);
        slf
    }

    /// Set number of worker threads
    pub fn workers(mut slf: PyRefMut<'_, Self>, workers: usize) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().workers(workers);
        slf
    }

    /// Enable depolarizing noise
    pub fn with_depolarizing_noise(mut slf: PyRefMut<'_, Self>, p: f64) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().noise(DepolarizingNoise { p });
        slf
    }

    /// Enable custom depolarizing noise
    pub fn with_custom_depolarizing_noise(
        mut slf: PyRefMut<'_, Self>,
        p_prep: f64,
        p_meas: f64,
        p1: f64,
        p2: f64,
    ) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().noise(DepolarizingCustomNoise { p_prep, p_meas, p1, p2 });
        slf
    }

    /// Enable biased depolarizing noise
    pub fn with_biased_depolarizing_noise(mut slf: PyRefMut<'_, Self>, p: f64) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().noise(BiasedDepolarizingNoise { p });
        slf
    }

    /// Use state vector engine
    pub fn with_state_vector_engine(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().quantum_engine(QuantumEngineType::StateVector);
        slf
    }

    /// Use sparse stabilizer engine
    pub fn with_sparse_stabilizer_engine(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().quantum_engine(QuantumEngineType::SparseStabilizer);
        slf
    }

    /// Set noise model from enum
    pub fn noise(mut slf: PyRefMut<'_, Self>, noise_model: PyLlvmNoiseModel) -> PyRefMut<'_, Self> {
        let config = match noise_model.variant {
            NoiseModelVariant::PassThrough => NoiseModelConfig::PassThrough,
            NoiseModelVariant::Depolarizing { p } => NoiseModelConfig::Depolarizing(p),
            NoiseModelVariant::DepolarizingCustom { p_prep, p_meas, p1, p2 } => {
                NoiseModelConfig::DepolarizingCustom { p_prep, p_meas, p1, p2 }
            }
            NoiseModelVariant::BiasedDepolarizing { p } => NoiseModelConfig::BiasedDepolarizing(p),
        };
        slf.builder = slf.builder.clone().noise(config);
        slf
    }

    /// Set quantum engine from enum
    pub fn quantum_engine(mut slf: PyRefMut<'_, Self>, engine: PyLlvmQuantumEngine) -> PyRefMut<'_, Self> {
        let engine_type = match engine.variant {
            QuantumEngineVariant::StateVector => QuantumEngineType::StateVector,
            QuantumEngineVariant::SparseStabilizer => QuantumEngineType::SparseStabilizer,
        };
        slf.builder = slf.builder.clone().quantum_engine(engine_type);
        slf
    }

    /// Enable verbose output
    pub fn verbose(mut slf: PyRefMut<'_, Self>, verbose: bool) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().verbose(verbose);
        slf
    }

    /// Enable debug output
    pub fn debug(mut slf: PyRefMut<'_, Self>, debug: bool) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().debug(debug);
        slf
    }

    /// Set maximum number of qubits allowed for allocation
    pub fn max_qubits(mut slf: PyRefMut<'_, Self>, max_qubits: usize) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().max_qubits(max_qubits);
        slf
    }

    /// Keep temporary files
    pub fn keep_temp_files(mut slf: PyRefMut<'_, Self>, keep: bool) -> PyRefMut<'_, Self> {
        slf.builder = slf.builder.clone().keep_temp_files(keep);
        slf
    }

    /// Build the simulation
    pub fn build(&self) -> PyResult<PyLlvmSimulation> {
        let simulation = self.builder.clone().build()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to build simulation: {e}")))?;
        Ok(PyLlvmSimulation { simulation })
    }

    /// Run the simulation
    pub fn run(&self, py: Python<'_>, shots: usize) -> PyResult<PyObject> {
        let shot_vec = self.builder.clone().run(shots)
            .map_err(|e| PyRuntimeError::new_err(format!("Simulation failed: {e}")))?;
        
        // Convert ShotVec to ShotMap then to Python dict
        let shot_map = shot_vec.try_as_shot_map()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert to ShotMap: {e}")))?;
        
        let py_dict = PyDict::new(py);
        for register in shot_map.register_names() {
            // Try to get as BitVec first (most common for quantum registers)
            if let Ok(values) = shot_map.try_bits_as_u64(&register) {
                let i64_values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
                let py_list = PyList::new(py, i64_values)?;
                py_dict.set_item(register, py_list)?;
            }
            // Try as i64 directly
            else if let Ok(values) = shot_map.try_i64s(&register) {
                let py_list = PyList::new(py, values)?;
                py_dict.set_item(register, py_list)?;
            }
            // Try as u32 and convert
            else if let Ok(values) = shot_map.try_u32s(&register) {
                let i64_values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
                let py_list = PyList::new(py, i64_values)?;
                py_dict.set_item(register, py_list)?;
            }
            // Default to zeros if we can't convert
            else {
                let zeros = vec![0i64; shot_map.num_shots()];
                let py_list = PyList::new(py, zeros)?;
                py_dict.set_item(register, py_list)?;
            }
        }
        Ok(py_dict.into())
    }
}

/// Python wrapper for LlvmSimulation
#[pyclass(name = "LlvmSimulation")]
pub struct PyLlvmSimulation {
    simulation: LlvmSimulation,
}

#[pymethods]
impl PyLlvmSimulation {
    /// Run the simulation
    pub fn run(&mut self, py: Python<'_>, shots: usize) -> PyResult<PyObject> {
        let shot_vec = self.simulation.run(shots)
            .map_err(|e| PyRuntimeError::new_err(format!("Simulation failed: {e}")))?;
        
        // Convert ShotVec to ShotMap then to Python dict
        let shot_map = shot_vec.try_as_shot_map()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to convert to ShotMap: {e}")))?;
        
        let py_dict = PyDict::new(py);
        for register in shot_map.register_names() {
            // Try to get as BitVec first (most common for quantum registers)
            if let Ok(values) = shot_map.try_bits_as_u64(&register) {
                let i64_values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
                let py_list = PyList::new(py, i64_values)?;
                py_dict.set_item(register, py_list)?;
            }
            // Try as i64 directly
            else if let Ok(values) = shot_map.try_i64s(&register) {
                let py_list = PyList::new(py, values)?;
                py_dict.set_item(register, py_list)?;
            }
            // Try as u32 and convert
            else if let Ok(values) = shot_map.try_u32s(&register) {
                let i64_values: Vec<i64> = values.into_iter().map(|v| v as i64).collect();
                let py_list = PyList::new(py, i64_values)?;
                py_dict.set_item(register, py_list)?;
            }
            // Default to zeros if we can't convert
            else {
                let zeros = vec![0i64; shot_map.num_shots()];
                let py_list = PyList::new(py, zeros)?;
                py_dict.set_item(register, py_list)?;
            }
        }
        Ok(py_dict.into())
    }

    /// Get simulation statistics
    pub fn stats(&self) -> (usize, usize) {
        self.simulation.stats()
    }
}

/// Register the llvm_sim module
pub fn register_llvm_sim_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLlvmNoiseModel>()?;
    m.add_class::<PyLlvmQuantumEngine>()?;
    m.add_class::<PyLlvmSimBuilder>()?;
    m.add_class::<PyLlvmSimulation>()?;
    Ok(())
}