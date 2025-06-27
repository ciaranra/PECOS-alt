//! Example of using `GeneralNoise` with JSON configuration

use pecos_engines::noise::GeneralNoiseModelBuilder;
use pecos_qasm::config::NoiseConfig;
use pecos_qasm::simulation::{NoiseModelType, qasm_sim};
use serde_json::json;
use std::collections::BTreeMap;

fn main() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    // Example 1: Simple configuration from JSON
    println!("Example 1: Simple GeneralNoise from JSON");
    let json_config = json!({
        "type": "GeneralNoise",
        "p1": 0.001,
        "p2": 0.01,
        "p_prep": 0.001,
        "p_meas_0": 0.001,
        "p_meas_1": 0.001
    });

    let noise_config: NoiseConfig = serde_json::from_value(json_config).unwrap();
    let noise_model: NoiseModelType = noise_config.into();

    let results = qasm_sim(qasm).noise(noise_model).seed(42).run(100).unwrap();

    println!("Shot results: {:?}", &results.shots[..5]);

    // Example 2: Complex configuration with all parameters
    println!("\nExample 2: Complex GeneralNoise configuration");
    let json_config = json!({
        "type": "GeneralNoise",
        "seed": 123,
        "scale": 1.5,
        "p1": 0.001,
        "p2": 0.01,
        "p_prep": 0.001,
        "p_meas_0": 0.002,
        "p_meas_1": 0.002,
        "noiseless_gates": ["H", "MEASURE"],
        "p1_pauli_model": {
            "X": 0.5,
            "Y": 0.3,
            "Z": 0.2
        },
        "p2_pauli_model": {
            "IX": 0.1,
            "IY": 0.06,
            "IZ": 0.08,
            "XI": 0.1,
            "XX": 0.06,
            "XY": 0.06,
            "XZ": 0.06,
            "YI": 0.06,
            "YX": 0.06,
            "YY": 0.06,
            "YZ": 0.06,
            "ZI": 0.08,
            "ZX": 0.06,
            "ZY": 0.06,
            "ZZ": 0.04
        },
        "p_idle_coherent": false,
        "p_idle_linear_rate": 0.0001,
        "leakage_scale": 0.5,
        "emission_scale": 0.8
    });

    let noise_config: NoiseConfig = serde_json::from_value(json_config).unwrap();
    let noise_model: NoiseModelType = noise_config.into();

    let results = qasm_sim(qasm)
        .noise(noise_model)
        .workers(4)
        .run(100)
        .unwrap();

    println!("Shot results: {:?}", &results.shots[..5]);

    // Example 3: Programmatic configuration (for comparison)
    println!("\nExample 3: Programmatic GeneralNoise configuration");

    let mut p1_model = BTreeMap::new();
    p1_model.insert("X".to_string(), 0.5);
    p1_model.insert("Y".to_string(), 0.3);
    p1_model.insert("Z".to_string(), 0.2);

    let builder = GeneralNoiseModelBuilder::new()
        .with_p1_probability(0.001)
        .with_p2_probability(0.01)
        .with_prep_probability(0.001)
        .with_meas_0_probability(0.002)
        .with_meas_1_probability(0.002)
        .with_scale(1.5)
        .with_p1_pauli_model(&p1_model)
        .with_seed(456);

    let noise_model = NoiseModelType::GeneralFromBuilder(Box::new(builder));

    let results = qasm_sim(qasm).noise(noise_model).run(100).unwrap();

    println!("Shot results: {:?}", &results.shots[..5]);
}
