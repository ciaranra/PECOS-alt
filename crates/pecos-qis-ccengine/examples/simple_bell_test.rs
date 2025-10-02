//! Simplest possible Bell state test

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime};
use pecos_qis_interface::{QisInterface, QuantumOp};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Bell State Test ===\n");

    // Test 1: Using Native Runtime for real Bell state simulation
    println!("1. Native Runtime (real Bell state):");
    let mut interface1 = QisInterface::new();
    let q0 = interface1.allocate_qubit();
    let q1 = interface1.allocate_qubit();
    let r0 = interface1.allocate_result();
    let r1 = interface1.allocate_result();

    interface1.queue_operation(QuantumOp::H(q0).into());
    interface1.queue_operation(QuantumOp::CX(q0, q1).into());
    interface1.queue_operation(QuantumOp::Measure(q0, r0).into());
    interface1.queue_operation(QuantumOp::Measure(q1, r1).into());

    let native_results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .program(interface1)
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)  // For reproducible results
        .run(10)?;

    println!("  Bell state results over 10 shots:");
    let mut count_00 = 0;
    let mut count_11 = 0;
    for shot in &native_results.shots {
        let m0 = shot.data.get("result_0").map(|v| format!("{:?}", v)).unwrap_or("None".to_string());
        let m1 = shot.data.get("result_1").map(|v| format!("{:?}", v)).unwrap_or("None".to_string());
        println!("    Shot: q0={}, q1={}", m0, m1);

        if m0 == "U32(0)" && m1 == "U32(0)" { count_00 += 1; }
        if m0 == "U32(1)" && m1 == "U32(1)" { count_11 += 1; }
    }
    println!("  Summary: |00⟩ occurred {} times, |11⟩ occurred {} times", count_00, count_11);

    // Test 2: Direct QIS interface without measurements
    println!("\n2. Direct QisInterface (no measurements):");
    let mut interface2 = QisInterface::new();
    let q0 = interface2.allocate_qubit();
    let q1 = interface2.allocate_qubit();

    interface2.queue_operation(QuantumOp::H(q0).into());
    interface2.queue_operation(QuantumOp::CX(q0, q1).into());

    // Run without measurements using native runtime
    let no_measure_results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .program(interface2)
        )
        .quantum(state_vector())
        .qubits(2)
        .run(1)?;

    println!("  No measurements shot data: {:?}", no_measure_results.shots[0].data);

    Ok(())
}