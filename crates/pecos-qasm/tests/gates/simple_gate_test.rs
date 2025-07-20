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
fn test_simple_gates() {
    // Test simple circuit with cx and u gates
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";

        qreg q[3];
        cx q[0],q[1];
        u(0, 0, 1*pi) q[0];
        cz q[1],q[2];  // This should expand to h-cx-h
    "#;

    let result = QASMParser::parse_str(qasm);

    match result {
        Ok(program) => {
            println!("Operations:");
            for (i, op) in program.operations.iter().enumerate() {
                match op {
                    Operation::Gate {
                        name,
                        qubits,
                        parameters,
                    } => {
                        println!(
                            "  [{i}] Gate: {name} on qubits {qubits:?} with params {parameters:?}"
                        );
                    }
                    Operation::NativeGate(gate) => {
                        println!(
                            "  [{i}] NativeGate: {:?} on qubits {:?} with params {:?}",
                            gate.gate_type, gate.qubits, gate.params
                        );
                    }
                    _ => {}
                }
            }

            let cx_count = program
                .operations
                .iter()
                .filter(|op| is_gate_with_name(op, "cx"))
                .count();

            let u_count = program
                .operations
                .iter()
                .filter(|op| is_gate_with_name(op, "u"))
                .count();

            println!("CX count: {cx_count}, U count: {u_count}");

            // We expect 2 cx (1 original + 1 from cz expansion) and 1 u gate
            assert_eq!(
                cx_count, 2,
                "Expected 2 CX gates (1 original + 1 from cz expansion)"
            );
            assert_eq!(u_count, 1, "Expected 1 U gate");
        }
        Err(e) => {
            panic!("Failed to parse circuit: {e}");
        }
    }
}
