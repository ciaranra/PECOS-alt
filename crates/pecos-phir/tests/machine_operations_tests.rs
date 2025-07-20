mod common;

#[cfg(test)]
mod tests {
    use pecos_core::errors::PecosError;
    use pecos_engines::shot_results::Data;

    // Import helpers from common module

    // Test machine operations
    #[test]
    fn test_machine_operations() -> Result<(), PecosError> {
        use pecos_engines::Engine;
        use pecos_engines::ShotVec;
        use pecos_phir::v0_1::ast::PHIRProgram;
        use pecos_phir::v0_1::engine::PHIREngine;

        // Define the PHIR program inline
        let phir_json = r#"{
          "format": "PHIR/JSON",
          "version": "0.1.0",
          "metadata": {
            "num_qubits": 2
          },
          "ops": [
            {"data": "qvar_define", "data_type": "qubits", "variable": "q", "size": 2},
            {"data": "cvar_define", "data_type": "i32", "variable": "result", "size": 32},
            {"data": "cvar_define", "data_type": "i32", "variable": "m", "size": 32},
            {"qop": "H", "args": [["q", 0]]},
            {"qop": "CX", "args": [["q", 0], ["q", 1]]},
            {"mop": "Idle", "args": [["q", 0], ["q", 1]], "duration": [5.0, "ms"]},
            {"mop": "Transport", "args": [["q", 0]], "duration": [2.0, "us"], "metadata": {"from_position": [0, 0], "to_position": [1, 0]}},
            {"mop": "Skip"},
            {"qop": "Measure", "args": [["q", 0], ["q", 1]], "returns": [["m", 0], ["m", 1]]},
            {"cop": "=", "args": [2], "returns": ["result"]},
            {"cop": "Result", "args": ["result"], "returns": ["output"]}
          ]
        }"#;

        // Parse JSON into PHIRProgram
        let program: PHIRProgram = serde_json::from_str(phir_json)
            .map_err(|e| PecosError::Input(format!("Failed to parse PHIR program: {e}")))?;

        // Create engine directly
        let mut engine = PHIREngine::from_program(program.clone())?;

        // Execute directly
        let shot = engine.process(())?;

        // Create a shotVec for compatibility with the rest of the test
        let mut results = ShotVec::default();
        results.shots.push(shot);

        // Print results information for debugging
        println!("ShotResults: {results:?}");

        // The actual result value will depend on the quantum simulation,
        // but we just need to verify that the engine successfully processes
        // machine operations without errors and exports the result value
        assert!(!results.shots.is_empty(), "Expected non-empty results");

        let shot = &results.shots[0];
        assert!(
            shot.data.contains_key("output"),
            "Expected 'output' register to be present"
        );

        // Check that the value is 2 (from the assignment in the JSON)
        // Accept either I32(2) or U32(2) as valid results
        let value = shot.data.get("output").unwrap();
        assert!(
            matches!(value, &Data::I32(2) | &Data::U32(2)),
            "Expected output to be 2, got {value:?}"
        );

        Ok(())
    }

    // Test simple machine operations
    #[test]
    fn test_simple_machine_operations() -> Result<(), PecosError> {
        use pecos_engines::Engine;
        use pecos_engines::ShotVec;
        use pecos_phir::v0_1::ast::PHIRProgram;
        use pecos_phir::v0_1::engine::PHIREngine;

        // Define the PHIR program inline
        let phir_json = r#"{
          "format": "PHIR/JSON",
          "version": "0.1.0",
          "metadata": {
            "num_qubits": 2
          },
          "ops": [
            {"data": "qvar_define", "data_type": "qubits", "variable": "q", "size": 2},
            {"data": "cvar_define", "data_type": "i32", "variable": "result", "size": 32},
            {"qop": "H", "args": [["q", 0]]},
            {"mop": "Idle", "args": [["q", 0], ["q", 1]], "duration": [5.0, "ms"]},
            {"mop": "Delay", "args": [["q", 0]], "duration": [2.0, "us"]},
            {"mop": "Transport", "args": [["q", 1]], "duration": [1.0, "ms"], "metadata": {"from_position": [0, 0], "to_position": [1, 0]}},
            {"mop": "Timing", "args": [["q", 0], ["q", 1]], "metadata": {"timing_type": "sync", "label": "sync_point_1"}},
            {"qop": "CX", "args": [["q", 0], ["q", 1]]},
            {"cop": "=", "args": [42], "returns": ["result"]},
            {"cop": "Result", "args": ["result"], "returns": ["output"]}
          ]
        }"#;

        // Parse JSON into PHIRProgram
        let program: PHIRProgram = serde_json::from_str(phir_json)
            .map_err(|e| PecosError::Input(format!("Failed to parse PHIR program: {e}")))?;

        // Create engine directly
        let mut engine = PHIREngine::from_program(program.clone())?;

        // Execute directly
        let shot = engine.process(())?;

        // Create a shotVec for compatibility with the rest of the test
        let mut results = ShotVec::default();
        results.shots.push(shot);

        // Print results information for debugging
        println!("ShotResults: {results:?}");

        // The actual result value will depend on the quantum simulation,
        // but we just need to verify that the engine successfully processes
        // simple machine operations without errors
        assert!(!results.shots.is_empty(), "Expected non-empty results");

        let shot = &results.shots[0];
        assert!(
            shot.data.contains_key("output"),
            "Expected 'output' register to be present"
        );

        // Check that the value is 42 (from the assignment in the JSON file)
        // Accept either I32(42) or U32(42) as valid results
        let value = shot.data.get("output").unwrap();
        assert!(
            matches!(value, &Data::I32(42) | &Data::U32(42)),
            "Expected output to be 42, got {value:?}"
        );

        Ok(())
    }
}
