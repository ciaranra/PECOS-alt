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

//! Statistical equivalence matrix between the engines and neo stacks
//! (validation-gate item V1).
//!
//! Each cell runs the same QASM program with the same mapped noise
//! configuration through `sim()` on both stacks and compares the target
//! outcome rate with Jeffreys credible intervals: the two stacks'
//! intervals must overlap, and where the rate has an exact analytic
//! value, each stack's interval must contain it.
//!
//! Program-type coverage beyond QASM (HUGR) and exact worker-count
//! invariance are covered by `neo_routing_test.rs`; surface-code-scale
//! decoded equivalence is covered by `neo_surface_ler_test.rs`.

#![cfg(feature = "neo")]

use pecos::{SimStack, sim};
use pecos_engines::shot_results::ShotVec;
use pecos_num::jeffreys_interval;
use pecos_programs::Qasm;

const SHOTS: usize = 20_000;
/// ~4.4 sigma per side: stack disagreement, not sampling noise, is what
/// fails a cell.
const CONFIDENCE: f64 = 0.99999;
const SEED: u64 = 42;

const X_MEASURE: &str = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    x q[0];
    measure q[0] -> c[0];
"#;

const RESET_MEASURE: &str = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[1];
    creg c[1];
    reset q[0];
    measure q[0] -> c[0];
"#;

const BELL: &str = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0],q[1];
    measure q[0] -> c[0];
    measure q[1] -> c[1];
"#;

const FEEDBACK: &str = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    x q[0];
    measure q[0] -> c[0];
    if (c == 1) x q[1];
    measure q[1] -> c[1];
"#;

/// One noise configuration of the matrix, applied identically to both
/// stacks (the facade maps it to neo's noise channels).
enum NoiseCell {
    Meas(f64),
    Prep(f64),
    P1(f64),
    P2(f64),
    Uniform(f64),
    GnmSimple { average_p1: f64, p_meas: f64 },
}

impl NoiseCell {
    fn run(&self, qasm: &str, stack: SimStack) -> ShotVec {
        let builder = sim(Qasm::from_string(qasm)).stack(stack).seed(SEED);
        let depol = |p_prep: f64, p_meas: f64, p1: f64, p2: f64| {
            pecos_engines::noise::DepolarizingNoiseModel::builder()
                .with_prep_probability(p_prep)
                .with_meas_probability(p_meas)
                .with_p1_probability(p1)
                .with_p2_probability(p2)
        };
        let results = match *self {
            Self::Meas(p) => builder.noise(depol(0.0, p, 0.0, 0.0)).run(SHOTS),
            Self::Prep(p) => builder.noise(depol(p, 0.0, 0.0, 0.0)).run(SHOTS),
            Self::P1(p) => builder.noise(depol(0.0, 0.0, p, 0.0)).run(SHOTS),
            Self::P2(p) => builder.noise(depol(0.0, 0.0, 0.0, p)).run(SHOTS),
            Self::Uniform(p) => builder
                .noise(pecos_engines::DepolarizingNoise { p })
                .run(SHOTS),
            Self::GnmSimple { average_p1, p_meas } => builder
                .noise(
                    // GeneralNoiseModel has realistic non-zero defaults;
                    // zero everything outside the simple Pauli subset so
                    // the cell physics is exactly known.
                    pecos_engines::noise::GeneralNoiseModel::builder()
                        .with_average_p1_probability(average_p1)
                        .with_average_p2_probability(0.0)
                        .with_p1_emission_ratio(0.0)
                        .with_p2_emission_ratio(0.0)
                        .with_prep_leak_ratio(0.0)
                        .with_p_idle_linear_rate(0.0)
                        .with_prep_probability(0.0)
                        .with_meas_0_probability(p_meas)
                        .with_meas_1_probability(p_meas),
                )
                .run(SHOTS),
        };
        results.expect("simulation run")
    }
}

/// Count shots whose register `c` reads any of the target bitstrings.
///
/// Every target set used here is symmetric under bit reversal, so the
/// count is independent of register bit ordering.
fn count_targets(results: &ShotVec, targets: &[&str]) -> u64 {
    results
        .shots
        .iter()
        .filter(|shot| {
            let bits = shot.data["c"].to_bitstring().expect("c register bits");
            targets.contains(&bits.as_str())
        })
        .count() as u64
}

