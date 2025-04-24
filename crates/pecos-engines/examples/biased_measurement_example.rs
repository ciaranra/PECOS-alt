use pecos_engines::Engine;
use pecos_engines::byte_message::ByteMessage;
use pecos_engines::engines::noise::BiasedMeasurementNoise;
use pecos_engines::engines::quantum::StateVecEngine;
use pecos_engines::{EngineSystem, QuantumSystem};
use std::collections::HashMap;

fn main() {
    // Create a simple quantum circuit that prepares a superposition and measures it
    // We expect a roughly 50/50 distribution of 0s and 1s in the ideal case
    let circ = ByteMessage::quantum_operations_builder()
        .add_h(&[0])
        .add_measurements(&[0], &[0])
        .build();

    // Create a quantum engine with 1 qubit
    let quantum = Box::new(StateVecEngine::new(1));

    example1_different_bias_levels(&circ, quantum.clone());
    example2_with_seed(&circ);
    example3_bell_state();
}

fn example1_different_bias_levels(circ: &ByteMessage, quantum: Box<StateVecEngine>) {
    // === EXAMPLE 1: Different bias levels ===
    println!("Example 1: Testing different bias levels with 10,000 shots");
    println!("{:-^80}", "");
    println!(
        "{:<30} | {:<10} | {:<10} | {:<10} | {:<10}",
        "Configuration", "Expected 0", "Actual 0", "Expected 1", "Actual 1"
    );
    println!("{:-^80}", "");

    // Try different bias configurations
    let configs = [
        (0.0, 0.0, "No bias (ideal)"),
        (0.2, 0.0, "20% flip 0->1, never 1->0"),
        (0.0, 0.2, "Never flip 0->1, 20% flip 1->0"),
        (0.3, 0.3, "30% flip both ways"),
        (0.5, 0.0, "50% flip 0->1, never 1->0"),
        (0.0, 0.5, "Never flip 0->1, 50% flip 1->0"),
        (1.0, 0.0, "Always flip 0->1, never 1->0"),
        (0.0, 1.0, "Never flip 0->1, always 1->0"),
    ];

    // Increased number of shots for more stable statistics
    let num_shots = 10000;

    for (p_flip_0, p_flip_1, desc) in configs {
        // Create the biased measurement noise model
        let noise = Box::new(BiasedMeasurementNoise::new(p_flip_0, p_flip_1));
        let mut system = QuantumSystem::new(noise, quantum.clone());

        // For deterministic testing, set a fixed seed
        system.set_seed(42).expect("Failed to set seed");

        // Run the circuit multiple times and collect statistics
        let mut counts = HashMap::new();

        for _ in 0..num_shots {
            system.reset().expect("Failed to reset");
            let results = system
                .process_as_system(circ.clone())
                .expect("Failed to process circuit");
            let measurements = results
                .parse_measurements()
                .expect("Failed to parse measurements");

            // Each measurement result is a tuple of (qubit_index, value)
            let result = measurements
                .first()
                .map_or("?", |&(_, value)| if value == 1 { "1" } else { "0" });
            *counts.entry(result.to_string()).or_insert(0) += 1;
        }

        // Calculate percentages
        let pct_0 = counts.get("0").unwrap_or(&0) * 100 / num_shots;
        let pct_1 = counts.get("1").unwrap_or(&0) * 100 / num_shots;

        // Calculate expected results based on probabilities
        // For a 50/50 input with H gate:
        // Expected 0s = 50% * (1-p_flip_0) + 50% * p_flip_1
        // Expected 1s = 50% * p_flip_0 + 50% * (1-p_flip_1)
        let expected_0 = ((50.0 * (1.0 - p_flip_0) + 50.0 * p_flip_1) as f64).round() as usize;
        let expected_1 = ((50.0 * p_flip_0 + 50.0 * (1.0 - p_flip_1)) as f64).round() as usize;

        println!(
            "{:<30} | {:<10} | {:<10} | {:<10} | {:<10}",
            desc,
            format!("{}%", expected_0),
            format!("{}%", pct_0),
            format!("{}%", expected_1),
            format!("{}%", pct_1)
        );
    }
    println!("{:-^80}", "");
    println!();
}

