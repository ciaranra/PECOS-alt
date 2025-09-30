//! Test the program abstraction for QisControlEngine

use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
use pecos_programs::{QisProgram, HugrProgram};
use pecos_qis_ccengine::{qis_control_engine, native_runtime};
use pecos_qis_interface::{QisInterface, QuantumOp};

#[test]
fn test_qis_interface_program() {
    // Create a simple Bell state program
    let mut interface = QisInterface::new();
    let q0 = interface.allocate_qubit();
    let q1 = interface.allocate_qubit();
    interface.queue_operation(QuantumOp::H(q0).into());
    interface.queue_operation(QuantumOp::CX(q0, q1).into());

    // Test that we can use QisInterface directly with .program()
    let engine = qis_control_engine()
        .runtime(native_runtime())
        .program(interface)
        .build()
        .expect("Failed to build engine");

    assert_eq!(engine.num_qubits(), 2);
}

#[test]
fn test_qis_program_conversion_success() {
    // Test that QisProgram conversion now works
    let qis_program = QisProgram::from_string("define void @main() { ret void }");

    let result = qis_control_engine()
        .runtime(native_runtime())
        .try_program(qis_program);

    assert!(result.is_ok(), "QisProgram conversion should now work");

    // Test that we can build the engine with the converted program
    let engine = result.unwrap().build();
    assert!(engine.is_ok());
}

#[test]
fn test_hugr_program_conversion_error() {
    // Test that HugrProgram conversion fails appropriately when given invalid data
    let hugr_program = HugrProgram::from_bytes(vec![1, 2, 3, 4]); // Invalid HUGR data

    let result = qis_control_engine()
        .runtime(native_runtime())
        .try_program(hugr_program);

    // Should fail during HUGR compilation step
    assert!(result.is_err());
}

#[test]
fn test_builder_api_consistency() {
    // Test that the API looks similar to other engines
    let interface = QisInterface::new();

    // The fluent API should work
    let _builder = qis_control_engine()
        .runtime(native_runtime())
        .program(interface);

    // Multiple calls should work
    let _builder2 = qis_control_engine()
        .program(QisInterface::new())
        .runtime(native_runtime());
}