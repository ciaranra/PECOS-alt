use pecos_qasm::QASMParser;

#[test]
fn test_all_standard_includes_available() {
    // Test that all three standard include files are available
    let test_cases = vec![
        ("qelib1.inc", "h q[0];"),             // h gate is defined in qelib1.inc
        ("pecos.inc", "h q[0];"),              // h gate is also in pecos.inc
        ("hqslib1.inc", "U1q(pi/2, 0) q[0];"), // U1q is specific to hqslib1.inc
    ];

    for (include_file, gate_call) in test_cases {
        let qasm = format!(
            r#"
            OPENQASM 2.0;
            include "{include_file}";
            qreg q[1];
            {gate_call}
            "#
        );

        let result = QASMParser::parse_str(&qasm);
        assert!(
            result.is_ok(),
            "Failed to use {}: {:?}",
            include_file,
            result.err()
        );
    }
}

#[test]
fn test_unknown_include_file_fails() {
    let qasm = r#"
        OPENQASM 2.0;
        include "nonexistent.inc";
        qreg q[1];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_new_include_file_automatically_available() {
    // This test demonstrates that any new .inc file added to includes/
    // will automatically be available without code changes

    // For now, we just verify the existing ones work
    let standard_includes = ["qelib1.inc", "pecos.inc", "hqslib1.inc"];

    for include in &standard_includes {
        let qasm = format!(
            r#"
            OPENQASM 2.0;
            include "{include}";
            qreg q[1];
            "#
        );

        let result = QASMParser::parse_str(&qasm);
        assert!(
            result.is_ok(),
            "{include} should be automatically available"
        );
    }
}
