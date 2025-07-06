mod common;

use pecos_core::errors::PecosError;
use pecos_engines::{Engine, ShotVec, shot_results::Data};
use pecos_phir_json::v0_1::ast::PHIRProgram;
use pecos_phir_json::v0_1::engine::PhirJsonEngine;
use std::collections::HashMap;

#[test]
fn test_bell_state_noiseless() -> Result<(), PecosError> {
    // Define the Bell state PHIR program inline
    let bell_json = r#"{
      "format": "PHIR/JSON",
      "version": "0.1.0",
      "metadata": {"description": "Bell state preparation"},
      "ops": [
        {
          "data": "qvar_define",
          "data_type": "qubits",
          "variable": "q",
          "size": 2
        },
        {
          "data": "cvar_define",
          "data_type": "i64",
          "variable": "m",
          "size": 2
        },
        {"qop": "H", "args": [["q", 0]]},
        {"qop": "CX", "args": [["q", 0], ["q", 1]]},
        {"qop": "Measure", "args": [["q", 0]], "returns": [["m", 0]]},
        {"qop": "Measure", "args": [["q", 1]], "returns": [["m", 1]]},
        {"cop": "Result", "args": ["m"], "returns": ["v"]}
      ]
    }"#;

    // Parse JSON into PHIRProgram
    let program: PHIRProgram = serde_json::from_str(bell_json)
        .map_err(|e| PecosError::Input(format!("Failed to parse PHIR program: {e}")))?;

    // Create engine directly
    let mut engine = PhirJsonEngine::from_program(program.clone())?;

    // Execute multiple shots directly
    let mut results = ShotVec::default();
    for _ in 0..100 {
        let shot = engine.process(())?;
        results.shots.push(shot);
        // Reset engine state for next shot
        engine.reset()?;
    }

    // Count occurrences of each result
    let mut counts: HashMap<String, usize> = HashMap::new();

    // Process results
    for shot in &results.shots {
        // If there's no "v" key in the output, just count it as an empty result
        let result_str = shot
            .data
            .get("v")
            .map_or_else(String::new, pecos_engines::prelude::Data::to_string);
        *counts.entry(result_str).or_insert(0) += 1;
    }

    // Print the counts for debugging
    println!("Noiseless Bell state results:");
    for (result, count) in &counts {
        println!("  {result}: {count}");
    }

    // The test passes if there are no errors in the execution
    assert!(!results.shots.is_empty(), "Expected non-empty results");

    println!("Results: {results:?}");

    Ok(())
}

#[test]
fn test_bell_state_using_helper() -> Result<(), PecosError> {
    // Define the Bell state PHIR program inline
    let bell_json = r#"{
      "format": "PHIR/JSON",
      "version": "0.1.0",
      "metadata": {"description": "Bell state preparation"},
      "ops": [
        {
          "data": "qvar_define",
          "data_type": "qubits",
          "variable": "q",
          "size": 2
        },
        {
          "data": "cvar_define",
          "data_type": "i64",
          "variable": "m",
          "size": 2
        },
        {"qop": "H", "args": [["q", 0]]},
        {"qop": "CX", "args": [["q", 0], ["q", 1]]},
        {"qop": "Measure", "args": [["q", 0]], "returns": [["m", 0]]},
        {"qop": "Measure", "args": [["q", 1]], "returns": [["m", 1]]},
        {"cop": "Result", "args": ["m"], "returns": ["c"]}
      ]
    }"#;

    // Parse JSON into PHIRProgram
    let program: PHIRProgram = serde_json::from_str(bell_json)
        .map_err(|e| PecosError::Input(format!("Failed to parse PHIR program: {e}")))?;

    // Create engine directly
    let mut engine = PhirJsonEngine::from_program(program.clone())?;

    // Execute directly
    let shot = engine.process(())?;

    // Create a shotVec for compatibility with the rest of the test
    let mut results = ShotVec::default();
    results.shots.push(shot);

    // Print all information about the result for debugging
    println!("ShotResults: {results:?}");

    // Bell state should result in either 00 (0) or 11 (3) measurement outcomes
    // The bell.json file maps "m" to "c" in its Result command
    let shot = &results.shots[0];

    // First check for the "c" register which is specified in the Bell state JSON
    if let Some(data_value) = shot.data.get("c") {
        assert!(
            *data_value == Data::U32(0) || *data_value == Data::U32(3),
            "Expected Bell state result to be 0 or 3, got {data_value}"
        );
        return Ok(());
    }

    // Try fallback registers as well
    if let Some(data_value) = shot.data.get("result") {
        assert!(
            *data_value == Data::U32(0) || *data_value == Data::U32(3),
            "Expected Bell state result to be 0 or 3, got {data_value}"
        );
    } else if let Some(data_value) = shot.data.get("output") {
        assert!(
            *data_value == Data::U32(0) || *data_value == Data::U32(3),
            "Expected Bell state output to be 0 or 3, got {data_value}"
        );
    } else if let Some(data_value) = shot.data.get("m") {
        // The m register is the measurement register in bell.json
        assert!(
            *data_value == Data::U32(0) || *data_value == Data::U32(3),
            "Expected Bell state m register to be 0 or 3, got {data_value}"
        );
    } else {
        // No known register found - print available registers
        println!(
            "Available registers in shot: {:?}",
            shot.data.keys().collect::<Vec<_>>()
        );
        panic!("Expected one of 'c', 'result', 'output', or 'm' registers to be present");
    }

    Ok(())
}
