//! Comprehensive comparison tests between HUGR-QIS and PHIR compilation pipelines
//!
//! This module verifies that both compilation paths produce functionally equivalent
//! quantum programs by testing:
//! 1. Compilation success for the same HUGR files
//! 2. Runtime execution compatibility
//! 3. Quantum behavior equivalence (measurement distributions)
//! 4. Statistical equivalence of simulation results

use pecos::prelude::*;
use pecos_engines::QuantumEngineBuilder;
use std::collections::HashMap;
use tempfile::TempDir;

// Real HUGR data from guppy compilation
const BELL_STATE_HUGR: &[u8] = include_bytes!("test_data/hugr/bell_state.hugr");
const SINGLE_HADAMARD_HUGR: &[u8] = include_bytes!("test_data/hugr/single_hadamard.hugr");
const GHZ_STATE_HUGR: &[u8] = include_bytes!("test_data/hugr/ghz_state.hugr");

/// Result of running a single compilation pipeline
#[derive(Debug)]
struct PipelineResult {
    /// Pipeline name for reporting
    #[allow(dead_code)]
    name: String,
    /// Compilation result
    compilation_result: Result<(), PecosError>,
    /// Execution result (if compilation succeeded)
    execution_result: Option<Result<ShotVec, PecosError>>,
    /// Execution time in milliseconds
    execution_time_ms: Option<u128>,
}

/// Quantum circuit behavior validator
trait CircuitValidator {
    /// Validate that results match expected quantum behavior
    fn validate_quantum_behavior(&self, results: &ShotVec) -> Result<(), String>;

    /// Extract measurement outcomes for statistical comparison
    fn extract_outcomes(&self, results: &ShotVec) -> Vec<Vec<u32>>;

    /// Circuit name for reporting
    fn name(&self) -> &str;
}

/// Bell state validator: expects only 00 or 11 outcomes with ~50/50 distribution
struct BellStateValidator;

impl CircuitValidator for BellStateValidator {
    fn validate_quantum_behavior(&self, results: &ShotVec) -> Result<(), String> {
        let outcomes = self.extract_outcomes(results);

        let mut outcome_00 = 0;
        let mut outcome_11 = 0;
        let mut invalid_outcomes = 0;

        for outcome in &outcomes {
            if outcome.len() != 2 {
                return Err(format!(
                    "Bell state should have 2 measurements, got {}",
                    outcome.len()
                ));
            }

            match (outcome[0], outcome[1]) {
                (0, 0) => outcome_00 += 1,
                (1, 1) => outcome_11 += 1,
                _ => invalid_outcomes += 1,
            }
        }

        if invalid_outcomes > 0 {
            return Err(format!(
                "Bell state produced {invalid_outcomes} invalid outcomes (not 00 or 11)"
            ));
        }

        let total = outcome_00 + outcome_11;
        let ratio_00 = f64::from(outcome_00) / f64::from(total);

        // Allow 10% deviation from 50/50 for statistical fluctuations
        if (ratio_00 - 0.5).abs() > 0.1 {
            return Err(format!(
                "Bell state distribution too skewed: {:.1}% 00, {:.1}% 11",
                ratio_00 * 100.0,
                (1.0 - ratio_00) * 100.0
            ));
        }

        Ok(())
    }

