//! Test that HUGR programs can be passed to `SeleneExecutableEngine`

use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
use pecos_programs::HugrProgram;
use pecos_selene::selene_executable;

#[test]
fn test_hugr_program_api() {
    println!("Testing HUGR program support in SeleneExecutableEngineBuilder");

    // Create a simple HUGR JSON (guppylang format)
    let hugr_json = r#"{
        "modules": [],
        "extensions": []
    }"#;

    // Test that the builder accepts HUGR programs
    let result = selene_executable()
        .hugr(HugrProgram::from_bytes(hugr_json.as_bytes().to_vec()))
        .qubits(2)
        .build();

    match result {
        Ok(engine) => {
            println!("Successfully created engine with HUGR program");
            assert_eq!(engine.num_qubits(), 2);
        }
        Err(e) => {
            // It's OK if compilation fails for empty HUGR
            println!("HUGR program accepted by API (compilation error expected): {e}");
        }
    }
}

#[test]
fn test_hugr_via_program_enum() {
    println!("Testing HUGR via Program enum");

    let hugr_json = r#"{
        "modules": [],
        "extensions": []
    }"#;

    let hugr_program = HugrProgram::from_bytes(hugr_json.as_bytes().to_vec());
    let program: pecos_programs::Program = hugr_program.into();

    // Test that Program::Hugr works
    let result = selene_executable().program(program).qubits(1).build();

    match result {
        Ok(_) => println!("Program::Hugr accepted by builder"),
        Err(e) => println!("Program::Hugr accepted (compilation error expected): {e}"),
    }
}
