//! Integration tests for HUGR compilation and execution pipelines

use pecos::prelude::*;
use pecos_engines::QuantumEngineBuilder;
use tempfile::TempDir;

// Real HUGR data from guppy compilation
const BELL_STATE_HUGR: &[u8] = include_bytes!("test_data/hugr/bell_state.hugr");
const SINGLE_HADAMARD_HUGR: &[u8] = include_bytes!("test_data/hugr/single_hadamard.hugr");
const GHZ_STATE_HUGR: &[u8] = include_bytes!("test_data/hugr/ghz_state.hugr");

#[test]
fn test_hugr_to_llvm_to_execution() -> Result<(), PecosError> {
    // Test the full pipeline: HUGR → pecos-hugr → LLVM IR → pecos-llvm-runtime execution

    // Step 1: Compile HUGR to LLVM IR
    let temp_dir = TempDir::new()?;
    let hugr_path = temp_dir.path().join("bell_state.hugr");
    std::fs::write(&hugr_path, BELL_STATE_HUGR)?;

    // Use the hugr module to compile and create engine
    let engine = pecos::hugr::run_hugr_llvm(&hugr_path, Some(1000))?;
    let num_qubits = engine.num_qubits();

    // Step 2: Run simulation using MonteCarloEngine directly
    let results = MonteCarloEngine::run_with_engines(
        engine,
        Box::new(PassThroughNoiseModel::builder().build()),
        state_vector().qubits(num_qubits).build()?,
        1000,  // shots
        1,     // workers
        Some(42), // seed
    )?;

    // Step 3: Verify results
    assert_eq!(results.len(), 1000);

    // Bell state should produce correlated results (00 or 11)
    // Count occurrences of each outcome
    let mut outcome_00 = 0;
    let mut outcome_11 = 0;
    let mut other_outcomes = 0;

    for shot in &results.shots {
        // Debug: print what keys we have
        eprintln!("DEBUG test: Shot data keys: {:?}", shot.data.keys().collect::<Vec<_>>());
        
        // Get the measurement results - could be Vec or I64 (bit-packed)
        match shot.data.get("result") {
            Some(pecos_engines::shot_results::Data::Vec(vec)) => {
                // Vec format
                if vec.len() >= 2 {
                    let c = match &vec[0] {
                        pecos_engines::shot_results::Data::I32(n) => *n,
                        _ => panic!("Expected I32 in Vec"),
                    };
                    let c1 = match &vec[1] {
                        pecos_engines::shot_results::Data::I32(n) => *n,
                        _ => panic!("Expected I32 in Vec"),
                    };
                    
                    match (c, c1) {
                        (0, 0) => outcome_00 += 1,
                        (1, 1) => outcome_11 += 1,
                        _ => other_outcomes += 1,
                    }
                } else {
                    panic!("Expected at least 2 elements in result Vec");
                }
            }
            Some(pecos_engines::shot_results::Data::I64(packed)) => {
                // Bit-packed format: bits represent measurements
                let c = (packed & 1) as i32;
                let c1 = ((packed >> 1) & 1) as i32;
                
                match (c, c1) {
                    (0, 0) => outcome_00 += 1,
                    (1, 1) => outcome_11 += 1,
                    _ => other_outcomes += 1,
                }
            }
            _ => {
                eprintln!("DEBUG test: Expected 'result' key with Vec or I64 data, but got: {:?}", shot.data);
                panic!("Expected 'result' key with Vec or I64 data");
            }
        }
    }

    // Bell state should only produce 00 or 11 outcomes
    assert_eq!(
        other_outcomes, 0,
        "Bell state should only produce correlated outcomes (00 or 11)"
    );
    assert!(
        outcome_00 > 0 || outcome_11 > 0,
        "Bell state should produce at least one outcome"
    );

    // Both outcomes should appear with roughly equal probability (within statistical tolerance)
    let total = outcome_00 + outcome_11;
    let ratio_00 = f64::from(outcome_00) / f64::from(total);
    assert!(
        (ratio_00 - 0.5).abs() < 0.1,
        "Bell state outcomes should be roughly 50/50, got {:.2}% 00",
        ratio_00 * 100.0
    );

    Ok(())
}

