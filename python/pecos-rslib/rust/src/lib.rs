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
mod coin_toss_bindings;
mod cpp_sparse_sim_bindings;
mod engine_bindings;
mod engine_builders;
mod noise_helpers;
mod pauli_prop_bindings;
// mod pcg_bindings;
mod hugr_compilation_bindings;
mod pecos_rng_bindings;
mod phir_json_bridge;
mod quest_bindings;
mod qulacs_bindings;
mod shot_results_bindings;
mod sim;
mod sparse_sim;
mod sparse_stab_bindings;
mod sparse_stab_engine_bindings;
mod state_vec_bindings;
mod state_vec_engine_bindings;

// Disabled - conflicts with pecos-qis-interface due to duplicate symbols
// #[cfg(feature = "hugr-llvm-pipeline")]
// mod hugr_bindings;

use byte_message_bindings::{PyByteMessage, PyByteMessageBuilder};
use coin_toss_bindings::RsCoinToss;
use cpp_sparse_sim_bindings::CppSparseSim;
use engine_builders::{PyHugrProgram, PyPhirJsonProgram, PyQasmProgram, PyQisProgram};
use pauli_prop_bindings::PyPauliProp;
use pecos_rng_bindings::RngPcg;
use pyo3::prelude::*;
use quest_bindings::{QuestDensityMatrix, QuestStateVec};
use qulacs_bindings::RsQulacs;
use sparse_stab_bindings::SparseSim;
use sparse_stab_engine_bindings::PySparseStabEngine;
use state_vec_bindings::RsStateVec;
use state_vec_engine_bindings::PyStateVecEngine;

/// Clear the global JIT compilation cache (useful for testing)
#[pyfunction]
fn clear_jit_cache() {
    #[cfg(feature = "jit")]
    {
        pecos_qis_jit::JitExecutor::clear_global_cache();
    }
    #[cfg(not(feature = "jit"))]
    {
        log::warn!("JIT cache clear requested but JIT feature not enabled");
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _pecos_rslib(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    log::debug!("_pecos_rslib module initializing (version 2)...");
    m.add_class::<SparseSim>()?;
    m.add_class::<phir_json_bridge::PhirJsonEngine>()?;
    m.add_class::<CppSparseSim>()?;
    m.add_class::<RsStateVec>()?;
    m.add_class::<RsQulacs>()?;
    m.add_class::<RsCoinToss>()?;
    m.add_class::<PyPauliProp>()?;
    m.add_class::<PyByteMessage>()?;
    m.add_class::<PyByteMessageBuilder>()?;
    m.add_class::<shot_results_bindings::PyShotVec>()?;
    m.add_class::<shot_results_bindings::PyShotMap>()?;
    m.add_class::<PyStateVecEngine>()?;
    m.add_class::<PySparseStabEngine>()?;
    m.add_class::<RngPcg>()?;
    m.add_class::<QuestStateVec>()?;
    m.add_class::<QuestDensityMatrix>()?;

    // Register the unified sim() function
    sim::register_sim_module(m)?;

    // Register engine builders (QasmEngineBuilder, etc.)
    engine_builders::register_engine_builders(m)?;

    // Register HUGR compilation functions
    hugr_compilation_bindings::register_hugr_compilation_functions(m)?;

    // Register program types
    m.add_class::<PyQasmProgram>()?;
    m.add_class::<PyQisProgram>()?;
    m.add_class::<PyHugrProgram>()?;
    m.add_class::<PyPhirJsonProgram>()?;

    // Register engine builder functions
    m.add_function(wrap_pyfunction!(engine_builders::qasm_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::qis_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::qis_control_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::native_runtime, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::phir_json_engine, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::sim_builder, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::general_noise, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::depolarizing_noise, m)?)?;
    m.add_function(wrap_pyfunction!(
        engine_builders::biased_depolarizing_noise,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(engine_builders::state_vector, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::sparse_stabilizer, m)?)?;
    m.add_function(wrap_pyfunction!(engine_builders::sparse_stab, m)?)?;

    // Utility functions
    m.add_function(wrap_pyfunction!(clear_jit_cache, m)?)?;

    Ok(())
}
