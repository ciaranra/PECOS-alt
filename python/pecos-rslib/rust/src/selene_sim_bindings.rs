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

//! Python bindings for the Selene simulation interface

use pecos_selene_ceng::{
    selene_sim, SeleneSimBuilder, SeleneSimulation, 
    selene_engine, SeleneEngineBuilder, SeleneEngine,
    NoiseModelConfig, QuantumEngineType
};
use pecos_engines::ClassicalControlEngineBuilder;
use crate::shot_results_bindings::PyShotVec;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Python class for Selene noise models
#[pyclass(name = "SeleneNoiseModel")]
#[derive(Clone)]
pub struct PySeleneNoiseModel {
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
impl PySeleneNoiseModel {
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

/// Python class for Selene quantum engines
#[pyclass(name = "SeleneQuantumEngine")]
#[derive(Clone)]
pub struct PySeleneQuantumEngine {
    variant: QuantumEngineVariant,
}

#[derive(Clone)]
enum QuantumEngineVariant {
    StateVector,
    SparseStabilizer,
}

#[pymethods]
impl PySeleneQuantumEngine {
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

/// Python wrapper for Selene simulation builder
#[pyclass(name = "selene_sim_builder")]
pub struct PySeleneSimBuilder {
    builder: Option<SeleneSimBuilder>,
}

#[pymethods]
impl PySeleneSimBuilder {
    /// Create a new Selene simulation builder from source (string or file path)
    #[new]
    pub fn new(source: &str) -> PyResult<Self> {
        let builder = selene_sim().llvm_ir(source);
        Ok(Self { builder: Some(builder) })
    }

    /// Set the number of qubits
    pub fn qubits(&mut self, n: usize) -> PyResult<()> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.qubits(n));
        }
        Ok(())
    }

    /// Set the noise model
    pub fn noise(&mut self, noise_model: PySeleneNoiseModel) -> PyResult<()> {
        if let Some(builder) = self.builder.take() {
            let config = match noise_model.variant {
                NoiseModelVariant::PassThrough => NoiseModelConfig::PassThrough,
                NoiseModelVariant::Depolarizing { p } => NoiseModelConfig::Depolarizing(p),
                NoiseModelVariant::DepolarizingCustom { p_prep, p_meas, p1, p2 } => {
                    NoiseModelConfig::DepolarizingCustom { p_prep, p_meas, p1, p2 }
                }
                NoiseModelVariant::BiasedDepolarizing { p } => NoiseModelConfig::BiasedDepolarizing(p),
            };
            self.builder = Some(builder.noise(config));
        }
        Ok(())
    }

    /// Set the quantum engine type
    pub fn quantum_engine(&mut self, engine: PySeleneQuantumEngine) -> PyResult<()> {
        if let Some(builder) = self.builder.take() {
            let engine_type = match engine.variant {
                QuantumEngineVariant::StateVector => QuantumEngineType::StateVector,
                QuantumEngineVariant::SparseStabilizer => QuantumEngineType::SparseStabilizer,
            };
            self.builder = Some(builder.quantum_engine(engine_type));
        }
        Ok(())
    }

    /// Set the random seed
    pub fn seed(&mut self, seed: u64) -> PyResult<()> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.seed(seed));
        }
        Ok(())
    }

    /// Enable optimization
    pub fn optimize(&mut self) -> PyResult<()> {
        if let Some(builder) = self.builder.take() {
            self.builder = Some(builder.optimize());
        }
        Ok(())
    }

    /// Build the simulation
    pub fn build(&mut self) -> PyResult<PySeleneSimulation> {
        if let Some(builder) = self.builder.take() {
            let sim = builder.build_simulation()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to build Selene simulation: {}", e)))?;
            Ok(PySeleneSimulation { simulation: sim })
        } else {
            Err(PyRuntimeError::new_err("Builder already consumed"))
        }
    }
}

/// Python wrapper for Selene simulation
#[pyclass(name = "SeleneSimulation")]
pub struct PySeleneSimulation {
    simulation: SeleneSimulation,
}



#[pymethods]
impl PySeleneSimulation {
    /// Run the simulation for a given number of shots
    pub fn run(&mut self, shots: usize) -> PyResult<PyShotVec> {
        let results = self.simulation.run(shots)
            .map_err(|e| PyRuntimeError::new_err(format!("Selene simulation failed: {}", e)))?;
        
        // Return ShotVec directly wrapped in PyShotVec
        Ok(PyShotVec::from(results))
    }
}



/// Register the Selene simulation module
pub fn register_selene_sim_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PySeleneNoiseModel>()?;
    parent_module.add_class::<PySeleneQuantumEngine>()?;
    parent_module.add_class::<PySeleneSimBuilder>()?;
    parent_module.add_class::<PySeleneSimulation>()?;
    
    // Add the builder functions
    parent_module.add_function(wrap_pyfunction!(selene_sim_builder, parent_module)?)?;
    
    // Add HUGR support functions if feature is enabled
    #[cfg(feature = "hugr")]
    {
        parent_module.add_function(wrap_pyfunction!(selene_sim_builder_hugr, parent_module)?)?;
    }
    
    Ok(())
}

/// Python function to create a Selene simulation builder
#[pyfunction]
fn selene_sim_builder(source: &str) -> PyResult<PySeleneSimBuilder> {
    let builder = selene_sim().llvm_ir(source);
    Ok(PySeleneSimBuilder {
        builder: Some(builder),
    })
}


/// Python function to create a Selene simulation builder from HUGR bytes
#[pyfunction]
#[cfg(feature = "hugr")]
fn selene_sim_builder_hugr(hugr_bytes: &[u8]) -> PyResult<PySeleneSimBuilder> {
    use pecos_selene_ceng::hugr_compiler::get_extension_registry;
    use std::io::Cursor;
    
    // Deserialize HUGR from bytes using the proper extension registry
    let reader = Cursor::new(hugr_bytes);
    let hugr = match hugr::Hugr::load(reader, Some(get_extension_registry())) {
        Ok(h) => h,
        Err(e) => return Err(PyRuntimeError::new_err(format!("Failed to deserialize HUGR: {}", e))),
    };
    
    let builder = selene_sim().hugr(hugr);
    Ok(PySeleneSimBuilder {
        builder: Some(builder),
    })
}

