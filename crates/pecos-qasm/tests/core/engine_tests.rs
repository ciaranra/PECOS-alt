use pecos_qasm::prelude::*;

#[test]
fn test_program_source_getter() -> Result<(), PecosError> {
    // Define test QASM code
    let qasm_source = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    // Create QASMProgram from source
    let program = QASMProgram::from_str(qasm_source)?;

    // Check that the source getter returns the original source code
    assert_eq!(program.source(), qasm_source);

    Ok(())
}
