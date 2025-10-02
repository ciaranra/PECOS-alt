//! Debug QIS flow to understand the execution

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime};
use pecos_qis_interface::{QisInterface, QuantumOp};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Debug QIS Flow ===\n");

    // Create a simple Bell state program directly with QisInterface
    let mut interface = QisInterface::new();
    let q0 = interface.allocate_qubit();
    let q1 = interface.allocate_qubit();
    let r0 = interface.allocate_result();
    let r1 = interface.allocate_result();

    interface.queue_operation(QuantumOp::H(q0).into());
    interface.queue_operation(QuantumOp::CX(q0, q1).into());
    interface.queue_operation(QuantumOp::Measure(q0, r0).into());
    interface.queue_operation(QuantumOp::Measure(q1, r1).into());

    println!("Created interface with:");
    println!("  - {} qubits", interface.allocated_qubits.len());
    println!("  - {} results", interface.allocated_results.len());
    println!("  - {} operations", interface.operations.len());
    for (i, op) in interface.operations.iter().enumerate() {
        println!("    {}: {:?}", i, op);
    }

    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .program(interface)
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(20)?;

    println!("\nResults:");
    let mut count_00 = 0;
    let mut count_01 = 0;
    let mut count_10 = 0;
    let mut count_11 = 0;

    for (i, shot) in results.shots.iter().enumerate() {
        let m0 = shot.data.get("m0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(99);
        let m1 = shot.data.get("m1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(99);

        println!("  Shot {}: m0={}, m1={}", i, m0, m1);

        match (m0, m1) {
            (0, 0) => count_00 += 1,
            (0, 1) => count_01 += 1,
            (1, 0) => count_10 += 1,
            (1, 1) => count_11 += 1,
            _ => println!("    Unexpected: ({}, {})", m0, m1),
        }
    }

    println!("\nCounts:");
    println!("  |00⟩: {}", count_00);
    println!("  |01⟩: {}", count_01);
    println!("  |10⟩: {}", count_10);
    println!("  |11⟩: {}", count_11);

    if count_00 > 0 && count_11 > 0 && count_01 == 0 && count_10 == 0 {
        println!("\nSUCCESS: Bell state working correctly!");
    } else {
        println!("\nFAILED: Bell state not working as expected");
    }

    Ok(())
}