//! Test that LLVM runtime correctly handles arbitrary register names
//! This ensures no special handling of specific names like "c"

use pecos_engines::{ClassicalEngine, Engine};
use pecos_qis_runtime::QisEngine;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::io::Write;
use tempfile::NamedTempFile;

/// Generate a Bell state LLVM program with a custom register name
fn generate_bell_state_llvm(register_name: &str) -> String {
    format!(
        r#"; Bell State Circuit with custom register name
; This tests that any register name works correctly

declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.reg = constant [{} x i8] c"{}\00"

define void @main() #0 {{
    ; Create Bell state: |00⟩ + |11⟩
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__cx__body(i64 0, i64 1)

    ; Measure both qubits
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)

    ; Record both results to the custom named register
    call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.reg, i32 0, i32 0))
    call void @__quantum__rt__result_record_output(i64 1, i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.reg, i32 0, i32 0))

    ret void
}}

attributes #0 = {{ "EntryPoint" }}
"#,
        register_name.len() + 1, // +1 for null terminator
        register_name,
        register_name.len() + 1,
        register_name.len() + 1,
        register_name.len() + 1,
        register_name.len() + 1,
    )
}

#[test]
fn test_arbitrary_register_names() {
    // Test with various register names
    let test_names = vec![
        "c",                                              // Original name
        "result",                                         // Should not be special
        "output",                                         // Generic name
        "measurements",                                   // Descriptive name
        "q_results",                                      // With underscore
        "data123",                                        // With numbers
        "UPPERCASE",                                      // All caps
        "CamelCase",                                      // Mixed case
        "_underscore_start",                              // Starting with underscore
        "very_long_register_name_that_should_still_work", // Long name
    ];

    for register_name in test_names {
        println!("\nTesting with register name: '{register_name}'");

        let llvm_code = generate_bell_state_llvm(register_name);

        // Write to temporary file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(llvm_code.as_bytes())
            .expect("Failed to write LLVM code");
        temp_file.flush().expect("Failed to flush temp file");

        let mut engine = QisEngine::new(temp_file.path().to_path_buf());
        engine.compile().expect("Failed to compile LLVM code");

        // Run a single shot
        let shot = engine.process(()).expect("Failed to process shot");

        // Verify the register exists with the correct name
        assert!(
            shot.data.contains_key(register_name),
            "Expected register '{}' not found in shot data. Found: {:?}",
            register_name,
            shot.data.keys().collect::<Vec<_>>()
        );

        // Verify it's a valid Bell state outcome (0 or 3)
        if let Some(pecos_engines::shot_results::Data::I64(value)) = shot.data.get(register_name) {
            assert!(
                value == &0 || value == &3,
                "Invalid Bell state outcome {value} for register '{register_name}'. Expected 0 or 3."
            );
            println!("  Register '{register_name}' correctly contains value {value}");
        } else {
            panic!("Register '{register_name}' has unexpected data type");
        }
    }
}

#[test]
fn test_fuzzed_register_names() {
    // Use a fixed seed for reproducibility
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    // Generate 20 random register names
    for i in 0..20 {
        // Generate random alphanumeric string of length 5-15
        let name_length = rng.random_range(5..=15);
        let register_name: String = (0..name_length)
            .map(|_| {
                // Generate random alphanumeric character
                let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        println!("\nFuzz test {}: register name '{}'", i + 1, register_name);

        let llvm_code = generate_bell_state_llvm(&register_name);

        // Write to temporary file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(llvm_code.as_bytes())
            .expect("Failed to write LLVM code");
        temp_file.flush().expect("Failed to flush temp file");

        let mut engine = QisEngine::new(temp_file.path().to_path_buf());
        engine.compile().expect("Failed to compile LLVM code");

        // Run a single shot
        let shot = engine.process(()).expect("Failed to process shot");

        // Verify the register exists with the correct name
        println!("  Shot data: {:?}", shot.data);
        assert!(
            shot.data.contains_key(&register_name),
            "Expected register '{}' not found in shot data. Found: {:?}",
            register_name,
            shot.data.keys().collect::<Vec<_>>()
        );

        // Verify it's a valid Bell state outcome
        if let Some(pecos_engines::shot_results::Data::I64(value)) = shot.data.get(&register_name) {
            assert!(
                value == &0 || value == &3,
                "Invalid Bell state outcome {value} for register '{register_name}'. Expected 0 or 3."
            );
            println!("  Register '{register_name}' correctly contains value {value}");
        }
    }
}

#[test]
fn test_multiple_registers_different_names() {
    // Test that multiple registers with different names work correctly
    let llvm_code = r#"; Multiple registers with different names
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.alice = constant [6 x i8] c"alice\00"
@.str.bob = constant [4 x i8] c"bob\00"
@.str.charlie = constant [8 x i8] c"charlie\00"

define void @main() #0 {
    ; Apply H to three qubits
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__h__body(i64 1)
    call void @__quantum__qis__h__body(i64 2)

    ; Measure and record to different registers
    call void @__quantum__qis__m__body(i64 0, i64 0)
    call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str.alice, i32 0, i32 0))

    call void @__quantum__qis__m__body(i64 1, i64 1)
    call void @__quantum__rt__result_record_output(i64 1, i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str.bob, i32 0, i32 0))

    call void @__quantum__qis__m__body(i64 2, i64 2)
    call void @__quantum__rt__result_record_output(i64 2, i8* getelementptr inbounds ([8 x i8], [8 x i8]* @.str.charlie, i32 0, i32 0))

    ret void
}

attributes #0 = { "EntryPoint" }
"#;

    // Write to temporary file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file
        .write_all(llvm_code.as_bytes())
        .expect("Failed to write LLVM code");
    temp_file.flush().expect("Failed to flush temp file");

    let mut engine = QisEngine::new(temp_file.path().to_path_buf());
    engine.compile().expect("Failed to compile LLVM code");

    let shot = engine.process(()).expect("Failed to process shot");

    // Verify all three registers exist
    assert!(
        shot.data.contains_key("alice"),
        "Register 'alice' not found"
    );
    assert!(shot.data.contains_key("bob"), "Register 'bob' not found");
    assert!(
        shot.data.contains_key("charlie"),
        "Register 'charlie' not found"
    );

    // Verify each contains 0 or 1 (from H gate measurement)
    for name in &["alice", "bob", "charlie"] {
        if let Some(pecos_engines::shot_results::Data::I64(value)) = shot.data.get(*name) {
            assert!(
                value == &0 || value == &1,
                "Invalid measurement outcome {value} for register '{name}'. Expected 0 or 1."
            );
            println!("Register '{name}' contains value {value}");
        }
    }

    println!("All registers correctly preserved with their names");
}
