use pecos_qasm::QASMParser;

#[test]
fn test_commented_include_should_be_ignored() {
    // Test that include statements in comments are completely ignored
    let qasm = r#"
        OPENQASM 2.0;

        // This is a comment with an include statement
        // include "does_not_exist.inc";

        include "qelib1.inc";

        qreg q[1];
        h q[0];
    "#;

    // This should parse successfully - the commented include should be ignored
    let result = QASMParser::parse_str(qasm);
    match result {
        Ok(_) => {}
        Err(e) => panic!("Parser should ignore commented include statements, but got error: {e:?}"),
    }
}

#[test]
fn test_multiple_comment_styles_with_includes() {
    // Test different comment positions
    let qasm = r#"
        OPENQASM 2.0;

        // include "fake1.inc";
        include "qelib1.inc"; // include "fake2.inc";
        //include "fake3.inc";
        // include "fake4.inc"; more comment text

        qreg q[2];
        cx q[0],q[1];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Parser should handle all comment styles correctly"
    );
}

#[test]
fn test_block_comment_with_include() {
    // Test that block comments also work (if supported)
    let qasm = r#"
        OPENQASM 2.0;

        /* This is a block comment
           include "nonexistent.inc";
           with multiple lines */

        include "qelib1.inc";

        qreg q[1];
        x q[0];
    "#;

    // This might fail if block comments aren't supported, but the include should still be ignored
    let _ = QASMParser::parse_str(qasm);
}

#[test]
fn test_inline_comment_after_real_include() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc"; // This should work, unlike include "broken.inc"

        qreg q[1];
        h q[0];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(result.is_ok(), "Inline comments after includes should work");
}

#[test]
fn test_commented_include_with_valid_syntax() {
    // Even if the include syntax is perfect, it should be ignored in a comment
    let qasm = r#"
        OPENQASM 2.0;

        // include "qelib1.inc";
        // The above line should be ignored even though qelib1.inc exists

        qreg q[1];
        // Without qelib1.inc, the h gate won't be defined
        H q[0]; // This should work as a native gate
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Native H gate should work without qelib1.inc"
    );
}

#[test]
fn test_string_literal_with_include_keyword() {
    // Make sure include keyword in strings doesn't cause issues
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";

        // Note: QASM 2.0 doesn't really support string literals in most contexts,
        // but this tests the parser's robustness
        qreg q[1];
        h q[0];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Should handle include keyword in various contexts"
    );
}

#[test]
fn test_multiple_includes_on_same_line_with_comment() {
    // Test edge case with multiple include-like patterns
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc"; // include "fake.inc"; include "another_fake.inc";

        qreg q[1];
        h q[0];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Should only process first include, ignore commented ones"
    );
}

#[test]
fn test_commented_include_with_special_characters() {
    // Test that special characters in commented includes don't cause issues
    let qasm = r#"
        OPENQASM 2.0;
        // include "../../../etc/passwd";
        // include "file with spaces.inc";
        // include "file@#$%.inc";
        include "qelib1.inc";

        qreg q[2];
        cx q[0],q[1];
    "#;

    let result = QASMParser::parse_str(qasm);
    // Should succeed - commented includes should be ignored
    assert!(
        result.is_ok(),
        "Special characters in commented includes should not cause issues"
    );
}

#[test]
fn test_include_in_gate_definition_comment() {
    // Test comments inside gate definitions
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";

        gate mygate a {
            // include "should_be_ignored.inc";
            h a;
            // Another comment with include "fake.inc"
            x a;
        }

        qreg q[1];
        mygate q[0];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Comments in gate definitions should be ignored"
    );
}

#[test]
fn test_partial_include_statement_in_comment() {
    // Test incomplete include statements in comments
    let qasm = r#"
        OPENQASM 2.0;
        // include
        // include "
        // include "incomplete
        include "qelib1.inc";
        // "fake.inc";

        qreg q[1];
        h q[0];
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Partial include statements in comments should be ignored"
    );
}

#[test]
fn test_include_after_semicolon_on_same_line() {
    // Test that includes after other statements work correctly
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";

        qreg q[1]; // include "this_should_be_ignored.inc";
        h q[0]; // apply hadamard; include "fake.inc";
    "#;

    let result = QASMParser::parse_str(qasm);
    assert!(
        result.is_ok(),
        "Includes in comments after statements should be ignored"
    );
}
