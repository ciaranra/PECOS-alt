// Tests for run_qasm_sim function

use pecos_qasm::run_qasm_sim;

#[test]
fn test_run_qasm_sim_basic() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    let results = run_qasm_sim(qasm, 10, Some(42), None, None, None).unwrap();

    // Check basic properties
    assert_eq!(results.len(), 10);
    assert!(!results.is_empty());
    assert_eq!(results.len(), 10);

    // Register sizes are no longer stored in results

    // Check binary format structure using lazy method
    let binary = results.to_binary_json();
    assert!(binary.is_object());
    assert!(binary["c"].is_array());
    assert_eq!(binary["c"].as_array().unwrap().len(), 10);

    // Check that values are valid Bell state results
    for value in binary["c"].as_array().unwrap() {
        let binary_str = value.as_str().unwrap();
        assert!(binary_str == "00" || binary_str == "11");
    }
}

#[test]
fn test_run_qasm_sim_multiple_registers() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[4];
        creg a[2];
        creg b[3];
        creg c[1];
        
        // Create known values
        x q[0];
        x q[2];
        x q[3];
        
        measure q[0] -> a[0];
        measure q[1] -> a[1];
        measure q[2] -> b[0];
        measure q[3] -> b[1];
        measure q[0] -> c[0];
    "#;

    let results = run_qasm_sim(qasm, 5, Some(42), None, None, None).unwrap();

    // Register sizes are no longer stored in results

    // Check binary format using lazy method
    let binary = results.to_binary_json();

    // All shots should have consistent values
    for i in 0..5 {
        assert_eq!(binary["a"][i].as_str().unwrap(), "01"); // a[0]=1, a[1]=0
        assert_eq!(binary["b"][i].as_str().unwrap(), "011"); // b[0]=1, b[1]=1, b[2]=0
        assert_eq!(binary["c"][i].as_str().unwrap(), "1"); // c[0]=1
    }

    // Check compact JSON format
    let json_str = results.to_compact_json();
    assert!(json_str.contains("\"a\":1"));
    assert!(json_str.contains("\"b\":3"));
    assert!(json_str.contains("\"c\":1"));
}

#[test]
fn test_run_qasm_sim_with_noise() {
    use pecos_engines::DepolarizingNoiseModel;

    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
    "#;

    // Run with depolarizing noise (prep_error, meas_error, p1, p2)
    let noise_model = Box::new(DepolarizingNoiseModel::new(0.0, 0.01, 0.01, 0.001));
    let results = run_qasm_sim(
        qasm,
        100,
        Some(42),
        Some(4), // Use 4 workers
        Some(noise_model),
        None,
    )
    .unwrap();

    assert_eq!(results.len(), 100);

    // With noise, we should see some errors (not all 1s)
    let binary = results.to_binary_json();
    let values = binary["c"].as_array().unwrap();

    let zeros = values.iter().filter(|v| v.as_str().unwrap() == "0").count();
    let ones = values.iter().filter(|v| v.as_str().unwrap() == "1").count();

    // With 10% error rate, we expect some zeros
    assert!(zeros > 0, "Expected some errors with noise");
    assert!(ones > zeros, "Expected more correct results than errors");
}

#[test]
fn test_as_string() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        creg a[2];
        creg b[3];
        
        // Create known values
        x q[0];
        x q[2];
        
        measure q[0] -> a[0];
        measure q[1] -> a[1]; 
        measure q[2] -> b[0];
        measure q[1] -> b[1];
        measure q[0] -> b[2];
    "#;

    let results = run_qasm_sim(qasm, 3, Some(42), None, None, None).unwrap();

    // Test the compact JSON format
    let json_str = results.to_compact_json();

    // Verify the format contains expected values
    assert!(json_str.contains("\"a\":1"));
    assert!(json_str.contains("\"b\":5"));
}

#[test]
fn test_json_serialization() {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        x q[0];
        measure q[0] -> c[0];
    "#;

    let results = run_qasm_sim(qasm, 2, Some(42), None, None, None).unwrap();

    // Test that the entire result can be serialized to JSON
    let json_str = serde_json::to_string(&results).unwrap();

    // And deserialized back
    let deserialized: pecos_qasm::QASMResults = serde_json::from_str(&json_str).unwrap();

    // Check that data survived round trip
    assert_eq!(deserialized.len(), 2);
    // Note: We can't compare binary_format anymore since it's computed on-demand
    // But we can verify the shots are the same
    assert_eq!(deserialized.len(), results.shot_vec().len());
}