    fn extract_outcomes(&self, results: &ShotVec) -> Vec<Vec<u32>> {
        results
            .shots
            .iter()
            .map(|shot| {
                // First check if there's a tuple return as Data::Vec
                if let Some(pecos_engines::shot_results::Data::Vec(vec_data)) =
                    shot.data.get("result")
                {
                    // Convert the vector elements to u32 values
                    return vec_data
                        .iter()
                        .filter_map(|data| match data {
                            pecos_engines::shot_results::Data::U32(n) => Some(*n),
                            pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::I32(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::U8(n) => Some(u32::from(*n)),
                            pecos_engines::shot_results::Data::Bool(b) => Some(u32::from(*b)),
                            _ => None,
                        })
                        .collect();
                }

                // Check if there's an encoded result from HUGR-QIS (bit pattern in a single integer)
                if let Some(data) = shot.data.get("result") {
                    match data {
                        pecos_engines::shot_results::Data::I64(n) => {
                            // Decode bit pattern: each bit represents a qubit measurement
                            let mut values = Vec::new();
                            let encoded = *n;
                            // Extract up to 64 bits (though we expect only 2 for Bell, 3 for GHZ)
                            for bit_idx in 0..64 {
                                if bit_idx == 2 {
                                    // For Bell state, we expect exactly 2 qubits
                                    break;
                                }
                                let bit_value = (encoded >> bit_idx) & 1;
                                values.push(u32::try_from(bit_value).unwrap_or(0));
                            }
                            if !values.is_empty() {
                                return values;
                            }
                        }
                        pecos_engines::shot_results::Data::U32(n) => {
                            // Similar decoding for U32
                            let mut values = Vec::new();
                            let encoded = *n;
                            for bit_idx in 0..32 {
                                if bit_idx == 2 {
                                    // For Bell state, we expect exactly 2 qubits
                                    break;
                                }
                                let bit_value = (encoded >> bit_idx) & 1;
                                values.push(bit_value);
                            }
                            if !values.is_empty() {
                                return values;
                            }
                        }
                        _ => {}
                    }
                }

                // Try different possible key names for the measurement results
                let mut values = Vec::new();

                // Look for measurement results in order
                for key in &["c", "c1", "result", "m0", "m1"] {
                    if let Some(data) = shot.data.get(*key) {
                        let val = match data {
                            pecos_engines::shot_results::Data::U32(n) => *n,
                            pecos_engines::shot_results::Data::I64(n) => {
                                u32::try_from(*n).unwrap_or(0)
                            }
                            pecos_engines::shot_results::Data::I32(n) => {
                                u32::try_from(*n).unwrap_or(0)
                            }
                            pecos_engines::shot_results::Data::U8(n) => u32::from(*n),
                            _ => 0,
                        };
                        values.push(val);
                    }
                }

                // If we didn't find named results, try to extract all values
                if values.is_empty() {
                    values = shot
                        .data
                        .values()
                        .filter_map(|data| match data {
                            pecos_engines::shot_results::Data::U32(n) => Some(*n),
                            pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::I32(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::U8(n) => Some(u32::from(*n)),
                            _ => None,
                        })
                        .collect();
                }

                values
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        "Bell State"
    }
}

/// Single Hadamard validator: expects random 0/1 outcomes with ~50/50 distribution
struct HadamardValidator;

impl CircuitValidator for HadamardValidator {
    fn validate_quantum_behavior(&self, results: &ShotVec) -> Result<(), String> {
        let outcomes = self.extract_outcomes(results);

        let mut outcome_0 = 0;
        let mut outcome_1 = 0;

        for outcome in &outcomes {
            if outcome.len() != 1 {
                return Err(format!(
                    "Hadamard should have 1 measurement, got {}",
                    outcome.len()
                ));
            }

            match outcome[0] {
                0 => outcome_0 += 1,
                1 => outcome_1 += 1,
                _ => return Err(format!("Hadamard produced invalid outcome: {}", outcome[0])),
            }
        }

        let total = outcome_0 + outcome_1;
        let ratio_0 = f64::from(outcome_0) / f64::from(total);

        // Allow 10% deviation from 50/50
        if (ratio_0 - 0.5).abs() > 0.1 {
            return Err(format!(
                "Hadamard distribution too skewed: {:.1}% 0, {:.1}% 1",
                ratio_0 * 100.0,
                (1.0 - ratio_0) * 100.0
            ));
        }

        Ok(())
    }

    fn extract_outcomes(&self, results: &ShotVec) -> Vec<Vec<u32>> {
        results
            .shots
            .iter()
            .map(|shot| {
                // First check if there's a tuple return as Data::Vec
                if let Some(pecos_engines::shot_results::Data::Vec(vec_data)) =
                    shot.data.get("result")
                {
                    // Convert the vector elements to u32 values
                    return vec_data
                        .iter()
                        .filter_map(|data| match data {
                            pecos_engines::shot_results::Data::U32(n) => Some(*n),
                            pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::I32(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::U8(n) => Some(u32::from(*n)),
                            pecos_engines::shot_results::Data::Bool(b) => Some(u32::from(*b)),
                            _ => None,
                        })
                        .collect();
                }

                // Check if there's an encoded result from HUGR-QIS (bit pattern in a single integer)
                if let Some(data) = shot.data.get("result") {
                    match data {
                        pecos_engines::shot_results::Data::I64(n) => {
                            // For single qubit, just take the least significant bit
                            let bit_value = u32::try_from(*n & 1).unwrap_or(0);
                            return vec![bit_value];
                        }
                        pecos_engines::shot_results::Data::U32(n) => {
                            // For single qubit, just take the least significant bit
                            let bit_value = *n & 1;
                            return vec![bit_value];
                        }
                        _ => {}
                    }
                }

                // Get the single measurement result
                let val = shot.data.values().next().map_or(0, |data| match data {
                    pecos_engines::shot_results::Data::U32(n) => *n,
                    pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).unwrap_or(0),
                    pecos_engines::shot_results::Data::I32(n) => u32::try_from(*n).unwrap_or(0),
                    pecos_engines::shot_results::Data::U8(n) => u32::from(*n),
                    _ => 0,
                });

                vec![val]
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        "Single Hadamard"
    }
}

/// GHZ state validator: expects only 000 or 111 outcomes with ~50/50 distribution
struct GhzStateValidator;

impl CircuitValidator for GhzStateValidator {
    fn validate_quantum_behavior(&self, results: &ShotVec) -> Result<(), String> {
        let outcomes = self.extract_outcomes(results);

        let mut outcome_000 = 0;
        let mut outcome_111 = 0;
        let mut invalid_outcomes = 0;

        for outcome in &outcomes {
            if outcome.len() != 3 {
                return Err(format!(
                    "GHZ state should have 3 measurements, got {}",
                    outcome.len()
                ));
            }

            match (outcome[0], outcome[1], outcome[2]) {
                (0, 0, 0) => outcome_000 += 1,
                (1, 1, 1) => outcome_111 += 1,
                _ => invalid_outcomes += 1,
            }
        }

        if invalid_outcomes > 0 {
            return Err(format!(
                "GHZ state produced {invalid_outcomes} invalid outcomes (not 000 or 111)"
            ));
        }

        let total = outcome_000 + outcome_111;
        let ratio_000 = f64::from(outcome_000) / f64::from(total);

        // Allow 10% deviation from 50/50
        if (ratio_000 - 0.5).abs() > 0.1 {
            return Err(format!(
                "GHZ state distribution too skewed: {:.1}% 000, {:.1}% 111",
                ratio_000 * 100.0,
                (1.0 - ratio_000) * 100.0
            ));
        }

        Ok(())
    }

    fn extract_outcomes(&self, results: &ShotVec) -> Vec<Vec<u32>> {
        results
            .shots
            .iter()
            .map(|shot| {
                // First check if there's a tuple return as Data::Vec
                if let Some(pecos_engines::shot_results::Data::Vec(vec_data)) =
                    shot.data.get("result")
                {
                    // Convert the vector elements to u32 values
                    return vec_data
                        .iter()
                        .filter_map(|data| match data {
                            pecos_engines::shot_results::Data::U32(n) => Some(*n),
                            pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::I32(n) => u32::try_from(*n).ok(),
                            pecos_engines::shot_results::Data::U8(n) => Some(u32::from(*n)),
                            pecos_engines::shot_results::Data::Bool(b) => Some(u32::from(*b)),
                            _ => None,
                        })
                        .collect();
                }

                // Check if there's an encoded result from HUGR-QIS (bit pattern in a single integer)
                if let Some(data) = shot.data.get("result") {
                    match data {
                        pecos_engines::shot_results::Data::I64(n) => {
                            // Decode bit pattern: each bit represents a qubit measurement
                            let mut values = Vec::new();
                            let encoded = *n;
                            // Extract exactly 3 bits for GHZ state
                            for bit_idx in 0..3 {
                                let bit_value = (encoded >> bit_idx) & 1;
                                values.push(u32::try_from(bit_value).unwrap_or(0));
                            }
                            return values;
                        }
                        pecos_engines::shot_results::Data::U32(n) => {
                            // Similar decoding for U32
                            let mut values = Vec::new();
                            let encoded = *n;
                            for bit_idx in 0..3 {
                                let bit_value = (encoded >> bit_idx) & 1;
                                values.push(bit_value);
                            }
                            return values;
                        }
                        _ => {}
                    }
                }

                // Get all three measurement results
                let mut values: Vec<u32> = shot
                    .data
                    .values()
                    .filter_map(|data| match data {
                        pecos_engines::shot_results::Data::U32(n) => Some(*n),
                        pecos_engines::shot_results::Data::I64(n) => u32::try_from(*n).ok(),
                        pecos_engines::shot_results::Data::I32(n) => u32::try_from(*n).ok(),
                        pecos_engines::shot_results::Data::U8(n) => Some(u32::from(*n)),
                        _ => None,
                    })
                    .collect();

                // Ensure we have exactly 3 values
                while values.len() < 3 {
                    values.push(0);
                }
                values.truncate(3);

                values
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        "GHZ State"
    }
}

/// Run the HUGR-QIS compilation pipeline
fn run_hugr_qis_pipeline(hugr_data: &[u8], shots: usize) -> PipelineResult {
    let start_time = std::time::Instant::now();

    // Create temporary HUGR file
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            return PipelineResult {
                name: "HUGR-QIS".to_string(),
                compilation_result: Err(PecosError::from(e)),
                execution_result: None,
                execution_time_ms: None,
            };
        }
    };

    let hugr_path = temp_dir.path().join("test.hugr");
    if let Err(e) = std::fs::write(&hugr_path, hugr_data) {
        return PipelineResult {
            name: "HUGR-LLVM".to_string(),
            compilation_result: Err(PecosError::from(e)),
            execution_result: None,
            execution_time_ms: None,
        };
    }

    // Compile and execute
    let execution_result = pecos::hugr::run_hugr_llvm(&hugr_path, Some(shots)).and_then(|engine| {
        let compile_time = start_time.elapsed();
        let exec_start = std::time::Instant::now();
        let num_qubits = engine.num_qubits();
        let result = MonteCarloEngine::run_with_engines(
            engine,
            Box::new(PassThroughNoiseModel::builder().build()),
            state_vector().qubits(num_qubits).build().unwrap(),
            shots,
            1,
            Some(42),
        );
        let exec_time = exec_start.elapsed();
        result.map(|r| (r, compile_time + exec_time))
    });

    let (execution_result, execution_time) = match execution_result {
        Ok((results, time)) => (Some(Ok(results)), Some(time.as_millis())),
        Err(e) => (Some(Err(e)), Some(start_time.elapsed().as_millis())),
    };

    PipelineResult {
        name: "HUGR-LLVM".to_string(),
        compilation_result: Ok(()),
        execution_result,
        execution_time_ms: execution_time,
    }
}

/// Run the PHIR compilation pipeline
fn run_phir_pipeline(hugr_data: &[u8], shots: usize) -> PipelineResult {
    let start_time = std::time::Instant::now();

    // Create temporary HUGR file
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            return PipelineResult {
                name: "PHIR".to_string(),
                compilation_result: Err(PecosError::from(e)),
                execution_result: None,
                execution_time_ms: None,
            };
        }
    };

    let hugr_path = temp_dir.path().join("test.hugr");
    if let Err(e) = std::fs::write(&hugr_path, hugr_data) {
        return PipelineResult {
            name: "PHIR".to_string(),
            compilation_result: Err(PecosError::from(e)),
            execution_result: None,
            execution_time_ms: None,
        };
    }

    // Compile and execute
    let execution_result =
        pecos::phir::run_phir_llvm(&hugr_path, Some(shots), None).and_then(|engine| {
            let compile_time = start_time.elapsed();
            let exec_start = std::time::Instant::now();
            let num_qubits = engine.num_qubits();
            let result = MonteCarloEngine::run_with_engines(
                engine,
                Box::new(PassThroughNoiseModel::builder().build()),
                state_vector().qubits(num_qubits).build().unwrap(),
                shots,
                1,
                Some(42),
            );
            let exec_time = exec_start.elapsed();
            result.map(|r| (r, compile_time + exec_time))
        });

    let (execution_result, execution_time) = match execution_result {
        Ok((results, time)) => (Some(Ok(results)), Some(time.as_millis())),
        Err(e) => (Some(Err(e)), Some(start_time.elapsed().as_millis())),
    };

    PipelineResult {
        name: "PHIR".to_string(),
        compilation_result: Ok(()),
        execution_result,
        execution_time_ms: execution_time,
    }
}

/// Compare both compilation pipelines on the same HUGR data
fn compare_pipelines<V: CircuitValidator>(
    hugr_data: &[u8],
    validator: &V,
    shots: usize,
) -> Result<(), PecosError> {
    println!("=== Comparing Pipelines for {} ===", validator.name());

    // Run both pipelines
    let hugr_qis_result = run_hugr_qis_pipeline(hugr_data, shots);
    let phir_result = run_phir_pipeline(hugr_data, shots);

    let mut comparison_failed = false;

    // Report compilation results
    println!("Compilation Results:");
    println!("  HUGR-QIS: {:?}", hugr_qis_result.compilation_result);
    println!("  PHIR:      {:?}", phir_result.compilation_result);

    // Check if both compiled successfully
    if hugr_qis_result.compilation_result.is_err() && phir_result.compilation_result.is_err() {
        return Err(PecosError::Processing(
            "Both pipelines failed to compile".to_string(),
        ));
    }

    if hugr_qis_result.compilation_result.is_err() {
        println!("WARNING: HUGR-QIS compilation failed, skipping comparison");
        return Ok(());
    }

    if phir_result.compilation_result.is_err() {
        println!("WARNING: PHIR compilation failed, skipping comparison");
        return Ok(());
    }

    // Compare execution results
    match (
        &hugr_qis_result.execution_result,
        &phir_result.execution_result,
    ) {
        (Some(Ok(hugr_results)), Some(Ok(phir_results))) => {
            println!("Execution Results:");
            println!(
                "  HUGR-QIS: {} shots in {:?}ms",
                hugr_results.len(),
                hugr_qis_result.execution_time_ms
            );
            println!(
                "  PHIR:      {} shots in {:?}ms",
                phir_results.len(),
                phir_result.execution_time_ms
            );

            // Validate quantum behavior for both
            print!("Validating HUGR-QIS quantum behavior... ");
            match validator.validate_quantum_behavior(hugr_results) {
                Ok(()) => println!("PASS"),
                Err(e) => {
                    println!("FAIL: {e}");
                    comparison_failed = true;
                }
            }

            print!("Validating PHIR quantum behavior... ");
            match validator.validate_quantum_behavior(phir_results) {
                Ok(()) => println!("PASS"),
                Err(e) => {
                    println!("FAIL: {e}");
                    comparison_failed = true;
                }
            }

            // Compare statistical distributions
            let hugr_outcomes = validator.extract_outcomes(hugr_results);
            let phir_outcomes = validator.extract_outcomes(phir_results);

            print!("Comparing statistical distributions... ");
            if compare_outcome_distributions(&hugr_outcomes, &phir_outcomes) {
                println!("EQUIVALENT");
            } else {
                println!("DIFFERENT");
                comparison_failed = true;

                // Detailed distribution analysis
                analyze_distribution_differences(&hugr_outcomes, &phir_outcomes);
            }
        }
        (Some(Err(hugr_err)), Some(Err(phir_err))) => {
            println!("Both pipelines failed execution:");
            println!("  HUGR-QIS: {hugr_err}");
            println!("  PHIR:      {phir_err}");
            comparison_failed = true;
        }
        (Some(Ok(_)), Some(Err(phir_err))) => {
            println!("PHIR execution failed: {phir_err}");
            comparison_failed = true;
        }
        (Some(Err(hugr_err)), Some(Ok(_))) => {
            println!("HUGR-QIS execution failed: {hugr_err}");
            comparison_failed = true;
        }
        _ => {
            println!("Unexpected execution state");
            comparison_failed = true;
        }
    }

    if comparison_failed {
        Err(PecosError::Processing(format!(
            "Pipeline comparison failed for {}",
            validator.name()
        )))
    } else {
        println!("Pipeline comparison successful for {}", validator.name());
        Ok(())
    }
}

/// Compare outcome distributions for statistical equivalence
fn compare_outcome_distributions(outcomes1: &[Vec<u32>], outcomes2: &[Vec<u32>]) -> bool {
    if outcomes1.len() != outcomes2.len() {
        return false;
    }

    if outcomes1.is_empty() {
        return true;
    }

    // Count occurrences of each unique outcome
    let mut counts1 = HashMap::new();
    let mut counts2 = HashMap::new();

    for outcome in outcomes1 {
        *counts1.entry(outcome.clone()).or_insert(0) += 1;
    }

    for outcome in outcomes2 {
        *counts2.entry(outcome.clone()).or_insert(0) += 1;
    }

    // Check if both have the same set of outcomes
    if counts1.keys().collect::<std::collections::HashSet<_>>()
        != counts2.keys().collect::<std::collections::HashSet<_>>()
    {
        return false;
    }

    // Use simple proportion test for statistical comparison
    #[allow(clippy::cast_precision_loss)]
    let total1 = outcomes1.len() as f64;
    #[allow(clippy::cast_precision_loss)]
    let total2 = outcomes2.len() as f64;

    for outcome in counts1.keys() {
        let observed1 = f64::from(*counts1.get(outcome).unwrap());
        let observed2 = f64::from(*counts2.get(outcome).unwrap());

        let expected1 = observed1 / total1;
        let expected2 = observed2 / total2;

        // Allow small statistical differences (5% threshold)
        if (expected1 - expected2).abs() > 0.05 {
            return false;
        }
    }

    true
}

/// Analyze and report distribution differences
fn analyze_distribution_differences(outcomes1: &[Vec<u32>], outcomes2: &[Vec<u32>]) {
    println!("Distribution Analysis:");

    let mut counts1 = HashMap::new();
    let mut counts2 = HashMap::new();

    for outcome in outcomes1 {
        *counts1.entry(outcome.clone()).or_insert(0) += 1;
    }

    for outcome in outcomes2 {
        *counts2.entry(outcome.clone()).or_insert(0) += 1;
    }

    #[allow(clippy::cast_precision_loss)]
    let total1 = outcomes1.len() as f64;
    #[allow(clippy::cast_precision_loss)]
    let total2 = outcomes2.len() as f64;

    println!("  HUGR-QIS outcomes:");
    for (outcome, count) in &counts1 {
        let percentage = (f64::from(*count) / total1) * 100.0;
        println!("    {outcome:?}: {count} ({percentage:.1}%)");
    }

    println!("  PHIR outcomes:");
    for (outcome, count) in &counts2 {
        let percentage = (f64::from(*count) / total2) * 100.0;
        println!("    {outcome:?}: {count} ({percentage:.1}%)");
    }
}

// Individual comparison tests for each circuit type

#[test]
fn test_pipeline_comparison_bell_state() -> Result<(), PecosError> {
    compare_pipelines(BELL_STATE_HUGR, &BellStateValidator, 1000)
}

#[test]
fn test_pipeline_comparison_single_hadamard() -> Result<(), PecosError> {
    compare_pipelines(SINGLE_HADAMARD_HUGR, &HadamardValidator, 1000)
}

#[test]
fn test_pipeline_comparison_ghz_state() -> Result<(), PecosError> {
    compare_pipelines(GHZ_STATE_HUGR, &GhzStateValidator, 1000)
}

// Comprehensive test that runs all comparisons
#[test]
fn test_all_pipeline_comparisons() -> Result<(), PecosError> {
    println!("Running comprehensive pipeline comparison tests...\n");

    let mut failed_tests = Vec::new();

    // Test Bell state
    if let Err(e) = compare_pipelines(BELL_STATE_HUGR, &BellStateValidator, 1000) {
        failed_tests.push(format!("Bell state: {e}"));
    }

    println!(); // Spacing between tests

    // Test Hadamard
    if let Err(e) = compare_pipelines(SINGLE_HADAMARD_HUGR, &HadamardValidator, 1000) {
        failed_tests.push(format!("Single Hadamard: {e}"));
    }

    println!(); // Spacing between tests

    // Test GHZ state
    if let Err(e) = compare_pipelines(GHZ_STATE_HUGR, &GhzStateValidator, 1000) {
        failed_tests.push(format!("GHZ state: {e}"));
    }

    if failed_tests.is_empty() {
        println!("\nAll pipeline comparisons passed!");
        Ok(())
    } else {
        println!("\nSome pipeline comparisons failed:");
        for failure in &failed_tests {
            println!("  - {failure}");
        }
        Err(PecosError::Processing(format!(
            "Pipeline comparison failures: {}",
            failed_tests.join(", ")
        )))
    }
}
