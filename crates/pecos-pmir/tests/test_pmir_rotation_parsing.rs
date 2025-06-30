//! Test for understanding how HUGR represents rotation gates with angles

use pecos_pmir::hugr_to_past_ron;

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

    // Convert to PAST RON to examine structure
    match hugr_to_past_ron(hugr_json) {
        Ok(ron) => {
            println!("PAST RON representation:\n{ron}");

            // The RON should show the correct angle being parsed
            // With the fix, we expect to see RX(1.5708)
            assert!(
                ron.contains("RX(1.5708)"),
                "Expected RX(1.5708) in RON output"
            );
        }
        Err(e) => panic!("Failed to convert to RON: {e:?}"),
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

    // Convert to PAST RON to examine structure
    match hugr_to_past_ron(hugr_json) {
        Ok(ron) => {
            println!("\nEdge-based angle passing RON:\n{ron}");

            // The RON will show the edges and how constants flow
            // With the fix, we expect to see RZ(3.14159)
            assert!(
                ron.contains("RZ(3.14159)"),
                "Expected RZ(3.14159) in RON output"
            );

            // Also check that we have a Const node with the pi value
            assert!(
                ron.contains("Const(Float(3.14159))"),
                "Expected Const(Float(3.14159)) in RON output"
            );
        }
        Err(e) => panic!("Failed to convert to RON: {e:?}"),
    }
}
