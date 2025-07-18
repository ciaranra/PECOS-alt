//! Example of using `GeneralNoiseModelBuilder` directly and via JSON configuration
//!
//! This example demonstrates:
//! 1. Direct builder usage (recommended)
//! 2. JSON configuration that converts to builders internally
//! 3. Complex noise model configurations

use pecos_core::gate_type::GateType;
use pecos_engines::noise::GeneralNoiseModel;
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

    // Example 1: Direct builder usage (recommended approach)
    println!("Example 1: Direct GeneralNoiseModelBuilder usage");
    let builder = GeneralNoiseModel::builder()
        .with_p1_probability(0.001)
        .with_p2_probability(0.01)
        .with_prep_probability(0.001)
        .with_meas_0_probability(0.001)
        .with_meas_1_probability(0.001)
        .with_seed(42);

    let noise_model = NoiseModelType::General(Box::new(builder));
    let results = qasm_sim(qasm).noise(noise_model).run(100).unwrap();
    println!("Shot results: {:?}", &results.shots[..5]);

    // Example 2: JSON configuration (converts to builder internally)
    println!("\nExample 2: JSON configuration (for backward compatibility)");
    let json_config = json!({
        "type": "GeneralNoise",
        "p1": 0.001,
        "p2": 0.01,
        "p_prep": 0.001,
        "p_meas_0": 0.001,
        "p_meas_1": 0.001,
        "seed": 42
    });

    let noise_config: NoiseConfig = serde_json::from_value(json_config).unwrap();
    let noise_model: NoiseModelType = noise_config.into();

    let results = qasm_sim(qasm).noise(noise_model).run(100).unwrap();
    println!("Shot results: {:?}", &results.shots[..5]);

    // Example 3: Complex builder configuration with all parameters
    println!("\nExample 3: Complex GeneralNoiseModelBuilder configuration");

    let mut p1_model = BTreeMap::new();
    p1_model.insert("X".to_string(), 0.5);
    p1_model.insert("Y".to_string(), 0.3);
    p1_model.insert("Z".to_string(), 0.2);

    let mut p2_model = BTreeMap::new();
    p2_model.insert("IX".to_string(), 0.1);
    p2_model.insert("IY".to_string(), 0.06);
    p2_model.insert("IZ".to_string(), 0.08);
    p2_model.insert("XI".to_string(), 0.1);
    p2_model.insert("XX".to_string(), 0.06);
    p2_model.insert("XY".to_string(), 0.06);
    p2_model.insert("XZ".to_string(), 0.06);
    p2_model.insert("YI".to_string(), 0.06);
    p2_model.insert("YX".to_string(), 0.06);
    p2_model.insert("YY".to_string(), 0.06);
    p2_model.insert("YZ".to_string(), 0.06);
    p2_model.insert("ZI".to_string(), 0.08);
    p2_model.insert("ZX".to_string(), 0.06);
    p2_model.insert("ZY".to_string(), 0.06);
    p2_model.insert("ZZ".to_string(), 0.04);

    let builder = GeneralNoiseModel::builder()
        .with_seed(123)
        .with_scale(1.5)
        .with_p1_probability(0.001)
        .with_p2_probability(0.01)
        .with_prep_probability(0.001)
        .with_meas_0_probability(0.002)
        .with_meas_1_probability(0.002)
        .with_noiseless_gate(GateType::H)
        .with_noiseless_gate(GateType::Measure)
        .with_p1_pauli_model(&p1_model)
        .with_p2_pauli_model(&p2_model)
        .with_p_idle_coherent(false)
        .with_p_idle_linear_rate(0.0001)
        .with_leakage_scale(0.5)
        .with_emission_scale(0.8);

    let noise_model = NoiseModelType::General(Box::new(builder));
    let results = qasm_sim(qasm)
        .noise(noise_model)
        .workers(4)
        .run(100)
        .unwrap();
    println!("Shot results: {:?}", &results.shots[..5]);

    // Example 4: Fluent API style
    println!("\nExample 4: Fluent API style (method chaining)");
    let results = qasm_sim(qasm)
        .noise(NoiseModelType::General(Box::new(
            GeneralNoiseModel::builder()
                .with_p1_probability(0.001)
                .with_p2_probability(0.01)
                .with_seed(789),
        )))
        .workers(4)
        .run(100)
        .unwrap();

    println!("Shot results: {:?}", &results.shots[..5]);
}