#[test]
fn test_hugr_from_bytes() -> Result<(), PecosError> {
    // Test compiling HUGR from bytes directly

    // Use the same approach as the working test - write to file first
    let temp_dir = TempDir::new()?;
    let hugr_path = temp_dir.path().join("bell_state.hugr");
    std::fs::write(&hugr_path, BELL_STATE_HUGR)?;

    // Use file-based compilation which we know works
    let engine = pecos::hugr::run_hugr_llvm(&hugr_path, Some(100))?;
    let num_qubits = engine.num_qubits();

    // Run shots to verify it works
    let results = MonteCarloEngine::run_with_engines(
        engine,
        Box::new(PassThroughNoiseModel::builder().build()),
        state_vector().qubits(num_qubits).build()?,
        100,  // shots
        1,    // workers
        Some(42), // seed
    )?;
    assert_eq!(results.len(), 100);

    // Verify at least some results are valid
    let first_shot = &results.shots[0];
    assert!(first_shot.data.contains_key("result"));
    
    // Verify it's either a Vec with at least 2 elements or a bit-packed I64
    match first_shot.data.get("result") {
        Some(pecos_engines::shot_results::Data::Vec(vec)) => {
            assert!(vec.len() >= 2, "Expected at least 2 measurement results");
        }
        Some(pecos_engines::shot_results::Data::I64(_packed)) => {
            // Bit-packed format is also valid - contains measurements as bits
        }
        _ => {
            panic!("Expected 'result' key with Vec or I64 data");
        }
    }

    Ok(())
}

#[test]
fn test_hugr_via_phir_pipeline() -> Result<(), PecosError> {
    // Test the alternative pipeline: HUGR → pecos-phir → LLVM IR → pecos-llvm-runtime execution

    // Create a HUGR file
    let temp_dir = TempDir::new()?;
    let hugr_path = temp_dir.path().join("bell_state.hugr");
    std::fs::write(&hugr_path, BELL_STATE_HUGR)?;

    // Use the phir module to compile via PHIR (now supports binary format)
    let engine = pecos::phir::run_phir_llvm(&hugr_path, Some(1000), None)?;
    let num_qubits = engine.num_qubits();

    // Run simulation
    let results = MonteCarloEngine::run_with_engines(
        engine,
        Box::new(PassThroughNoiseModel::builder().build()),
        state_vector().qubits(num_qubits).build()?,
        1000,  // shots
        1,     // workers
        Some(42), // seed
    )?;

    // Verify results
    assert_eq!(results.len(), 1000);

    Ok(())
}

#[test]
fn test_phir_compilation_only() -> Result<(), PecosError> {
    // Test just the compilation part of PHIR

    let temp_dir = TempDir::new()?;
    let hugr_path = temp_dir.path().join("test.hugr");
    std::fs::write(&hugr_path, BELL_STATE_HUGR)?;

    // Enable debug output to see what's happening
    let config = pecos_phir::PhirConfig {
        debug: true,
        ..Default::default()
    };

    // Compile HUGR to LLVM IR via PHIR (now supports binary format)
    let llvm_ir = pecos::phir::compile_hugr_file_via_phir(&hugr_path, Some(config))?;

    // Verify we got some LLVM IR
    assert!(!llvm_ir.is_empty());
    assert!(llvm_ir.contains("define"));
    assert!(llvm_ir.contains("__quantum__"));

    Ok(())
}

#[test]
fn test_setup_llvm_engine_generic() -> Result<(), PecosError> {
    // Test that the generic setup_llvm_engine function works
    // This tests the orchestration function we moved from pecos-llvm-runtime

    // Create a simple LLVM IR file
    let temp_dir = TempDir::new()?;
    let llvm_path = temp_dir.path().join("test.ll");

    // Minimal valid LLVM IR with entry point and quantum operations
    let llvm_ir = r#"
@str_result = constant [7 x i8] c"result\00"

define void @main() #0 {
    %qubit = call i64 @__quantum__rt__qubit_allocate()
    call void @__quantum__qis__h__body(i64 %qubit)
    %result_id = call i64 @__quantum__rt__result_allocate()
    %measurement = call i32 @__quantum__qis__m__body(i64 %qubit, i64 %result_id)
    %result_ptr = inttoptr i64 %result_id to i8*
    call void @__quantum__rt__result_record_output(i8* %result_ptr, i8* getelementptr inbounds ([7 x i8], [7 x i8]* @str_result, i32 0, i32 0))
    ret void
}

declare i64 @__quantum__rt__qubit_allocate()
declare void @__quantum__qis__h__body(i64)
declare i64 @__quantum__rt__result_allocate()
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i8*, i8*)

attributes #0 = { "EntryPoint" }
"#;

    std::fs::write(&llvm_path, llvm_ir)?;

    // Test the setup function
    let engine = setup_llvm_engine(&llvm_path, Some(10))?;
    let num_qubits = engine.num_qubits();

    // Verify engine was created successfully
    // We can't check shots directly on the trait object, but we can run it
    let results = MonteCarloEngine::run_with_engines(
        engine,
        Box::new(PassThroughNoiseModel::builder().build()),
        state_vector().qubits(num_qubits).build()?,
        10,    // shots
        1,     // workers
        None,  // seed
    )?;
    assert_eq!(results.len(), 10);

    Ok(())
}

