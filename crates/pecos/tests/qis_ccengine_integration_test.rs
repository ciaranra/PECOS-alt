//! Integration test for QisControlEngine with the full sim() API
//!
//! Tests that our QIS architecture works correctly with Bell state programs

use pecos_engines::{sim_builder, sparse_stabilizer, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime, selene_simple_runtime};
use pecos_qis_interface::{QisInterface, QuantumOp};

/// Create a Bell state QIS program
fn create_bell_state_interface() -> QisInterface {
    let mut interface = QisInterface::new();

    // Allocate two qubits
    let q0 = interface.allocate_qubit();
    let q1 = interface.allocate_qubit();

    // Create Bell state: (|00⟩ + |11⟩)/√2
    // H on first qubit: |0⟩ → (|0⟩ + |1⟩)/√2
    interface.queue_operation(QuantumOp::H(q0).into());
    // CNOT: (|00⟩ + |10⟩)/√2 → (|00⟩ + |11⟩)/√2
    interface.queue_operation(QuantumOp::CX(q0, q1).into());

    // Add measurements
    let r0 = interface.allocate_result();
    let r1 = interface.allocate_result();
    interface.queue_operation(QuantumOp::Measure(q0, r0).into());
    interface.queue_operation(QuantumOp::Measure(q1, r1).into());

    interface
}

#[test]
fn test_qis_control_engine_with_native_runtime() {
    // Create a Bell state program
    let interface = create_bell_state_interface();

    // Build the QisControlEngine with native runtime
    let engine_builder = qis_control_engine()
        .runtime(native_runtime())
        .program(interface);

    // Run simulation using the sim() API
    let results = sim_builder()
        .classical(engine_builder)
        .quantum(sparse_stabilizer())
        .qubits(2)
        .run(100)
        .expect("Simulation failed");

    // Verify we get Bell state results (only |00⟩ and |11⟩)
    assert_eq!(results.len(), 100, "Should have 100 shots");

    let mut count_00 = 0;
    let mut count_11 = 0;

    for shot in &results.shots {
        // Extract measurement values from the shot
        // Look for measurement registers (m0, m1, etc.)
        let mut bits = Vec::new();
        for j in 0..2 {
            let key = format!("m{}", j);
            if let Some(data) = shot.data.get(&key) {
                match data {
                    pecos_engines::shot_results::Data::U32(val) => {
                        bits.push(*val);
                    }
                    pecos_engines::shot_results::Data::BitVec(bitvec) => {
                        // Extract the first bit as a u32 (0 or 1)
                        let bit_val = if !bitvec.is_empty() && bitvec[0] { 1u32 } else { 0u32 };
                        bits.push(bit_val);
                    }
                    _ => {
                        // Unexpected data type - skip
                    }
                }
            }
        }

        if bits.len() == 2 {
            if bits == vec![0, 0] {
                count_00 += 1;
            } else if bits == vec![1, 1] {
                count_11 += 1;
            } else {
                panic!("Invalid Bell state measurement: {:?}", bits);
            }
        }
    }

    println!("Native runtime Bell state results:");
    println!("  |00⟩: {} times", count_00);
    println!("  |11⟩: {} times", count_11);

    // Verify roughly 50/50 distribution (with tolerance)
    assert!(count_00 > 20 && count_00 < 80, "00 count out of expected range");
    assert!(count_11 > 20 && count_11 < 80, "11 count out of expected range");
    assert_eq!(count_00 + count_11, 100, "Total should be 100");
}

#[test]
fn test_qis_control_engine_with_state_vector() {
    // Create a Bell state program
    let interface = create_bell_state_interface();

    // Build the QisControlEngine (defaults to native runtime)
    let engine_builder = qis_control_engine()
        .program(interface);

    // Run simulation with state vector quantum engine
    let results = sim_builder()
        .classical(engine_builder)
        .quantum(state_vector())
        .qubits(2)
        .seed(42)  // Use fixed seed for reproducibility
        .run(100)
        .expect("Simulation failed");

    // Verify Bell state results
    for shot in &results.shots {
        let mut bits = Vec::new();
        for i in 0..2 {
            let key = format!("r{}", i);
            if let Some(data) = shot.data.get(&key) {
                if let pecos_engines::shot_results::Data::U32(val) = data {
                    bits.push(*val);
                }
            }
        }

        if bits.len() == 2 {
            assert!(
                bits == vec![0, 0] || bits == vec![1, 1],
                "Bell state should only produce |00⟩ or |11⟩, got: {:?}",
                bits
            );
        }
    }

    println!("State vector Bell state test passed!");
}

#[test]
#[ignore]  // Ignore by default since Selene runtime may not be available
fn test_qis_control_engine_with_selene_runtime() {
    // Try to load Selene simple runtime
    let runtime = match selene_simple_runtime() {
        Ok(r) => r,
        Err(e) => {
            println!("Selene runtime not available: {}, skipping test", e);
            return;
        }
    };

    // Create a Bell state program
    let interface = create_bell_state_interface();

    // Build the QisControlEngine with Selene runtime
    let engine_builder = qis_control_engine()
        .runtime(runtime)
        .program(interface);

    // Run simulation
    let results = sim_builder()
        .classical(engine_builder)
        .quantum(state_vector())
        .qubits(2)
        .run(10)
        .expect("Simulation failed");

    // Verify Bell state results
    assert_eq!(results.len(), 10, "Should have 10 shots");

    for shot in &results.shots {
        let mut bits = Vec::new();
        for i in 0..2 {
            let key = format!("r{}", i);
            if let Some(data) = shot.data.get(&key) {
                if let pecos_engines::shot_results::Data::U32(val) = data {
                    bits.push(*val);
                }
            }
        }

        if bits.len() == 2 {
            assert!(
                bits == vec![0, 0] || bits == vec![1, 1],
                "Bell state should only produce |00⟩ or |11⟩"
            );
        }
    }

    println!("Selene runtime Bell state test passed!");
}

#[test]
fn test_fluent_api_compiles() {
    // This test verifies that the fluent API syntax works as expected
    let _ = || {
        let interface = create_bell_state_interface();

        // The exact fluent API the user requested
        let _results = sim_builder()
            .classical(
                qis_control_engine()
                    .runtime(native_runtime())
                    .program(interface)
            )
            .quantum(state_vector())
            .qubits(2)
            .run(10);
    };
}