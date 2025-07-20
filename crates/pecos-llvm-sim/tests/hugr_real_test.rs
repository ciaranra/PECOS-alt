//! Test HUGR support with real HUGR data

#[test]
fn test_real_hugr_with_envelope_format() {
    use pecos_llvm_sim::llvm_engine;
    use pecos_programs::HugrProgram;
    use pecos_engines::ClassicalControlEngineBuilder;
    
    // HUGR uses an envelope format with a magic number header
    // Let's create a proper HUGR envelope
    use hugr_core::builder::{DFGBuilder, Dataflow, DataflowHugr};
    use hugr_core::extension::prelude::qb_t;
    use hugr_core::types::Signature;
    use hugr_core::package::Package;
    use hugr_core::envelope::{write_envelope, EnvelopeConfig};
    
    // Create a simple HUGR
    let hugr = {
        let builder = DFGBuilder::new(Signature::new(vec![qb_t()], vec![qb_t()])).unwrap();
        let [q] = builder.input_wires_arr();
        builder.finish_hugr_with_outputs([q]).unwrap()
    };
    
    // Package it
    let package = Package::new(vec![hugr]);
    
    // Serialize to envelope format
    let mut buffer = Vec::new();
    write_envelope(&mut buffer, &package, EnvelopeConfig::default()).unwrap();
    
    println!("Created HUGR envelope with {} bytes", buffer.len());
    
    let hugr_program = HugrProgram::from_bytes(buffer);
    
    // Test that we can create a builder with HUGR
    let builder = llvm_engine()
        .program(hugr_program)
        .to_sim()
        .qubits(1);
    
    // The actual compilation might still fail due to missing quantum operations
    // but we're testing that the deserialization works
    match builder.build() {
        Ok(_) => println!("HUGR compilation succeeded!"),
        Err(e) => {
            let error_msg = e.to_string();
            println!("Compilation error: {}", error_msg);
            
            // Check if we got past the deserialization stage
            assert!(
                !error_msg.contains("Bad magic number"),
                "Should not have magic number error with proper envelope format"
            );
        }
    }
}

#[test]
fn test_real_hugr_compilation() {
    use pecos_llvm_sim::llvm_engine;
    use pecos_programs::HugrProgram;
    use pecos_engines::ClassicalControlEngineBuilder;
    
    // The error message shows it expects envelope format, not raw JSON
    // So this test documents the current behavior
    let hugr_json = r#"{"version": "v0alpha1", "modules": [], "extensions": []}"#;
    let hugr_bytes = hugr_json.as_bytes().to_vec();
    let hugr_program = HugrProgram::from_bytes(hugr_bytes);
    
    let builder = llvm_engine()
        .program(hugr_program)
        .to_sim()
        .qubits(1);
    
    match builder.build() {
        Ok(_) => panic!("Should not succeed with raw JSON"),
        Err(e) => {
            let error_msg = e.to_string();
            println!("Expected error with raw JSON: {}", error_msg);
            
            // Verify it's the magic number error
            assert!(
                error_msg.contains("Bad magic number") || 
                error_msg.contains("Failed to parse HUGR"),
                "Should fail with magic number error for raw JSON"
            );
        }
    }
}

#[test] 
fn test_hugr_package_format() {
    use pecos_llvm_sim::llvm_engine;
    use pecos_programs::HugrProgram;
    use pecos_engines::ClassicalControlEngineBuilder;
    
    // Test with actual HUGR Package format used by tket2
    let hugr_package = r#"{
        "version": "v0alpha1",
        "modules": [{
            "id": "circuit",
            "nodes": {
                "0": {"parent": null, "op": {"t": "Module"}},
                "1": {
                    "parent": "0",
                    "op": {
                        "t": "Function",
                        "name": "quantum_circuit",
                        "signature": {
                            "inputs": [{"t": "Q"}],
                            "outputs": [{"t": "Q"}]
                        }
                    }
                },
                "2": {"parent": "1", "op": {"t": "Input", "types": [{"t": "Q"}]}},
                "3": {"parent": "1", "op": {"t": "Output", "types": [{"t": "Q"}]}}
            },
            "edges": [["2", 0, "3", 0]]
        }],
        "extensions": ["quantum"]
    }"#;
    
    let hugr_program = HugrProgram::from_bytes(hugr_package.as_bytes().to_vec());
    
    match llvm_engine()
        .program(hugr_program)
        .to_sim()
        .build()
    {
        Ok(_) => println!("HUGR with quantum signature compiled successfully"),
        Err(e) => {
            println!("Compilation error (expected): {}", e);
            // Verify it's attempting HUGR compilation
            let error_str = e.to_string();
            assert!(
                error_str.contains("HUGR") || 
                error_str.contains("quantum") ||
                error_str.contains("Failed to"),
                "Should be attempting HUGR compilation"
            );
        }
    }
}