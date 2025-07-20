// Copyright 2024 The PECOS Developers
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

use pyo3::prelude::*;
use log::LevelFilter;

mod byte_message;
mod engines;
mod engine_builders;
mod error;
mod phir;
mod qasm;
mod llvm; // LLVM simulation with full feature parity
mod sparse_sim;
mod state_vec;

use byte_message::{PyByteMessage, PyByteMessageBuilder};
use engines::{PySparseStabEngineRs, PyStateVecEngineRs};
use qasm::{
    get_noise_models, get_quantum_engines, qasm_sim_builder, run_qasm, NoiseModel, QuantumEngine,
};
use llvm::{llvm_sim_builder, LlvmNoiseModel, LlvmQuantumEngine};
use sparse_sim::PySparseSimRs;
use state_vec::PyStateVecRs;

/// Python bindings for PECOS Rust implementations
#[pymodule]
fn _pecos_rslib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Initialize logger with default level of WARN to suppress debug messages
    // Users can override this by setting RUST_LOG environment variable
    if std::env::var("RUST_LOG").is_err() {
        // Only set up logging if RUST_LOG is not already set
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Warn)
            .try_init();
    }
    
    // Original engine classes
    m.add_class::<PyStateVecRs>()?;
    m.add_class::<PySparseSimRs>()?;
    
    // Byte message classes
    m.add_class::<PyByteMessage>()?;
    m.add_class::<PyByteMessageBuilder>()?;
    
    // Engine classes
    m.add_class::<PyStateVecEngineRs>()?;
    m.add_class::<PySparseStabEngineRs>()?;
    
    // QASM simulation enums and functions
    m.add_class::<NoiseModel>()?;
    m.add_class::<QuantumEngine>()?;
    m.add_function(wrap_pyfunction!(run_qasm, m)?)?;
    m.add_function(wrap_pyfunction!(get_noise_models, m)?)?;
    m.add_function(wrap_pyfunction!(get_quantum_engines, m)?)?;
    m.add_function(wrap_pyfunction!(qasm_sim_builder, m)?)?;
    
    // LLVM simulation
    m.add_class::<LlvmNoiseModel>()?;
    m.add_class::<LlvmQuantumEngine>()?;
    m.add_function(wrap_pyfunction!(llvm_sim_builder, m)?)?;
    
    // Add PHIR compilation submodule
    let phir_module = PyModule::new(m.py(), "phir")?;
    phir::register_phir_module(&phir_module)?;
    m.add_submodule(&phir_module)?;

    // Add engine builders for unified API
    engine_builders::register_engine_builders(&m)?;
    
    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    Ok(())
}