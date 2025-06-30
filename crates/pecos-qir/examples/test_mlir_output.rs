//! Example to test MLIR output generation

use pecos_qir::{PmirConfig, compile_hugr_via_pmir};

fn main() {
    // Simple Hadamard + Measure circuit
    let hugr_json = r#"{
        "version": "0.1.0",
        "name": "hadamard_test",
        "nodes": [
            {"op": {"type": "AllocQubit"}},
            {"op": {"type": "H"}},
            {"op": {"type": "Measure"}},
            {"op": {"type": "Output", "port": 0}}
        ],
        "edges": [
            {"src": [0, 0], "dst": [1, 0]},
            {"src": [1, 0], "dst": [2, 0]},
            {"src": [2, 0], "dst": [3, 0]}
        ]
    }"#;

    let config = PmirConfig {
        debug_output: true,
        optimization_level: 2,
        target_triple: None,
    };

    match compile_hugr_via_pmir(hugr_json, &config) {
        Ok(llvm_ir) => {
            println!("Success! Generated LLVM IR:\n{llvm_ir}");
        }
        Err(e) => {
            eprintln!("Error: {e:?}");

            // If MLIR tools aren't available, generate MLIR text anyway for inspection
            println!("\nGenerating MLIR text for inspection...");

            use pecos_pmir::{hugr_parser, mlir_lowering};

            if let Ok(past) = hugr_parser::parse_hugr_to_past(hugr_json) {
                if let Ok(mlir_module) = mlir_lowering::lower_past_to_pmir(&past, &config) {
                    println!("Generated MLIR:\n{mlir_module}");
                }
            }
        }
    }
}
