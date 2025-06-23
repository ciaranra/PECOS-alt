use pecos_qasm::QASMProgram;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example QASM code
    let qasm_str = r#"
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
    "#;

    // Pattern 1: Direct FromStr usage with type annotation
    let program1: QASMProgram = qasm_str.parse()?;
    println!("Program 1 has {} qubits", program1.num_qubits());

    // Pattern 2: Explicit FromStr call
    let program2 = QASMProgram::from_str(qasm_str)?;
    println!("Program 2 has {} qubits", program2.num_qubits());

    // Pattern 3: Using the parse() method with turbofish syntax
    let program3 = qasm_str.parse::<QASMProgram>()?;
    println!("Program 3 has {} qubits", program3.num_qubits());

    // Pattern 4: Using the ? operator for error propagation
    let result = qasm_str.parse::<QASMProgram>();
    let program4 = result?;
    println!("Program 4 has {} qubits", program4.num_qubits());

    // Pattern 5: String methods and iteration
    let lines = qasm_str.lines().collect::<Vec<_>>();
    let joined = lines.join("\n");
    let program5 = QASMProgram::from_str(&joined)?;
    println!("Program 5 has {} qubits", program5.num_qubits());

    // Pattern 6: Use with standard library methods expecting FromStr
    let program_vec = vec![qasm_str, qasm_str, qasm_str]
        .into_iter()
        .map(str::parse::<QASMProgram>)
        .collect::<Result<Vec<_>, _>>()?;

    println!("Parsed {} programs", program_vec.len());

    Ok(())
}
