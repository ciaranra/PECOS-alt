// Tests for the new qasm_sim API

use pecos_qasm::prelude::*;
use std::collections::HashMap;

#[test]
fn test_simple_run() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    let results = qasm_sim(qasm).run(100).unwrap();
    assert_eq!(results.len(), 100);

    // Check Bell state results
    let shot_map = results.try_as_shot_map().unwrap();
    let values = shot_map.try_bits_as_u64("c").unwrap();

    for val in values {
        assert!(val == 0 || val == 3); // |00> or |11>
    }
}

#[test]
fn test_build_once_run_multiple() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
    "#;

    let sim = qasm_sim(qasm).seed(42).build().unwrap();

    // Run multiple times
    let results1 = sim.run(100).unwrap();
    let results2 = sim.run(1000).unwrap();
    let results3 = sim.run(10).unwrap();

    assert_eq!(results1.len(), 100);
    assert_eq!(results2.len(), 1000);
    assert_eq!(results3.len(), 10);
}

#[test]
fn test_with_depolarizing_noise() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
    "#;

    let results = qasm_sim(qasm)
        .seed(42)
        .noise(DepolarizingNoise { p: 0.1 })
        .run(1000)
        .unwrap();

    let shot_map = results.try_as_shot_map().unwrap();
    let values = shot_map.try_bits_as_u64("c").unwrap();

    // Count errors
    let errors = values.iter().filter(|&&v| v == 0).count();

    // With 10% noise, expect some errors
    assert!(errors > 50);
    assert!(errors < 200);
}

#[test]
fn test_custom_depolarizing_noise() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    let results = qasm_sim(qasm)
        .seed(42)
        .noise(DepolarizingCustomNoise {
            p_prep: 0.01,
            p_meas: 0.01,
            p1: 0.001,
            p2: 0.1, // High two-qubit error
        })
        .run(1000)
        .unwrap();

    let shot_map = results.try_as_shot_map().unwrap();
    let values = shot_map.try_bits_as_u64("c").unwrap();

    // Count non-Bell states
    let mut counts = HashMap::new();
    for val in values {
        *counts.entry(val).or_insert(0) += 1;
    }

    // Should see some errors (01 and 10 states)
    assert!(counts.contains_key(&1) || counts.contains_key(&2));
}

#[test]
fn test_biased_measurement_noise() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
    "#;

    let results = qasm_sim(qasm)
        .seed(42)
        .noise(BiasedMeasurementNoise {
            p0: 0.0, // No 0->1 errors
            p1: 0.2, // 20% 1->0 errors
        })
        .run(1000)
        .unwrap();

    let shot_map = results.try_as_shot_map().unwrap();
    let values = shot_map.try_bits_as_u64("c").unwrap();

    let zeros = values.iter().filter(|&&v| v == 0).count();

    // Should see ~20% errors
    assert!(zeros > 150);
    assert!(zeros < 250);
}

#[test]
fn test_state_vector_engine() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        rz(0.5) q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    // StateVector can handle non-Clifford gates
    let results = qasm_sim(qasm)
        .seed(42)
        .quantum_engine(QuantumEngineType::StateVector)
        .run(100)
        .unwrap();

    assert_eq!(results.len(), 100);
}

#[test]
fn test_auto_workers() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        creg c[3];
        h q[0];
        h q[1];
        h q[2];
        measure q -> c;
    "#;

    let results = qasm_sim(qasm).seed(42).auto_workers().run(1000).unwrap();

    assert_eq!(results.len(), 1000);
}

#[test]
fn test_deterministic_with_seed() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
    "#;

    // Run twice with same seed
    let sim = qasm_sim(qasm).seed(123).build().unwrap();

    let results1 = sim.run(100).unwrap();
    let results2 = sim.run(100).unwrap();

    // Convert to comparable format
    let map1 = results1.try_as_shot_map().unwrap();
    let map2 = results2.try_as_shot_map().unwrap();

    let values1 = map1.try_bits_as_u64("c").unwrap();
    let values2 = map2.try_bits_as_u64("c").unwrap();

    // Same seed should give same results
    assert_eq!(values1, values2);
}

#[test]
fn test_full_configuration() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    let sim = qasm_sim(qasm)
        .seed(42)
        .workers(2)
        .quantum_engine(QuantumEngineType::SparseStabilizer)
        .noise(BiasedDepolarizingNoise { p: 0.01 })
        .build()
        .unwrap();

    // Run multiple times
    for shots in [10, 100, 1000] {
        let results = sim.run(shots).unwrap();
        assert_eq!(results.len(), shots);
    }
}

#[test]
fn test_passthrough_noise() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
    "#;

    let results = qasm_sim(qasm).noise(PassThroughNoise).run(100).unwrap();

    let shot_map = results.try_as_shot_map().unwrap();
    let values = shot_map.try_bits_as_u64("c").unwrap();

    // No noise, all should be 1
    assert!(values.iter().all(|&v| v == 1));
}

#[test]
fn test_general_noise() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        measure q[0] -> c[0];
    "#;

    // Should not panic with general noise
    let results = qasm_sim(qasm).noise(GeneralNoise).run(10).unwrap();

    assert_eq!(results.len(), 10);
}
