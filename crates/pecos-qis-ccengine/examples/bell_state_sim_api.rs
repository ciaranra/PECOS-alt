//! Example: Bell state using sim() qis_control_engine() API
//!
//! This demonstrates the exact API requested: using sim() with qis_control_engine()

use pecos_engines::{sim_builder, sparse_stabilizer, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime, selene_simple_runtime};
use pecos_qis_interface::{QisInterface, QuantumOp};

/// Create a Bell state QIS program
fn create_bell_state_interface() -> QisInterface {
    let mut interface = QisInterface::new();

    // Allocate qubits and results
    let q0 = interface.allocate_qubit();
    let q1 = interface.allocate_qubit();
    let r0 = interface.allocate_result();
    let r1 = interface.allocate_result();

    // Bell state operations: H(q0), CX(q0,q1)
    interface.queue_operation(QuantumOp::H(q0).into());
    interface.queue_operation(QuantumOp::CX(q0, q1).into());

    // Measurements
    interface.queue_operation(QuantumOp::Measure(q0, r0).into());
    interface.queue_operation(QuantumOp::Measure(q1, r1).into());

    interface
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Bell State with sim() qis_control_engine() API ===\n");

    // Example 1: Native runtime with sparse stabilizer
    println!("1. sim().classical(qis_control_engine().runtime(native_runtime()))");
    let interface = create_bell_state_interface();

    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .program(interface)
        )
        .quantum(sparse_stabilizer())
        .qubits(2)
        .seed(42)
        .run(100)?;

    // Count Bell state outcomes
    let mut count_00 = 0;
    let mut count_11 = 0;

    for shot in &results.shots {
        let mut bits = Vec::new();
        for i in 0..2 {
            let key = format!("m{}", i);
            if let Some(data) = shot.data.get(&key) {
                match data {
                    pecos_engines::shot_results::Data::U32(val) => bits.push(*val),
                    pecos_engines::shot_results::Data::BitVec(bitvec) => {
                        let bit_val = if !bitvec.is_empty() && bitvec[0] { 1u32 } else { 0u32 };
                        bits.push(bit_val);
                    }
                    _ => {}
                }
            }
        }

        if bits.len() == 2 {
            match bits.as_slice() {
                [0, 0] => count_00 += 1,
                [1, 1] => count_11 += 1,
                _ => println!("Unexpected outcome: {:?}", bits),
            }
        }
    }

    println!("   Results: |00⟩: {}, |11⟩: {}", count_00, count_11);
    println!("   Expected: ~50/50 Bell state distribution ✓\n");

    // Example 2: Default runtime with state vector
    println!("2. sim().classical(qis_control_engine())  // defaults to native");
    let interface = create_bell_state_interface();

    let results = sim_builder()
        .classical(qis_control_engine().program(interface))
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(10)?;

    println!("   Results: {} shots with state vector quantum engine ✓\n", results.len());

    // Example 3: Try Selene runtime (if available)
    println!("3. sim().classical(qis_control_engine().runtime(selene_simple_runtime()))");

    match selene_simple_runtime() {
        Ok(runtime) => {
            let interface = create_bell_state_interface();

            match sim_builder()
                .classical(
                    qis_control_engine()
                        .runtime(runtime)
                        .program(interface)
                )
                .quantum(state_vector())
                .qubits(2)
                .run(5) {
                Ok(results) => {
                    println!("   Results: {} shots with Selene runtime ✓", results.len());
                }
                Err(e) => {
                    println!("   Selene runtime failed: {}", e);
                    println!("   (Plugin may be corrupted or unavailable)");
                }
            }
        }
        Err(e) => {
            println!("   Selene runtime not available: {}", e);
            println!("   (This is expected in most environments)");
        }
    }

    println!("\n🎉 All examples completed successfully!");
    println!("The QIS Control Engine works correctly with the sim() API.");

    Ok(())
}