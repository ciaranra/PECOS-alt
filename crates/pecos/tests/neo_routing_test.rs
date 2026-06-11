// Copyright 2026 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License. You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! Contract tests for routing `sim()` to the pecos-neo stack.
//!
//! The neo stack must return the same `ShotVec` contract as the engines
//! stack: for deterministic programs, results are compared for exact
//! equality across stacks.

#![cfg(feature = "neo")]

use pecos::{SimStack, sim};
use pecos_programs::Qasm;

/// Deterministic program exercising measurement feedback: c ends as "11".
fn deterministic_conditional_qasm() -> Qasm {
    Qasm::from_string(
        r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        x q[0];
        measure q[0] -> c[0];
        if (c[0] == 1) x q[1];
        measure q[1] -> c[1];
        "#,
    )
}

#[test]
fn neo_stack_matches_engines_for_deterministic_qasm() {
    let engines = sim(deterministic_conditional_qasm())
        .seed(42)
        .run(5)
        .expect("engines run");

    let neo = sim(deterministic_conditional_qasm())
        .stack(SimStack::Neo)
        .seed(42)
        .run(5)
        .expect("neo run");

    assert_eq!(engines.shots.len(), 5);
    assert_eq!(
        engines, neo,
        "Deterministic program must produce identical ShotVecs on both stacks"
    );
    for shot in &neo.shots {
        assert_eq!(shot.data["c"].to_bitstring().unwrap(), "11");
    }
}

#[test]
fn neo_stack_parallel_matches_engines() {
    let engines = sim(deterministic_conditional_qasm())
        .seed(7)
        .workers(2)
        .run(6)
        .expect("engines run");

    let neo = sim(deterministic_conditional_qasm())
        .stack(SimStack::Neo)
        .seed(7)
        .workers(2)
        .run(6)
        .expect("neo run");

    assert_eq!(engines, neo);
}

#[test]
fn neo_stack_rejects_unrouted_noise() {
    let err = sim(deterministic_conditional_qasm())
        .stack(SimStack::Neo)
        .noise(pecos_engines::DepolarizingNoise { p: 0.01 })
        .run(5)
        .expect_err("noise is not yet routed to the neo stack");
    assert!(
        err.to_string().contains("not yet routed to the neo stack"),
        "unexpected error: {err}"
    );
}

#[test]
fn neo_stack_rejects_unrouted_quantum_backend() {
    let err = sim(deterministic_conditional_qasm())
        .stack(SimStack::Neo)
        .quantum(pecos_engines::state_vector())
        .run(5)
        .expect_err("explicit quantum backends are not yet routed");
    assert!(err.to_string().contains("not yet routed to the neo stack"));
}

#[test]
fn neo_stack_rejects_build() {
    let Err(err) = sim(deterministic_conditional_qasm())
        .stack(SimStack::Neo)
        .build()
    else {
        panic!("neo stack has no MonteCarloEngine; build() must error");
    };
    assert!(err.to_string().contains("MonteCarloEngine"));
}
