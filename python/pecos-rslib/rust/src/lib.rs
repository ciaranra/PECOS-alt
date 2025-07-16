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
mod noise_helpers;
mod pcg_bindings;
mod phir_json_bridge;
#[cfg(feature = "hugr-llvm-pipeline")]
mod hugr_bindings;
mod llvm_bindings;
mod llvm_context_bindings;
mod llvm_execution_guard;
mod llvm_sim_bindings;
mod phir_bindings;
mod qasm_sim_bindings;
mod shot_results_bindings;
mod sparse_sim;
mod sparse_stab_bindings;
mod sparse_stab_engine_bindings;
mod state_vec_bindings;
mod state_vec_engine_bindings;

use byte_message_bindings::{PyByteMessage, PyByteMessageBuilder};
use shot_results_bindings::{PyShotMap, PyShotVec};
use sparse_stab_bindings::SparseSim;
use sparse_stab_engine_bindings::PySparseStabEngine;
use state_vec_bindings::RsStateVec;
use state_vec_engine_bindings::PyStateVecEngine;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn _pecos_rslib(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SparseSim>()?;
    m.add_class::<phir_json_bridge::PhirJsonEngine>()?;
    m.add_class::<RsStateVec>()?;
    m.add_class::<PyByteMessage>()?;
    m.add_class::<PyByteMessageBuilder>()?;
    m.add_class::<PyStateVecEngine>()?;
    m.add_class::<PySparseStabEngine>()?;
    
    // Shot result types
    m.add_class::<PyShotVec>()?;
    m.add_class::<PyShotMap>()?;

    // Register QASM simulation functions
    qasm_sim_bindings::register_qasm_sim_module(m)?;

    // Register HUGR/QIR functions (only if hugr-llvm-pipeline feature is enabled)
    #[cfg(feature = "hugr-llvm-pipeline")]
    hugr_bindings::register_hugr_module(m)?;

    // Register PHIR functions
    phir_bindings::register_phir_module(m)?;

    // Register LLVM execution functions
    llvm_bindings::register_llvm_module(m)?;

    // Register LlvmSim functions
    llvm_sim_bindings::register_llvm_sim_module(m)?;

    pcg_bindings::create_pcg_module(m)?;
    Ok(())
}