fn example2_with_seed(circ: &ByteMessage) {
    // === EXAMPLE 2: Using direct constructor with seed ===
    println!("Example 2: Using direct constructor with seed");

    let noise = Box::new(BiasedMeasurementNoise::with_seed(0.4, 0.1, 123));
    let quantum = Box::new(StateVecEngine::new(1));
    let mut system = QuantumSystem::new(noise, quantum);

    // Run the circuit multiple times and collect statistics
    let num_shots = 1000;
    let mut counts = HashMap::new();

    for _ in 0..num_shots {
        system.reset().expect("Failed to reset");
        let results = system
            .process_as_system(circ.clone())
            .expect("Failed to process circuit");
        let measurements = results
            .parse_measurements()
            .expect("Failed to parse measurements");

        let result = measurements
            .first()
            .map_or("?", |&(_, value)| if value == 1 { "1" } else { "0" });
        *counts.entry(result.to_string()).or_insert(0) += 1;
    }

    // Calculate percentages
    let pct_0 = counts.get("0").unwrap_or(&0) * 100 / num_shots;
    let pct_1 = counts.get("1").unwrap_or(&0) * 100 / num_shots;

    // Calculate expected results for this configuration
    let p_flip_0 = 0.4;
    let p_flip_1 = 0.1;
    let expected_0 = ((50.0 * (1.0 - p_flip_0) + 50.0 * p_flip_1) as f64).round() as usize;
    let expected_1 = ((50.0 * p_flip_0 + 50.0 * (1.0 - p_flip_1)) as f64).round() as usize;

    println!("Builder pattern with p_flip_0 = 0.4, p_flip_1 = 0.1");
    println!("  Expected results: 0: {expected_0}%, 1: {expected_1}%");
    println!("  Actual results:   0: {pct_0}%, 1: {pct_1}%");
    println!();
}

fn example3_bell_state() {
    // === EXAMPLE 3: Bell state with biased measurement ===
    println!("Example 3: Bell state with biased measurement");

    // Create a Bell state circuit
    let bell_circ = ByteMessage::quantum_operations_builder()
        .add_h(&[0])
        .add_cx(&[0], &[1])
        .add_measurements(&[0], &[0])
        .add_measurements(&[1], &[1])
        .build();

    // Create a new quantum system with 2 qubits
    let quantum2 = Box::new(StateVecEngine::new(2));
    let noise2 = Box::new(BiasedMeasurementNoise::new(0.2, 0.3));
    let mut system2 = QuantumSystem::new(noise2, quantum2);

    // Set a fixed seed for deterministic results
    system2.set_seed(42).expect("Failed to set seed");

    // Run the bell circuit multiple times and collect statistics
    let num_shots = 1000;
    let mut bell_counts = HashMap::new();

    for _ in 0..num_shots {
        system2.reset().expect("Failed to reset");
        let results = system2
            .process_as_system(bell_circ.clone())
            .expect("Failed to process Bell circuit");
        let measurements = results
            .parse_measurements()
            .expect("Failed to parse measurements");

        // Combine the measurement results into a string
        let mut result = String::new();
        for &(_, value) in &measurements {
            result.push(if value == 1 { '1' } else { '0' });
        }

        *bell_counts.entry(result).or_insert(0) += 1;
    }

    println!("Bell state with biased measurement noise:");
    println!("  p_flip_0 = 0.2, p_flip_1 = 0.3");

    // Calculate expected probabilities for Bell state results with biased measurement
    // For Bell state (|00⟩ + |11⟩)/√2, with no bias we expect 50% |00⟩ and 50% |11⟩
    let p_flip_0 = 0.2;
    let p_flip_1 = 0.3;

    // Calculate theoretical distributions
    let mut expected_probs = HashMap::new();
    expected_probs.insert(
        "00".to_string(),
        ((1.0 - p_flip_0) * (1.0 - p_flip_0) + p_flip_1 * p_flip_1) * 50.0,
    );
    expected_probs.insert(
        "01".to_string(),
        ((1.0 - p_flip_0) * p_flip_0 + p_flip_1 * (1.0 - p_flip_1)) * 50.0,
    );
    expected_probs.insert(
        "10".to_string(),
        (p_flip_0 * (1.0 - p_flip_0) + (1.0 - p_flip_1) * p_flip_1) * 50.0,
    );
    expected_probs.insert(
        "11".to_string(),
        (p_flip_0 * p_flip_0 + (1.0 - p_flip_1) * (1.0 - p_flip_1)) * 50.0,
    );

    // Sort both expected and actual by the pattern (00, 01, 10, 11) for easier comparison
    let patterns = ["00", "01", "10", "11"];

    println!("{:-^60}", "");
    println!("{:<10} | {:<20} | {:<20}", "Pattern", "Expected", "Actual");
    println!("{:-^60}", "");

    for pattern in &patterns {
        let expected = expected_probs.get(*pattern).unwrap_or(&0.0);
        let actual_count = bell_counts.get(*pattern).unwrap_or(&0);
        let actual_pct = actual_count * 100 / num_shots;

        println!(
            "{:<10} | {:<20} | {:<20}",
            pattern,
            format!("{:.2}%", expected),
            format!("{} ({}%)", actual_count, actual_pct)
        );
    }
    println!("{:-^60}", "");
}
