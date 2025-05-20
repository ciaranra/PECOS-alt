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

//! Tests for the `run_sim` function in the PECOS crate.

use pecos::prelude::*;
use pecos_engines::{PassThroughNoiseModel, engines::quantum::StateVecEngine};
use std::str::FromStr;

/// Simple bell state program for testing.
const BELL_STATE_QASM: &str = r#"
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
creg c[2];
h q[0];
cx q[0], q[1];
measure q -> c;
"#;

#[test]
fn test_run_sim_with_qasm_engine() {
    // Create a QASMEngine directly
    let engine = QASMEngine::from_str(BELL_STATE_QASM).unwrap();

    // Run simulation with explicit Box::new
    let results = run_sim(
        Box::new(engine),
        100,      // shots
        Some(42), // seed for determinism
        None,     // workers (default: 1)
        None,     // noise model (default: PassThroughNoiseModel)
        None,     // quantum engine (default: StateVecEngine)
    )
    .unwrap();

    // Verify results contain the 'c' register with 100 measurements
    assert!(results.register_shots.contains_key("c"));
    assert_eq!(results.register_shots["c"].len(), 100);

    // With the bell state and no noise, we should only see |00⟩ and |11⟩ states
    for result in &results.register_shots["c"] {
        // For a 2-qubit register, each shot result should be 0 or 3 (binary 00 or 11)
        assert!(*result == 0 || *result == 3);
    }
}

#[test]
fn test_run_sim_with_qasm_program() {
    // Create a QASMProgram
    let program = QASMProgram::from_str(BELL_STATE_QASM).unwrap();

    // Run simulation with into_engine_box method
    let results = run_sim(
        program.into_engine_box(),
        100,      // shots
        Some(42), // seed for determinism
        None,     // workers (default: 1)
        None,     // noise model (default: PassThroughNoiseModel)
        None,     // quantum engine (default: StateVecEngine)
    )
    .unwrap();

    // Verify results contain the 'c' register with 100 measurements
    assert!(results.register_shots.contains_key("c"));
    assert_eq!(results.register_shots["c"].len(), 100);

    // With the bell state and no noise, we should only see |00⟩ and |11⟩ states
    for result in &results.register_shots["c"] {
        // For a 2-qubit register, each shot result should be 0 or 3 (binary 00 or 11)
        assert!(*result == 0 || *result == 3);
    }
}

#[test]
fn test_run_sim_workers_parameter() {
    // Create QASMProgram
    let program = QASMProgram::from_str(BELL_STATE_QASM).unwrap();

    // Run simulation with 4 workers
    let results = run_sim(
        program.into_engine_box(),
        100,
        Some(42),
        Some(4), // 4 workers
        None,
        None,
    )
    .unwrap();

    // Verify results are correct
    assert!(results.register_shots.contains_key("c"));
    assert_eq!(results.register_shots["c"].len(), 100);
}

#[test]
fn test_run_sim_with_custom_noise_model() {
    // Create QASMProgram
    let program = QASMProgram::from_str(BELL_STATE_QASM).unwrap();

    // Create a custom noise model (PassThroughNoiseModel has no effect)
    let noise_model = Box::new(PassThroughNoiseModel);

    // Run simulation with custom noise model
    let results = run_sim(
        program.into_engine_box(),
        100,
        Some(42),
        None,
        Some(noise_model),
        None,
    )
    .unwrap();

    // Verify results are correct
    assert!(results.register_shots.contains_key("c"));
    assert_eq!(results.register_shots["c"].len(), 100);
}

#[test]
fn test_run_sim_with_custom_quantum_engine() {
    // Create QASMProgram
    let program = QASMProgram::from_str(BELL_STATE_QASM).unwrap();

    // Create a custom quantum engine
    let quantum_engine = Box::new(StateVecEngine::new(2)); // 2 qubits

    // Run simulation with custom quantum engine
    let results = run_sim(
        program.into_engine_box(),
        100,
        Some(42),
        None,
        None,
        Some(quantum_engine),
    )
    .unwrap();

    // Verify results are correct
    assert!(results.register_shots.contains_key("c"));
    assert_eq!(results.register_shots["c"].len(), 100);
}

#[test]
fn test_run_sim_determinism() {
    // Create two identical QASMProgram instances
    let program1 = QASMProgram::from_str(BELL_STATE_QASM).unwrap();
    let program2 = QASMProgram::from_str(BELL_STATE_QASM).unwrap();

    // Run simulations with the same seed
    let results1 = run_sim(
        program1.into_engine_box(),
        100,
        Some(42), // same seed
        None,
        None,
        None,
    )
    .unwrap();

    let results2 = run_sim(
        program2.into_engine_box(),
        100,
        Some(42), // same seed
        None,
        None,
        None,
    )
    .unwrap();

    // Results should be identical
    assert_eq!(results1, results2);

    // Now run with a different seed
    let program3 = QASMProgram::from_str(BELL_STATE_QASM).unwrap();
    let results3 = run_sim(
        program3.into_engine_box(),
        100,
        Some(43), // different seed
        None,
        None,
        None,
    )
    .unwrap();

    // Results should be different (this is probabilistic but very likely)
    // We're checking if the measurements are completely identical, which is
    // extremely unlikely with different seeds over 100 shots
    assert!(results1 != results3);
}

#[test]
fn test_run_sim_different_shots() {
    // Create QASMProgram
    let program = QASMProgram::from_str(BELL_STATE_QASM).unwrap();

    // Run with 50 shots
    let results1 = run_sim(
        program.clone().into_engine_box(),
        50,
        Some(42),
        None,
        None,
        None,
    )
    .unwrap();

    // Run with 200 shots
    let results2 = run_sim(program.into_engine_box(), 200, Some(42), None, None, None).unwrap();

    // Verify shot count matches
    assert_eq!(results1.register_shots["c"].len(), 50);
    assert_eq!(results2.register_shots["c"].len(), 200);
}
