//! Test for the PMIR (PECOS MLIR) compilation pipeline

use pecos_pmir::{PmirConfig, compile_hugr_via_pmir};

#[test]
fn test_simple_hadamard_measure() {
    // Sample HUGR JSON (new format with modules array)
    let hugr_json = r#"{
        "modules": [{
            "version": "live",
            "metadata": {"name": "hadamard_test"},
            "nodes": [
                {"parent": 0, "op": "Module"},
                {"parent": 0, "op": "FuncDefn", "name": "main"},
                {"parent": 1, "op": "Input"},
                {"parent": 1, "op": "Output"},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": "Extension", "name": "H"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"}
            ],
            "edges": [
                [[2, 0], [4, 0]],
                [[4, 0], [5, 0]],
                [[5, 0], [6, 0]],
                [[6, 0], [3, 0]]
            ]
        }],
        "extensions": []
    }"#;

    let config = PmirConfig {
        debug_output: true,
        ..Default::default()
    };

    let result = compile_hugr_via_pmir(hugr_json, &config);

    match result {
        Ok(llvm_ir) => {
            println!("Generated LLVM IR:\n{llvm_ir}");

            // Check that the LLVM IR contains expected quantum operations
            assert!(llvm_ir.contains("@__quantum__rt__qubit_allocate"));
            assert!(llvm_ir.contains("@__quantum__qis__h__body"));
            assert!(llvm_ir.contains("@__quantum__qis__m__body"));
            assert!(llvm_ir.contains("ret i32"));
        }
        Err(e) => {
            eprintln!("Compilation failed: {e:?}");
            panic!("PMIR compilation failed");
        }
    }
}

#[test]
fn test_bell_state_circuit() {
    let hugr_json = r#"{
        "modules": [{
            "version": "live",
            "metadata": {"name": "bell_state"},
            "nodes": [
                {"parent": 0, "op": "Module"},
                {"parent": 0, "op": "FuncDefn", "name": "main"},
                {"parent": 1, "op": "Input"},
                {"parent": 1, "op": "Output"},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": "Extension", "name": "QAlloc"},
                {"parent": 1, "op": "Extension", "name": "H"},
                {"parent": 1, "op": "Extension", "name": "CX"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"},
                {"parent": 1, "op": "Extension", "name": "MeasureFree"}
            ],
            "edges": [
                [[2, 0], [4, 0]],
                [[2, 0], [5, 0]],
                [[4, 0], [6, 0]],
                [[6, 0], [7, 0]],
                [[5, 0], [7, 1]],
                [[7, 0], [8, 0]],
                [[7, 1], [9, 0]],
                [[8, 0], [3, 0]],
                [[9, 0], [3, 1]]
            ]
        }],
        "extensions": []
    }"#;

    let config = PmirConfig::default();

    let result = compile_hugr_via_pmir(hugr_json, &config);

    match result {
        Ok(llvm_ir) => {
            println!("Bell state LLVM IR:\n{llvm_ir}");

            // Check for Bell state operations
            assert!(llvm_ir.contains("@__quantum__qis__h__body"));
            assert!(llvm_ir.contains("@__quantum__qis__cx__body")); // HUGR uses cx not cnot
            assert!(llvm_ir.contains("@__quantum__qis__m__body"));

            // Should allocate two qubits (count only calls, not declarations)
            let alloc_count = llvm_ir
                .matches("call i64 @__quantum__rt__qubit_allocate") // HUGR returns i64 not i8*
                .count();
            assert_eq!(alloc_count, 2);
        }
        Err(e) => {
            eprintln!("Bell state compilation failed: {e:?}");
            panic!("PMIR compilation failed");
        }
    }
}
