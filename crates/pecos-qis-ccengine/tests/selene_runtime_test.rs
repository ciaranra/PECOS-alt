//! Tests for SeleneRuntime integration

use pecos_qis_ccengine::{SeleneRuntime, QisControlEngine, QisRuntime, QisJitInterface};
use pecos_qis_interface::QisInterface;

#[test]
fn test_selene_runtime_creation() {
    // Create a SeleneRuntime with a path to a (potentially non-existent) .so
    let runtime = SeleneRuntime::new("/tmp/test_selene.so");

    // Should be able to create the runtime even if the .so doesn't exist yet
    assert_eq!(runtime.num_qubits(), 0);
    assert!(runtime.is_complete());
}

#[test]
fn test_selene_runtime_with_control_engine() {
    // Create a SeleneRuntime
    let runtime = Box::new(SeleneRuntime::new("/tmp/test_selene.so"));

    // Create a JIT interface for testing
    let interface = Box::new(QisJitInterface::new());

    // Create the control engine with both interface and runtime
    let mut engine = QisControlEngine::new(interface, runtime);

    // Create another interface to set on the engine
    let new_interface = Box::new(QisJitInterface::new());

    // Should be able to set the interface
    // (actual plugin loading is deferred until needed)
    engine.set_interface(new_interface);
}

#[test]
#[ignore] // Ignore by default since it needs an actual .so file
fn test_selene_runtime_with_real_plugin() {
    // This test would require an actual Selene plugin .so file
    // It's here as a template for when we have a real plugin to test with

    let plugin_path = std::env::var("SELENE_PLUGIN_PATH")
        .unwrap_or_else(|_| "/path/to/selene_plugin.so".to_string());

    let mut runtime = SeleneRuntime::new(&plugin_path);

    // Create a simple program
    let mut interface = QisInterface::new();
    let q0 = interface.allocate_qubit();
    interface.queue_operation(pecos_qis_interface::QuantumOp::H(q0).into());

    // Load and execute
    runtime.load_interface(interface).unwrap();

    // Try to get operations
    match runtime.execute_until_quantum() {
        Ok(Some(ops)) => {
            println!("Got {} operations from Selene runtime", ops.len());
            assert!(!ops.is_empty());
        }
        Ok(None) => {
            println!("No operations from Selene runtime");
        }
        Err(e) => {
            // Expected if plugin doesn't exist
            println!("Failed to execute with Selene runtime: {:?}", e);
        }
    }
}