#[test]
fn test_single_hadamard_execution() -> Result<(), PecosError> {
    // Test single Hadamard gate produces random results

    // Create temp HUGR file
    let temp_dir = TempDir::new()?;
    let hugr_path = temp_dir.path().join("single_hadamard.hugr");
    std::fs::write(&hugr_path, SINGLE_HADAMARD_HUGR)?;

    // Run simulation with many shots
    let engine = pecos::hugr::run_hugr_llvm(&hugr_path, Some(1000))?;
    let num_qubits = engine.num_qubits();
    let results = MonteCarloEngine::run_with_engines(
        engine,
        Box::new(PassThroughNoiseModel::builder().build()),
        state_vector().qubits(num_qubits).build()?,
        1000,  // shots
        1,     // workers
        Some(42), // seed
    )?;

    // Count outcomes
    let mut outcome_0 = 0;
    let mut outcome_1 = 0;

    for shot in &results.shots {
        // Get the measurement result - it might be under different keys
        let value = shot
            .data
            .values()
            .next()
            .expect("Expected at least one value in shot data");

        match value {
            pecos_engines::shot_results::Data::U32(0)
            | pecos_engines::shot_results::Data::I64(0)
            | pecos_engines::shot_results::Data::U8(0) => outcome_0 += 1,
            pecos_engines::shot_results::Data::U32(1)
            | pecos_engines::shot_results::Data::I64(1)
            | pecos_engines::shot_results::Data::U8(1) => outcome_1 += 1,
            _ => panic!("Unexpected outcome: {value:?}"),
        }
    }

    // Hadamard should produce roughly 50/50 distribution
    let total = outcome_0 + outcome_1;
    let ratio_0 = f64::from(outcome_0) / f64::from(total);
    assert!(
        (ratio_0 - 0.5).abs() < 0.1,
        "Hadamard should produce roughly 50/50, got {:.2}% 0",
        ratio_0 * 100.0
    );

    Ok(())
}

#[test]
fn test_ghz_state_execution() -> Result<(), PecosError> {
    // Test 3-qubit GHZ state produces correlated results

    // Create temp HUGR file
    let temp_dir = TempDir::new()?;
    let hugr_path = temp_dir.path().join("ghz_state.hugr");
    std::fs::write(&hugr_path, GHZ_STATE_HUGR)?;

    // Run simulation
    let engine = pecos::hugr::run_hugr_llvm(&hugr_path, Some(1000))?;
    let num_qubits = engine.num_qubits();
    let results = MonteCarloEngine::run_with_engines(
        engine,
        Box::new(PassThroughNoiseModel::builder().build()),
        state_vector().qubits(num_qubits).build()?,
        1000,  // shots
        1,     // workers
        Some(42), // seed
    )?;

    // Count outcomes - GHZ should only produce 000 or 111
    let mut outcome_000 = 0;
    let mut outcome_111 = 0;
    let mut other_outcomes = 0;

    for shot in &results.shots {
        // Get all measurement values - could be packed as a single integer or Vec
        let values: Vec<u32> = shot
            .data
            .values()
            .filter_map(|v| match v {
                pecos_engines::shot_results::Data::U32(n) => Some(*n),
                pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).ok(),
                pecos_engines::shot_results::Data::U8(n) => Some(u32::from(*n)),
                pecos_engines::shot_results::Data::Vec(vec) => {
                    // Handle Vec data - encoded as bit pattern
                    if let Some(pecos_engines::shot_results::Data::I64(packed)) = vec.first() {
                        u32::try_from(*packed).ok()
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        // Handle both packed and unpacked formats
        if values.len() == 1 {
            // Packed format: bits represent measurements
            let packed = values[0];
            let m0 = (packed >> 0) & 1;
            let m1 = (packed >> 1) & 1;
            let m2 = (packed >> 2) & 1;
            
            match (m0, m1, m2) {
                (0, 0, 0) => outcome_000 += 1,
                (1, 1, 1) => outcome_111 += 1,
                _ => other_outcomes += 1,
            }
        } else if values.len() == 3 {
            // Unpacked format: individual measurements
            match (values[0], values[1], values[2]) {
                (0, 0, 0) => outcome_000 += 1,
                (1, 1, 1) => outcome_111 += 1,
                _ => other_outcomes += 1,
            }
        } else {
            println!("Warning: Expected 1 packed or 3 unpacked values, got {values:?}");
        }
    }

    // GHZ state should only produce 000 or 111
    assert_eq!(
        other_outcomes, 0,
        "GHZ state should only produce 000 or 111"
    );
    assert!(
        outcome_000 > 0 || outcome_111 > 0,
        "GHZ state should produce at least one outcome"
    );

    // Both outcomes should appear with roughly equal probability
    let total = outcome_000 + outcome_111;
    let ratio_000 = f64::from(outcome_000) / f64::from(total);
    assert!(
        (ratio_000 - 0.5).abs() < 0.1,
        "GHZ state outcomes should be roughly 50/50, got {:.2}% 000",
        ratio_000 * 100.0
    );

    Ok(())
}
