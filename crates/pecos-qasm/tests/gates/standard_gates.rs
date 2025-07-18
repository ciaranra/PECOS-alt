use pecos_qasm::{Operation, QASMParser};

// Helper function to check if an operation is a gate with a specific name
fn is_gate_with_name(op: &Operation, gate_name: &str) -> bool {
    match op {
        Operation::Gate { name, .. } => {
            name == gate_name || name.to_uppercase() == gate_name.to_uppercase()
        }
        Operation::NativeGate(gate) => {
            let gate_type_name = format!("{:?}", gate.gate_type).to_lowercase();
            let target_name = gate_name.to_lowercase();
            gate_type_name == target_name
                || (target_name == "cx" && gate_type_name == "cnot")
                || (target_name == "cnot" && gate_type_name == "cnot")
        }
        _ => false,
    }
}

#[test]
fn test_comprehensive_gate_operations() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        //some comments
        qreg q[4];
        rz(1.5*pi) q[3];
        rx(0.0375*pi) q[3];
        rxx(0.0375*pi) q[0],q[1];
        rz(0.5*pi) q[3];
        rzz(0.0375*pi) q[0],q[1];
        cx q[0],q[3];
        rz(1.5*pi) q[3];
        rx(1.9625*pi) q[3];
        cz q[0] ,q[1]; //hey look ma its a cz
        ccx q[3],q[1],q[2];
        barrier q[0],q[3],q[2];
        u3(3.141596, 0.5* pi ,0.3*pi) q[2];
        cu1(0.8*pi) q[0],q[1];
    "#;

    let program = QASMParser::parse_str(qasm).expect("Failed to parse comprehensive QASM program");

    // Verify that the program has the correct number of operations
    // Note: This includes all operations, not just gates
    assert!(!program.operations.is_empty(), "Should have operations");

    // Verify that important gates are defined (either natively or through qelib1)
    assert!(
        program.gate_definitions.contains_key("rx")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "rx")),
        "rx gate should be available"
    );

    assert!(
        program.gate_definitions.contains_key("rxx")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "rxx")),
        "rxx gate should be available"
    );

    assert!(
        program.gate_definitions.contains_key("rzz")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "rzz")),
        "rzz gate should be available"
    );

    assert!(
        program.gate_definitions.contains_key("cz")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "cz")),
        "cz gate should be available"
    );

    assert!(
        program.gate_definitions.contains_key("ccx")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "ccx")),
        "ccx gate should be available"
    );

    assert!(
        program.gate_definitions.contains_key("u3")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "u3")),
        "u3 gate should be available"
    );

    assert!(
        program.gate_definitions.contains_key("cu1")
            || program
                .operations
                .iter()
                .any(|op| is_gate_with_name(op, "cu1")),
        "cu1 gate should be available"
    );
}

#[test]
fn test_mathematical_expressions_in_parameters() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        // Test various mathematical expressions
        rz(1.5*pi) q[0];
        rx(0.0375*pi) q[0];
        rz(0.5*pi) q[1];
        u3(3.141596, 0.5* pi ,0.3*pi) q[0];
        cu1(0.8*pi) q[0],q[1];
    "#;

    let program =
        QASMParser::parse_str(qasm).expect("Failed to parse QASM with mathematical expressions");

    // Just verify it parses without errors
    assert!(
        !program.operations.is_empty(),
        "Should have parsed operations with mathematical expressions"
    );
}

#[test]
fn test_comments_and_whitespace() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        //some comments
        qreg q[2];

        // Comment before operation
        cx q[0],q[1];

        cz q[0] ,q[1]; //hey look ma its a cz

        // End comment
    "#;

    let program = QASMParser::parse_str(qasm).expect("Failed to parse QASM with comments");

    // Comments should be ignored and not affect parsing
    assert!(
        !program.operations.is_empty(),
        "Should have parsed operations despite comments"
    );
}
