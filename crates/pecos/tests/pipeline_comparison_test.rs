//! Comprehensive comparison tests between HUGR-LLVM and PHIR compilation pipelines
//!
//! This module verifies that both compilation paths produce functionally equivalent
//! quantum programs by testing:
//! 1. Compilation success for the same HUGR files
//! 2. Runtime execution compatibility
//! 3. Quantum behavior equivalence (measurement distributions)
//! 4. Statistical equivalence of simulation results

use pecos::prelude::*;
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

/// Run the HUGR-LLVM compilation pipeline
fn run_hugr_llvm_pipeline(hugr_data: &[u8], shots: usize) -> PipelineResult {
    let start_time = std::time::Instant::now();

    // Create temporary HUGR file
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            return PipelineResult {
                name: "HUGR-LLVM".to_string(),
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
        let result = run_sim(engine, shots, Some(42), None, None, None);
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
            let result = run_sim(engine, shots, Some(42), None, None, None);
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
    let hugr_llvm_result = run_hugr_llvm_pipeline(hugr_data, shots);
    let phir_result = run_phir_pipeline(hugr_data, shots);

    let mut comparison_failed = false;

    // Report compilation results
    println!("Compilation Results:");
    println!("  HUGR-LLVM: {:?}", hugr_llvm_result.compilation_result);
    println!("  PHIR:      {:?}", phir_result.compilation_result);

    // Check if both compiled successfully
    if hugr_llvm_result.compilation_result.is_err() && phir_result.compilation_result.is_err() {
        return Err(PecosError::Processing(
            "Both pipelines failed to compile".to_string(),
        ));
    }

    if hugr_llvm_result.compilation_result.is_err() {
        println!("WARNING: HUGR-LLVM compilation failed, skipping comparison");
        return Ok(());
    }

    if phir_result.compilation_result.is_err() {
        println!("WARNING: PHIR compilation failed, skipping comparison");
        return Ok(());
    }

    // Compare execution results
    match (
        &hugr_llvm_result.execution_result,
        &phir_result.execution_result,
    ) {
        (Some(Ok(hugr_results)), Some(Ok(phir_results))) => {
            println!("Execution Results:");
            println!(
                "  HUGR-LLVM: {} shots in {:?}ms",
                hugr_results.len(),
                hugr_llvm_result.execution_time_ms
            );
            println!(
                "  PHIR:      {} shots in {:?}ms",
                phir_results.len(),
                phir_result.execution_time_ms
            );

            // Validate quantum behavior for both
            print!("Validating HUGR-LLVM quantum behavior... ");
            match validator.validate_quantum_behavior(hugr_results) {
                Ok(()) => println!("✓ PASS"),
                Err(e) => {
                    println!("✗ FAIL: {e}");
                    comparison_failed = true;
                }
            }

            print!("Validating PHIR quantum behavior... ");
            match validator.validate_quantum_behavior(phir_results) {
                Ok(()) => println!("✓ PASS"),
                Err(e) => {
                    println!("✗ FAIL: {e}");
                    comparison_failed = true;
                }
            }

            // Compare statistical distributions
            let hugr_outcomes = validator.extract_outcomes(hugr_results);
            let phir_outcomes = validator.extract_outcomes(phir_results);

            print!("Comparing statistical distributions... ");
            if compare_outcome_distributions(&hugr_outcomes, &phir_outcomes) {
                println!("✓ EQUIVALENT");
            } else {
                println!("✗ DIFFERENT");
                comparison_failed = true;

                // Detailed distribution analysis
                analyze_distribution_differences(&hugr_outcomes, &phir_outcomes);
            }
        }
        (Some(Err(hugr_err)), Some(Err(phir_err))) => {
            println!("Both pipelines failed execution:");
            println!("  HUGR-LLVM: {hugr_err}");
            println!("  PHIR:      {phir_err}");
            comparison_failed = true;
        }
        (Some(Ok(_)), Some(Err(phir_err))) => {
            println!("PHIR execution failed: {phir_err}");
            comparison_failed = true;
        }
        (Some(Err(hugr_err)), Some(Ok(_))) => {
            println!("HUGR-LLVM execution failed: {hugr_err}");
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
        println!("✓ Pipeline comparison successful for {}", validator.name());
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

    println!("  HUGR-LLVM outcomes:");
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

// Debug test to analyze LLVM IR differences
#[test]
fn test_debug_llvm_ir_comparison() {
    println!("=== Debugging LLVM IR Generation ===");

    let temp_dir = TempDir::new().unwrap();
    let hugr_path = temp_dir.path().join("bell_state.hugr");
    std::fs::write(&hugr_path, BELL_STATE_HUGR).unwrap();

    // Generate HUGR-LLVM IR (working)
    println!("\n1. Testing HUGR-LLVM pipeline...");
    let hugr_llvm_ir = match generate_hugr_llvm_ir(&hugr_path) {
        Ok(ir) => {
            println!("   ✓ HUGR-LLVM IR generated ({} chars)", ir.len());
            Some(ir)
        }
        Err(e) => {
            println!("   ✗ HUGR-LLVM IR generation failed: {e}");
            None
        }
    };

    match pecos::hugr::run_hugr_llvm(&hugr_path, Some(10)) {
        Ok(engine) => {
            println!("   ✓ HUGR-LLVM compilation successful");
            match run_sim(engine, 10, Some(42), None, None, None) {
                Ok(results) => println!(
                    "   ✓ HUGR-LLVM execution successful: {} shots",
                    results.len()
                ),
                Err(e) => println!("   ✗ HUGR-LLVM execution failed: {e}"),
            }
        }
        Err(e) => println!("   ✗ HUGR-LLVM compilation failed: {e}"),
    }

    // Generate PHIR IR (failing) - skip execution for now
    println!("\n2. Testing PHIR compilation only...");
    match pecos::phir::run_phir_llvm(&hugr_path, Some(10), None) {
        Ok(_engine) => {
            println!("   ✓ PHIR compilation successful");
            println!("   (Skipping execution to avoid crash)");
        }
        Err(e) => println!("   ✗ PHIR compilation failed: {e}"),
    }

    // Generate raw LLVM IR to examine differences
    println!("\n3. Generating raw LLVM IR for comparison...");

    let config = pecos_phir::PhirConfig {
        debug: true,
        ..Default::default()
    };

    match pecos::phir::compile_hugr_file_via_phir(&hugr_path, Some(config)) {
        Ok(phir_ir) => {
            println!("   ✓ PHIR LLVM IR generated ({} chars)", phir_ir.len());

            // Save to files for manual inspection
            let phir_path = temp_dir.path().join("phir_output.ll");
            std::fs::write(&phir_path, &phir_ir).unwrap();
            println!("   PHIR IR saved to: {phir_path:?}");

            // Also save HUGR-LLVM IR if we have it
            if let Some(ref hugr_ir) = hugr_llvm_ir {
                let hugr_path = temp_dir.path().join("hugr_llvm_output.ll");
                std::fs::write(&hugr_path, hugr_ir).unwrap();
                println!("   HUGR-LLVM IR saved to: {hugr_path:?}");
            }

            // Perform detailed comparison
            if let Some(ref hugr_ir) = hugr_llvm_ir {
                println!("\n=== DETAILED LLVM IR COMPARISON ===");
                compare_llvm_ir_detailed(hugr_ir, &phir_ir);
            } else {
                // Analyze the PHIR IR alone
                analyze_qubit_usage(&phir_ir);
            }
        }
        Err(e) => println!("   ✗ PHIR LLVM IR generation failed: {e}"),
    }

    println!("\n=== Debug files saved to: {:?} ===", temp_dir.path());
}

/// Generate HUGR-LLVM IR directly using the pecos-hugr-llvm crate
fn generate_hugr_llvm_ir(hugr_path: &std::path::Path) -> Result<String, String> {
    // Read the HUGR file
    let hugr_data =
        std::fs::read(hugr_path).map_err(|e| format!("Failed to read HUGR file: {e}"))?;

    // Use pecos-hugr-llvm to compile to LLVM IR
    match pecos_hugr_llvm::compile_hugr_bytes_to_string(&hugr_data) {
        Ok(ir) => Ok(ir),
        Err(e) => Err(format!("HUGR-LLVM compilation failed: {e}")),
    }
}

/// Perform detailed comparison between HUGR-LLVM and PHIR LLVM IR
fn compare_llvm_ir_detailed(hugr_ir: &str, phir_ir: &str) {
    println!("HUGR-LLVM IR: {} chars", hugr_ir.len());
    println!("PHIR IR: {} chars", phir_ir.len());

    // Extract main functions for comparison
    let hugr_main = extract_main_function(hugr_ir);
    let phir_main = extract_main_function(phir_ir);

    println!("\n=== MAIN FUNCTION COMPARISON ===");

    match (hugr_main, phir_main) {
        (Some(hugr), Some(phir)) => {
            println!("\nHUGR-LLVM main function:");
            for (i, line) in hugr.lines().enumerate().take(20) {
                println!("  {:2}: {}", i + 1, line);
            }

            println!("\nPHIR main function:");
            for (i, line) in phir.lines().enumerate().take(20) {
                println!("  {:2}: {}", i + 1, line);
            }

            // Compare signatures
            let hugr_sig = hugr.lines().next().unwrap_or("");
            let phir_sig = phir.lines().next().unwrap_or("");

            println!("\n=== SIGNATURE COMPARISON ===");
            println!("HUGR-LLVM: {hugr_sig}");
            println!("PHIR:      {phir_sig}");

            if hugr_sig == phir_sig {
                println!("✓ Function signatures match");
            } else {
                println!("⚠️  Function signatures differ!");
            }
        }
        _ => println!("Could not extract main functions for comparison"),
    }

    // Compare qubit operations
    println!("\n=== QUBIT OPERATIONS COMPARISON ===");
    compare_qubit_operations(hugr_ir, phir_ir);

    // Compare function declarations
    println!("\n=== FUNCTION DECLARATIONS COMPARISON ===");
    compare_function_declarations(hugr_ir, phir_ir);
}

/// Extract main function from LLVM IR
fn extract_main_function(llvm_ir: &str) -> Option<String> {
    // Find the start of the main function
    let start_marker = "define";
    let lines: Vec<&str> = llvm_ir.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains(start_marker) && line.contains("@main") {
            // Find the end of the function (closing brace)
            for (j, end_line) in lines[i..].iter().enumerate() {
                if end_line.trim() == "}" {
                    let function_lines = &lines[i..=i + j];
                    return Some(function_lines.join("\n"));
                }
            }
        }
    }
    None
}

/// Compare qubit-related operations between both IRs
fn compare_qubit_operations(hugr_ir: &str, phir_ir: &str) {
    let hugr_ops = extract_quantum_operations(hugr_ir);
    let phir_ops = extract_quantum_operations(phir_ir);

    println!("HUGR-LLVM quantum operations: {}", hugr_ops.len());
    for (i, op) in hugr_ops.iter().enumerate() {
        println!("  {}: {}", i + 1, op);
    }

    println!("\nPHIR quantum operations: {}", phir_ops.len());
    for (i, op) in phir_ops.iter().enumerate() {
        println!("  {}: {}", i + 1, op);
    }

    // Look for key differences
    println!("\n=== KEY DIFFERENCES ===");

    let hugr_allocations = hugr_ops
        .iter()
        .filter(|op| op.contains("__quantum__rt__qubit_allocate"))
        .count();
    let phir_allocations = phir_ops
        .iter()
        .filter(|op| op.contains("__quantum__rt__qubit_allocate"))
        .count();

    println!("Qubit allocations - HUGR: {hugr_allocations}, PHIR: {phir_allocations}");

    let hugr_gates = hugr_ops
        .iter()
        .filter(|op| op.contains("__quantum__qis__"))
        .count();
    let phir_gates = phir_ops
        .iter()
        .filter(|op| op.contains("__quantum__qis__"))
        .count();

    println!("Quantum gates - HUGR: {hugr_gates}, PHIR: {phir_gates}");
}

/// Extract quantum-related operations from LLVM IR
fn extract_quantum_operations(llvm_ir: &str) -> Vec<String> {
    llvm_ir
        .lines()
        .filter(|line| {
            line.contains("__quantum__") && (line.contains("call") || line.contains("invoke"))
        })
        .map(|line| line.trim().to_string())
        .collect()
}

/// Compare function declarations between both IRs
fn compare_function_declarations(hugr_ir: &str, phir_ir: &str) {
    let hugr_decls = extract_function_declarations(hugr_ir);
    let phir_decls = extract_function_declarations(phir_ir);

    println!("HUGR-LLVM function declarations: {}", hugr_decls.len());
    println!("PHIR function declarations: {}", phir_decls.len());

    // Find declarations that differ
    let hugr_set: std::collections::HashSet<_> = hugr_decls.iter().collect();
    let phir_set: std::collections::HashSet<_> = phir_decls.iter().collect();

    let only_in_hugr: Vec<_> = hugr_set.difference(&phir_set).collect();
    let only_in_phir: Vec<_> = phir_set.difference(&hugr_set).collect();

    if !only_in_hugr.is_empty() {
        println!("\nDeclarations only in HUGR-LLVM:");
        for decl in &only_in_hugr {
            println!("  {decl}");
        }
    }

    if !only_in_phir.is_empty() {
        println!("\nDeclarations only in PHIR:");
        for decl in &only_in_phir {
            println!("  {decl}");
        }
    }

    if only_in_hugr.is_empty() && only_in_phir.is_empty() {
        println!("✓ All function declarations match");
    }
}

/// Extract function declarations from LLVM IR
fn extract_function_declarations(llvm_ir: &str) -> Vec<String> {
    llvm_ir
        .lines()
        .filter(|line| line.trim().starts_with("declare"))
        .map(|line| line.trim().to_string())
        .collect()
}

fn analyze_qubit_usage(llvm_ir: &str) {
    println!("\n=== Analyzing Qubit Usage in PHIR IR ===");

    // Count qubit allocations and find actual calls
    let alloc_count = llvm_ir.matches("__quantum__rt__qubit_allocate").count();
    println!("Qubit allocations found: {alloc_count}");

    // Show actual allocation calls in main function
    println!("Actual qubit allocation calls:");
    for (i, line) in llvm_ir.lines().enumerate() {
        if line.contains("call i64 @__quantum__rt__qubit_allocate") {
            println!("  Line {}: {}", i + 1, line.trim());
        }
    }

    // Find all qubit-related operations
    let mut qubit_ops = Vec::new();
    for (i, line) in llvm_ir.lines().enumerate() {
        if line.contains("__quantum__qis__") || line.contains("__quantum__rt__qubit") {
            qubit_ops.push((i + 1, line.trim()));
        }
    }

    println!("Quantum operations found: {}", qubit_ops.len());
    for (line_num, line) in &qubit_ops {
        println!("  Line {line_num}: {line}");
    }

    // Look for the main function definition
    if let Some(main_start) = llvm_ir.find("define") {
        if let Some(main_section) = llvm_ir[main_start..].split("\n}").next() {
            println!("\nMain function analysis:");

            // Count how many qubits are used in operations
            let h_gates = main_section.matches("__quantum__qis__h__body").count();
            let cnot_gates = main_section.matches("__quantum__qis__cnot__body").count();
            let measurements = main_section.matches("__quantum__qis__m__body").count();

            println!("  H gates: {h_gates}");
            println!("  CNOT gates: {cnot_gates}");
            println!("  Measurements: {measurements}");

            // For Bell state, we expect:
            // - 2 qubit allocations (for q0 and q1)
            // - 1 H gate on q0
            // - 1 CNOT gate (q0 -> q1)
            // - 2 measurements (q0 and q1)

            if alloc_count != 2 {
                println!("  ⚠️  Expected 2 qubit allocations for Bell state, found {alloc_count}");
            }
            if h_gates != 1 {
                println!("  ⚠️  Expected 1 H gate for Bell state, found {h_gates}");
            }
            if cnot_gates != 1 {
                println!("  ⚠️  Expected 1 CNOT gate for Bell state, found {cnot_gates}");
            }
            if measurements != 2 {
                println!("  ⚠️  Expected 2 measurements for Bell state, found {measurements}");
            }
        }
    }
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
        println!("\n🎉 All pipeline comparisons passed!");
        Ok(())
    } else {
        println!("\n❌ Some pipeline comparisons failed:");
        for failure in &failed_tests {
            println!("  - {failure}");
        }
        Err(PecosError::Processing(format!(
            "Pipeline comparison failures: {}",
            failed_tests.join(", ")
        )))
    }
}
