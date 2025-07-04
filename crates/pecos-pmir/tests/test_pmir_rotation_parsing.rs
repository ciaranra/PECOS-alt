//! Test for understanding how HUGR represents rotation gates with angles

use pecos_pmir::{PmirConfig, hugr_to_pmir_mlir};

#[test]
fn test_hugr_rotation_gate_structure() {
    // This is how HUGR represents a rotation gate with an angle
    // The angle is passed through the dataflow as a constant input
    let hugr_json = r#"{
        "modules": [{
            "version": "live",
            "metadata": {"name": "rotation_test"},
            "nodes": [
                {"parent": 0, "op": "Module"},
                {"parent": 0, "op": "FuncDefn", "name": "main"},
                {"parent": 1, "op": "Input"},
                {"parent": 1, "op": "Output"},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": {"op": "Const", "value": 1.5708}},
                {"parent": 1, "op": "Extension", "name": "Rx"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"}
            ],
            "edges": [
                [[2, 0], [4, 0]],
                [[4, 0], [6, 0]],
                [[5, 0], [6, 1]],
                [[6, 0], [7, 0]],
                [[7, 0], [3, 0]]
            ]
        }],
        "extensions": []
    }"#;

    // Convert to PMIR MLIR to examine structure
    let config = PmirConfig::default();
    match hugr_to_pmir_mlir(hugr_json, &config) {
        Ok(mlir) => {
            println!("PMIR MLIR representation:\n{mlir}");

            // The MLIR should show the correct angle being parsed
            // With the fix, we expect to see the RX operation with angle
            assert!(
                mlir.contains("@__quantum__qis__rx__body"),
                "Expected RX operation in MLIR output"
            );
            assert!(
                mlir.contains("1.5708"),
                "Expected angle 1.5708 in MLIR output"
            );
        }
        Err(e) => panic!("Failed to convert to PMIR: {e:?}"),
    }
}

#[test]
fn test_edge_based_angle_passing() {
    // Test to understand how angles flow through edges
    let hugr_json = r#"{
        "modules": [{
            "version": "live",
            "metadata": {"name": "edge_test"},
            "nodes": [
                {"parent": 0, "op": "Module"},
                {"parent": 0, "op": "FuncDefn", "name": "main"},
                {"parent": 1, "op": "Input"},
                {"parent": 1, "op": "Output"},
                {"parent": 1, "op": {"op": "Const", "value": 3.14159}},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": "Extension", "name": "Rz"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"}
            ],
            "edges": [
                [[2, 0], [5, 0]],
                [[4, 0], [6, 1]],
                [[5, 0], [6, 0]],
                [[6, 0], [7, 0]],
                [[7, 0], [3, 0]]
            ]
        }],
        "extensions": []
    }"#;

    // Convert to PMIR MLIR to examine structure
    let config = PmirConfig::default();
    match hugr_to_pmir_mlir(hugr_json, &config) {
        Ok(mlir) => {
            println!("\nEdge-based angle passing MLIR:\n{mlir}");

            // The MLIR will show how constants flow
            // With the fix, we expect to see RZ operation with pi value
            assert!(
                mlir.contains("@__quantum__qis__rz__body"),
                "Expected RZ operation in MLIR output"
            );

            // Also check that we have the pi value
            assert!(
                mlir.contains("3.14159"),
                "Expected angle 3.14159 in MLIR output"
            );
        }
        Err(e) => panic!("Failed to convert to PMIR: {e:?}"),
    }
}
