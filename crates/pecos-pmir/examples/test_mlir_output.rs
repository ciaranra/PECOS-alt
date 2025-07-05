//! Example to test MLIR output generation

use pecos_pmir::{InputFormat, PMIRConfig, Pipeline};

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

    let config = PMIRConfig {
        debug: true,
        optimization_level: 2,
        target_triple: None,
        ..Default::default()
    };

    let pipeline = Pipeline::new(config);
    let result: Result<(), _> = pipeline.compile_and_execute(hugr_json, InputFormat::HUGR);

    match result {
        Ok(()) => {
            println!("Success! Pipeline execution completed.");
        }
        Err(e) => {
            eprintln!("Error: {e:?}");
            println!("This is expected since parsers are not yet implemented.");
        }
    }
}