/// Run one matrix cell on both stacks and apply the equivalence (and
/// optional analytic-truth) assertions.
fn check_cell(name: &str, qasm: &str, cell: &NoiseCell, targets: &[&str], analytic: Option<f64>) {
    let engines = count_targets(&cell.run(qasm, SimStack::Engines), targets);
    let neo = count_targets(&cell.run(qasm, SimStack::Neo), targets);

    let engines_ci = jeffreys_interval(engines, SHOTS as u64, CONFIDENCE);
    let neo_ci = jeffreys_interval(neo, SHOTS as u64, CONFIDENCE);
    println!(
        "{name}: engines {engines}/{SHOTS} CI [{:.5}, {:.5}], \
         neo {neo}/{SHOTS} CI [{:.5}, {:.5}], analytic {analytic:?}",
        engines_ci.0, engines_ci.1, neo_ci.0, neo_ci.1
    );

    assert!(
        engines_ci.0 <= neo_ci.1 && neo_ci.0 <= engines_ci.1,
        "{name}: stack rates are statistically incompatible: \
         engines {engines}/{SHOTS} vs neo {neo}/{SHOTS}"
    );

    if let Some(truth) = analytic {
        assert!(
            engines_ci.0 <= truth && truth <= engines_ci.1,
            "{name}: engines rate {engines}/{SHOTS} excludes the analytic value {truth}"
        );
        assert!(
            neo_ci.0 <= truth && truth <= neo_ci.1,
            "{name}: neo rate {neo}/{SHOTS} excludes the analytic value {truth}"
        );
    }
}

#[test]
fn meas_only_rates_match() {
    // Measurement flip only: P(c = 0 after X) = p_meas exactly.
    check_cell(
        "meas_only",
        X_MEASURE,
        &NoiseCell::Meas(0.2),
        &["0"],
        Some(0.2),
    );
}

#[test]
fn prep_only_rates_match() {
    // Preparation error only: P(c = 1 after reset) = p_prep exactly.
    check_cell(
        "prep_only",
        RESET_MEASURE,
        &NoiseCell::Prep(0.15),
        &["1"],
        Some(0.15),
    );
}

#[test]
fn p1_only_rates_match() {
    // Uniform 1q depolarizing after the X gate: X and Y flip the Z-basis
    // outcome, Z does not, so P(c = 0) = 2p/3 exactly.
    check_cell("p1_only", X_MEASURE, &NoiseCell::P1(0.3), &["0"], Some(0.2));
}

#[test]
fn p2_only_anticorrelation_matches() {
    // Uniform 2q depolarizing after the Bell CX: of the 15 two-qubit
    // Paulis, the 8 with exactly one X/Y factor anticommute with Z(x)Z
    // and break the outcome correlation, so P(01 or 10) = 8p/15 exactly.
    check_cell(
        "p2_only",
        BELL,
        &NoiseCell::P2(0.3),
        &["01", "10"],
        Some(8.0 * 0.3 / 15.0),
    );
}

#[test]
fn uniform_depolarizing_compound_matches() {
    // All channels at p = 0.1 on the x-measure program: the compound
    // error rate has no simple closed form; cross-stack agreement only.
    check_cell(
        "uniform_depol",
        X_MEASURE,
        &NoiseCell::Uniform(0.1),
        &["0"],
        None,
    );
}

#[test]
fn gnm_simple_subset_matches() {
    // GeneralNoiseModel's "average" convention stores p1 = 1.5 x average
    // (0.3 here, flip 0.2) on both stacks, composed with a 5% measurement
    // flip: P(c = 0) = 0.2 * 0.95 + 0.8 * 0.05 = 0.23 exactly.
    check_cell(
        "gnm_simple",
        X_MEASURE,
        &NoiseCell::GnmSimple {
            average_p1: 0.2,
            p_meas: 0.05,
        },
        &["0"],
        Some(0.23),
    );
}

#[test]
fn feedback_under_measurement_noise_matches() {
    // Conditional feedback with noisy measurement: the recorded c[0]
    // drives the correction, so both stacks must apply the measurement
    // flip with the same record-vs-state semantics to agree here.
    check_cell(
        "feedback_meas_noise",
        FEEDBACK,
        &NoiseCell::Meas(0.1),
        &["11"],
        None,
    );
}
