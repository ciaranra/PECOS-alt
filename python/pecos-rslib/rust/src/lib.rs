#![doc(html_root_url = "https://docs.rs/pecos-rslib")]
// Disable doctests since they don't work with our workspace setup
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(test(no_crate_inject))]
#![doc(test(attr(deny(warnings))))]

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

mod byte_message_bindings;
mod engine_bindings;
mod engine_builders;
mod noise_helpers;
// mod pcg_bindings;
mod pecos_rng_bindings;
pub mod phir_json_bridge;
// mod qasm_sim_bindings;
mod shot_results_bindings;
mod sparse_sim;
mod sparse_stab_bindings;
mod sparse_stab_engine_bindings;
mod state_vec_bindings;
mod state_vec_engine_bindings;
mod sim;
mod plugin_compiler_bindings;

use byte_message_bindings::{PyByteMessage, PyByteMessageBuilder};
use pecos_rng_bindings::RngPcg;
use pyo3::prelude::*;
use sparse_stab_bindings::SparseSim;
use sparse_stab_engine_bindings::PySparseStabEngine;
use state_vec_bindings::RsStateVec;
use state_vec_engine_bindings::PyStateVecEngine;
use engine_builders::{PyQasmProgram, PyLlvmProgram, PyHugrProgram, PyPhirJsonProgram};

/// A Python module implemented in Rust.
#[pymodule]
fn _pecos_rslib(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SparseSim>()?;
    m.add_class::<phir_json_bridge::PhirJsonEngine>()?;
    m.add_class::<RsStateVec>()?;
    m.add_class::<PyByteMessage>()?;
    m.add_class::<PyByteMessageBuilder>()?;
    m.add_class::<shot_results_bindings::PyShotVec>()?;
    m.add_class::<shot_results_bindings::PyShotMap>()?;
    m.add_class::<PyStateVecEngine>()?;
    m.add_class::<PySparseStabEngine>()?;
    m.add_class::<RngPcg>()?;

    // Register QASM simulation functions
    // qasm_sim_bindings::register_qasm_sim_module(m)?;
    
    // Register the unified sim() function
    sim::register_sim(m)?;

    // Register program types
    m.add_class::<PyQasmProgram>()?;
    m.add_class::<PyLlvmProgram>()?;
    m.add_class::<PyHugrProgram>()?;
    m.add_class::<PyPhirJsonProgram>()?;

    // Register engine builder functions
    m.add_function(wrap_pyfunction!(engine_builders::qasm_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::llvm_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::selene_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::phir_json_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::general_noise, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::depolarizing_noise, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::biased_depolarizing_noise, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::state_vector, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::sparse_stabilizer, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::sparse_stab, m)?)?;

    Ok(())
}
