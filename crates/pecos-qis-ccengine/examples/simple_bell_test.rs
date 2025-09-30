//! Simplest possible Bell state test

use pecos_engines::{sim_builder, state_vector, ClassicalControlEngineBuilder};
use pecos_qis_ccengine::{qis_control_engine, MockRuntime, mock_all_ones_runtime};
use pecos_qis_interface::{QisInterface, QuantumOp};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Bell State Test ===\n");

    // Test 1: Using Mock Runtime with predetermined results
    println!("1. Mock Runtime (predetermined |11⟩):");
    let mut interface1 = QisInterface::new();
    let q0 = interface1.allocate_qubit();
    let q1 = interface1.allocate_qubit();
    let r0 = interface1.allocate_result();
    let r1 = interface1.allocate_result();

    interface1.queue_operation(QuantumOp::H(q0).into());
    interface1.queue_operation(QuantumOp::CX(q0, q1).into());
    interface1.queue_operation(QuantumOp::Measure(q0, r0).into());
    interface1.queue_operation(QuantumOp::Measure(q1, r1).into());

    let mock_results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(mock_all_ones_runtime())
                .program(interface1)
        )
        .quantum(state_vector())
        .qubits(2)
        .run(1)?;

    println!("  Mock shot data: {:?}", mock_results.shots[0].data);

    // Test 2: Direct QIS interface without parsing
    println!("\n2. Direct QisInterface (no parsing):");
    let mut interface2 = QisInterface::new();
    let q0 = interface2.allocate_qubit();
    let q1 = interface2.allocate_qubit();
    // Don't allocate results - just use measure without storing

    interface2.queue_operation(QuantumOp::H(q0).into());
    interface2.queue_operation(QuantumOp::CX(q0, q1).into());

    // Try without measurements first
    let no_measure_results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(MockRuntime::new().with_qubits(2))
                .program(interface2)
        )
        .quantum(state_vector())
        .qubits(2)
        .run(1)?;

    println!("  No measurements shot data: {:?}", no_measure_results.shots[0].data);

    Ok(())
}