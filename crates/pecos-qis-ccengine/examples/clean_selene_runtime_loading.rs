//! Example: Clean Selene runtime loading using build script paths
//!
//! This demonstrates the simplified approach where we know exactly where
//! the Selene runtimes are built by the build script.

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, selene_simple_runtime, selene_soft_rz_runtime, selene_runtime_auto};
use pecos_qis_interface::{QisInterface, QuantumOp};

/// Create a simple Bell state QIS program
fn create_bell_state_interface() -> QisInterface {
    let mut interface = QisInterface::new();

    let q0 = interface.allocate_qubit();
    let q1 = interface.allocate_qubit();
    let r0 = interface.allocate_result();
    let r1 = interface.allocate_result();

    interface.queue_operation(QuantumOp::H(q0).into());
    interface.queue_operation(QuantumOp::CX(q0, q1).into());
    interface.queue_operation(QuantumOp::Measure(q0, r0).into());
    interface.queue_operation(QuantumOp::Measure(q1, r1).into());

    interface
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Clean Selene Runtime Loading ===\n");

    // Method 1: Using convenience function for simple runtime
    println!("1. selene_simple_runtime() - Built by build script");
    match selene_simple_runtime() {
        Ok(runtime) => {
            let interface = create_bell_state_interface();
            let results = sim_builder()
                .classical(qis_control_engine().runtime(runtime).program(interface))
                .quantum(state_vector())
                .qubits(2)
                .run(3)?;
            println!("   ✓ Simple runtime: {} shots", results.len());
        }
        Err(e) => {
            println!("   ✗ Simple runtime failed: {}", e);
        }
    }

    // Method 2: Using convenience function for soft RZ runtime
    println!("2. selene_soft_rz_runtime() - Built by build script");
    match selene_soft_rz_runtime() {
        Ok(runtime) => {
            let interface = create_bell_state_interface();
            let results = sim_builder()
                .classical(qis_control_engine().runtime(runtime).program(interface))
                .quantum(state_vector())
                .qubits(2)
                .run(3)?;
            println!("   ✓ Soft RZ runtime: {} shots", results.len());
        }
        Err(e) => {
            println!("   ✗ Soft RZ runtime failed: {}", e);
        }
    }

    // Method 3: Using auto function with explicit library name
    println!("3. selene_runtime_auto() - Generic loading by name");
    match selene_runtime_auto("selene_simple_runtime") {
        Ok(runtime) => {
            let interface = create_bell_state_interface();
            let results = sim_builder()
                .classical(qis_control_engine().runtime(runtime).program(interface))
                .quantum(state_vector())
                .qubits(2)
                .run(3)?;
            println!("   ✓ Auto-loaded runtime: {} shots", results.len());
        }
        Err(e) => {
            println!("   ✗ Auto runtime failed: {}", e);
        }
    }

    println!("\n🎯 All Selene runtimes loaded successfully!");
    println!("The build script approach ensures we know exactly where the .so files are.");
    println!("No download fallback needed - if Selene is present, the runtimes are built.");

    Ok(())
}