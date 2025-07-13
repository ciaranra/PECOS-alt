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

use pecos::prelude::*;
use pyo3::prelude::*;

#[pyfunction]
#[pyo3(name = "pcg32_random")]
pub fn py_pcg32_random() -> u32 {
    pcg32_random()
}

#[pyfunction]
#[pyo3(name = "pcg32_boundedrand")]
pub fn py_pcg32_boundedrand(bound: u32) -> u32 {
    pcg32_boundedrand(bound)
}

#[pyfunction]
#[pyo3(name = "pcg32_frandom")]
pub fn py_pcg32_frandom() -> f64 {
    pcg32_frandom()
}

#[pyfunction]
#[pyo3(name = "pcg32_srandom")]
pub fn py_pcg32_srandom(seq: u64) {
    pcg32_srandom(seq);
}

/// Create a submodule for PCG functions
pub fn create_pcg_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let pcg_module = PyModule::new(m.py(), "pcg")?;
    pcg_module.add_function(wrap_pyfunction!(py_pcg32_random, &pcg_module)?)?;
    pcg_module.add_function(wrap_pyfunction!(py_pcg32_boundedrand, &pcg_module)?)?;
    pcg_module.add_function(wrap_pyfunction!(py_pcg32_frandom, &pcg_module)?)?;
    pcg_module.add_function(wrap_pyfunction!(py_pcg32_srandom, &pcg_module)?)?;
    m.add_submodule(&pcg_module)?;
    Ok(())
}
