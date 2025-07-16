//! Tests for the unified simulation API
//!
//! These tests demonstrate the consistent API across different engine types.

// Note: These are compile-time tests to verify the API consistency.
// Actual execution tests would require fully implemented engines.

#[test]
fn test_unified_api_consistency() {
    use pecos_engines::{QuantumEngineType};
    
    // We can't easily create a mock engine due to the complex trait requirements,
    // so we'll test the API using the real engine builders from other crates.
    // This test is mainly to ensure the API types and methods exist and compile.
    
    // Test that QuantumEngineType has expected variants
    let _state_vec = QuantumEngineType::StateVector;
    let _sparse_stab = QuantumEngineType::SparseStabilizer;
    
    // Test that default is SparseStabilizer
    assert_eq!(QuantumEngineType::default(), QuantumEngineType::SparseStabilizer);
}

#[test] 
fn test_noise_conversions() {
    use pecos_engines::{
        PassThroughNoise, DepolarizingNoise, DepolarizingCustomNoise, BiasedDepolarizingNoise,
        noise::{NoiseModel, GeneralNoiseModelBuilder},
    };
    
    // Test that all noise types can be converted
    let _: Box<dyn NoiseModel> = PassThroughNoise.into();
    let _: Box<dyn NoiseModel> = DepolarizingNoise { p: 0.01 }.into();
    let _: Box<dyn NoiseModel> = DepolarizingCustomNoise { 
        p_prep: 0.001,
        p_meas: 0.002,
        p1: 0.003,
        p2: 0.004,
    }.into();
    let _: Box<dyn NoiseModel> = BiasedDepolarizingNoise { p: 0.01 }.into();
    let _: Box<dyn NoiseModel> = GeneralNoiseModelBuilder::new().into();
}

#[test]
fn test_sim_config() {
    use pecos_engines::{SimConfig, QuantumEngineType};
    
    let config = SimConfig::default();
    assert_eq!(config.workers, 1);
    assert_eq!(config.quantum_engine, QuantumEngineType::SparseStabilizer);
    assert!(config.seed.is_none());
    assert!(config.max_qubits.is_none());
    assert!(!config.verbose);
}