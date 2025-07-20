//! Tests for the unified simulation API
//!
//! These tests demonstrate the consistent API across different engine types.

// Note: These are compile-time tests to verify the API consistency.
// Actual execution tests would require fully implemented engines.

#[test]
fn test_quantum_engine_builders() {
    use pecos_engines::{state_vector, sparse_stabilizer};
    
    // Test that quantum engine builders can be created and configured
    let _state_vec_builder = state_vector().qubits(4);
    let _sparse_stab_builder = sparse_stabilizer().qubits(4);
    
    // Test that builders can be created without qubit count (will be set later)
    let _state_vec_no_qubits = state_vector();
    let _sparse_stab_no_qubits = sparse_stabilizer();
    
    // Test chaining
    let _chained = sparse_stabilizer().qubits(2);
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
    use pecos_engines::sim_builder::SimConfig;
    
    let config = SimConfig::default();
    assert_eq!(config.workers, 1);
    assert!(config.seed.is_none());
    assert!(!config.verbose);
}