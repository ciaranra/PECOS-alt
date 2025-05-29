use pecos_engines::byte_message::ByteMessage;
use pecos_engines::noise::{
    BiasedMeasurementNoiseModel, DepolarizingNoiseModel, NoiseModel, PassThroughNoiseModel,
};
use pecos_engines::quantum::StateVecEngine;
use pecos_engines::{Engine, EngineSystem, QuantumSystem};
use std::collections::HashMap;

// Helper function to count measurement results from multiple shots
fn count_results(
    noise_model: Box<dyn NoiseModel>,
    circ: &ByteMessage,
    num_shots: usize,
    num_qubits: usize,
) -> HashMap<String, usize> {
    let quantum = Box::new(StateVecEngine::new(num_qubits));
    let mut system = QuantumSystem::new(noise_model, quantum);
    system.set_seed(42).expect("Failed to set seed");

    let mut counts = HashMap::new();
    for _ in 0..num_shots {
        system.reset().expect("Failed to reset system");
        let results = system
            .process_as_system(circ.clone())
            .expect("Failed to process circuit");
        let measurements = results
            .parse_measurements()
            .expect("Failed to parse measurements");

        let result_str = measurements
            .iter()
            .map(|&(_, value)| if value == 1 { '1' } else { '0' })
            .collect::<String>();

        *counts.entry(result_str).or_insert(0) += 1;
    }

    counts
}

#[test]
fn test_biased_measurement_noise() {
    // Create a simple H-gate circuit with measurement
    let circ = ByteMessage::quantum_operations_builder()
        .add_h(&[0])
        .add_measurements(&[0], &[0])
        .build();

    // Test with different bias probabilities
    let configs = [
        (0.0, 0.0, "No bias"),     // Should be approximately 50-50
        (0.2, 0.0, "0->1 only"),   // Should bias toward 1
        (0.0, 0.2, "1->0 only"),   // Should bias toward 0
        (1.0, 0.0, "Always 0->1"), // Should always output 1
        (0.0, 1.0, "Always 1->0"), // Should always output 0
    ];

    for (p_flip_0, p_flip_1, desc) in configs {
        // Create biased measurement noise model
        let noise = BiasedMeasurementNoiseModel::builder()
            .with_prob_flip_from_0(p_flip_0)
            .with_prob_flip_from_1(p_flip_1)
            .with_seed(42)
            .build();

        // Get distribution after 1000 shots
        let counts = count_results(noise, &circ, 1000, 1);

        // Calculate percentages
        let count_0 = *counts.get("0").unwrap_or(&0);
        let count_1 = *counts.get("1").unwrap_or(&0);
        let pct_0 = count_0 * 100 / 1000;
        let pct_1 = count_1 * 100 / 1000;

        // Calculate expected percentages - with Hadamard, ideally 50% each
        // For a 50/50 input with H gate:
        // Expected 0s = 50% * (1-p_flip_0) + 50% * p_flip_1
        // Expected 1s = 50% * p_flip_0 + 50% * (1-p_flip_1)
        let expected_pct_0 = (0.5 * (1.0 - p_flip_0) + 0.5 * p_flip_1) * 100.0;
        let expected_pct_1 = (0.5 * p_flip_0 + 0.5 * (1.0 - p_flip_1)) * 100.0;

        // Allow for some statistical variance (±5%)
        let margin = 5;

        #[allow(clippy::cast_precision_loss)]
        let diff_0 = (pct_0 as f64 - expected_pct_0).abs();
        #[allow(clippy::cast_precision_loss)]
        let diff_1 = (pct_1 as f64 - expected_pct_1).abs();

        assert!(
            diff_0 <= f64::from(margin),
            "{desc}: Expected {expected_pct_0}% zeros, got {pct_0}%"
        );
        assert!(
            diff_1 <= f64::from(margin),
            "{desc}: Expected {expected_pct_1}% ones, got {pct_1}%"
        );
    }
}

#[test]
fn test_depolarizing_noise() {
    // Create a Bell state circuit
    let bell_circ = ByteMessage::quantum_operations_builder()
        .add_h(&[0])
        .add_cx(&[0], &[1])
        .add_measurements(&[0], &[0])
        .add_measurements(&[1], &[1])
        .build();

    // Test with no noise - should get ideal Bell state (00 and 11)
    let no_noise = DepolarizingNoiseModel::builder()
        .with_uniform_probability(0.0)
        .with_seed(42)
        .build();

    let ideal_counts = count_results(no_noise, &bell_circ, 1000, 2);

    // In the ideal case, we expect only 00 and 11 with roughly equal probability
    assert!(
        ideal_counts.get("01").unwrap_or(&0) + ideal_counts.get("10").unwrap_or(&0) < 50,
        "Ideal Bell state should have negligible 01 and 10 results"
    );

    let count_00 = *ideal_counts.get("00").unwrap_or(&0);
    let count_11 = *ideal_counts.get("11").unwrap_or(&0);

    // Allow truncation and wrap in test code
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let diff = (count_00 as i32 - count_11 as i32).abs();

    assert!(
        diff < 100,
        "00 and 11 should be approximately equal in ideal Bell state"
    );

    // Test with moderate noise - should see some 01 and 10 results
    let moderate_noise = DepolarizingNoiseModel::builder()
        .with_uniform_probability(0.1)
        .with_seed(42)
        .build();

    let noisy_counts = count_results(moderate_noise, &bell_circ, 1000, 2);

    // With noise, we expect to see some 01 and 10 results
    assert!(
        noisy_counts.get("01").unwrap_or(&0) + noisy_counts.get("10").unwrap_or(&0) > 50,
        "Noisy Bell state should have some 01 and 10 results"
    );
}

#[test]
fn test_pass_through_noise() {
    // Create a simple H-gate circuit with measurement
    let circ = ByteMessage::quantum_operations_builder()
        .add_h(&[0])
        .add_measurements(&[0], &[0])
        .build();

    // Create pass-through noise model (no noise)
    let no_noise = Box::new(PassThroughNoiseModel);

    // Run with 1000 shots
    let counts = count_results(no_noise, &circ, 1000, 1);

    // With a Hadamard gate, we expect roughly 50% 0s and 50% an 1s
    let count_0 = *counts.get("0").unwrap_or(&0);
    let count_1 = *counts.get("1").unwrap_or(&0);

    // Allow for some statistical variance (±5%)
    #[allow(clippy::cast_precision_loss)]
    let diff_0 = (count_0 as f64 - 500.0).abs();
    #[allow(clippy::cast_precision_loss)]
    let diff_1 = (count_1 as f64 - 500.0).abs();

    assert!(
        diff_0 <= 50.0,
        "Expected approximately 500 zeros, got {count_0}"
    );
    assert!(
        diff_1 <= 50.0,
        "Expected approximately 500 ones, got {count_1}"
    );
}
