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

    let shot_vec = run_qasm_sim(qasm, 10, Some(42), None, None, None).unwrap();

    // Check basic properties
    assert_eq!(shot_vec.len(), 10);
    assert!(!shot_vec.is_empty());

    // Convert to ShotMap for analysis
    let shot_map = shot_vec.try_as_shot_map().unwrap();

    // Check register exists
    assert!(shot_map.get("c").is_some());

    // Get measurements as binary strings
    let measurements = shot_map.try_bits_as_binary("c").unwrap();
    assert_eq!(measurements.len(), 10);

    // Check that values are valid Bell state results
    for binary_str in &measurements {
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

    let shot_vec = run_qasm_sim(qasm, 5, Some(42), None, None, None).unwrap();
    let shot_map = shot_vec.try_as_shot_map().unwrap();

    // Check binary format
    let a_binary = shot_map.try_bits_as_binary("a").unwrap();
    let b_binary = shot_map.try_bits_as_binary("b").unwrap();
    let c_binary = shot_map.try_bits_as_binary("c").unwrap();

    // All shots should have consistent values
    for i in 0..5 {
        assert_eq!(a_binary[i], "01"); // a[0]=1, a[1]=0
        assert_eq!(b_binary[i], "011"); // b[0]=1, b[1]=1, b[2]=0
        assert_eq!(c_binary[i], "1"); // c[0]=1
    }

    // Check decimal values
    let a_decimal = shot_map.try_bits_as_decimal("a").unwrap();
    let b_decimal = shot_map.try_bits_as_decimal("b").unwrap();
    let c_decimal = shot_map.try_bits_as_decimal("c").unwrap();

    for i in 0..5 {
        assert_eq!(a_decimal[i], "1"); // binary "01" = decimal 1
        assert_eq!(b_decimal[i], "3"); // binary "011" = decimal 3
        assert_eq!(c_decimal[i], "1"); // binary "1" = decimal 1
    }
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
    let shot_vec = run_qasm_sim(
        qasm,
        100,
        Some(42),
        Some(4), // Use 4 workers
        Some(noise_model),
        None,
    )
    .unwrap();

    assert_eq!(shot_vec.len(), 100);

    // Convert to ShotMap for analysis
    let shot_map = shot_vec.try_as_shot_map().unwrap();

    // With noise, we should see some errors (not all 1s)
    let binary_values = shot_map.try_bits_as_binary("c").unwrap();

    let zeros = binary_values.iter().filter(|v| *v == "0").count();
    let ones = binary_values.iter().filter(|v| *v == "1").count();

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

    let shot_vec = run_qasm_sim(qasm, 3, Some(42), None, None, None).unwrap();
    let shot_map = shot_vec.try_as_shot_map().unwrap();

    // Verify decimal values
    let a_decimal = shot_map.try_bits_as_decimal("a").unwrap();
    let b_decimal = shot_map.try_bits_as_decimal("b").unwrap();

    // All shots should have the same values
    for i in 0..3 {
        assert_eq!(a_decimal[i], "1"); // binary "01" = decimal 1
        assert_eq!(b_decimal[i], "5"); // binary "101" = decimal 5
    }
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

    let shot_vec = run_qasm_sim(qasm, 2, Some(42), None, None, None).unwrap();

    // Test that the ShotVec can be serialized to JSON
    let json_str = serde_json::to_string(&shot_vec).unwrap();

    // And deserialized back
    let deserialized: pecos_engines::ShotVec = serde_json::from_str(&json_str).unwrap();

    // Check that data survived round trip
    assert_eq!(deserialized.len(), 2);
    assert_eq!(deserialized.len(), shot_vec.len());

    // Verify the contents are the same
    let original_map = shot_vec.try_as_shot_map().unwrap();
    let deserialized_map = deserialized.try_as_shot_map().unwrap();

    let original_c = original_map.try_bits_as_binary("c").unwrap();
    let deserialized_c = deserialized_map.try_bits_as_binary("c").unwrap();

    assert_eq!(original_c, deserialized_c);
}
