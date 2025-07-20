//! QASM simulation API
//!
//! This module provides the `qasm_sim()` function which is now a thin wrapper
//! around the unified simulation API (`qasm_engine().program().to_sim()`).

use crate::unified_engine_builder::qasm_engine;
use pecos_programs::QasmProgram;
use pecos_engines::ClassicalControlEngineBuilder;

/// Create a new QASM simulation builder
///
/// This function now directly returns the unified `SimBuilder` with all the 
/// configuration methods available from the unified API.
///
/// # Example
///
/// ```
/// use pecos_qasm::qasm_engine;
/// use pecos_programs::QasmProgram;
/// use pecos_engines::{ClassicalControlEngineBuilder, noise::DepolarizingNoiseModel};
///
/// let qasm = r#"
///     OPENQASM 2.0;
///     include "qelib1.inc";
///     qreg q[2];
///     creg c[2];
///     h q[0];
///     cx q[0], q[1];
///     measure q -> c;
/// "#;
///
/// // Run with default settings (no noise)
/// let results = qasm_engine()
///     .program(QasmProgram::from_string(qasm))
///     .to_sim()
///     .run(100)
///     .unwrap();
///
/// // Run with noise
/// let noise_builder = DepolarizingNoiseModel::builder()
///     .with_p1_probability(0.001)
///     .with_p2_probability(0.01)
///     .with_prep_probability(0.001)
///     .with_meas_probability(0.001);
///
/// let results = qasm_engine()
///     .program(QasmProgram::from_string(qasm))
///     .to_sim()
///     .seed(42)
///     .noise(noise_builder)
///     .run(1000)
///     .unwrap();
/// ```
#[must_use]
pub fn qasm_sim(qasm: impl Into<String>) -> pecos_engines::SimBuilder<crate::unified_engine_builder::QasmEngineBuilder> {
    qasm_engine().program(QasmProgram::from_string(qasm)).to_sim()
}