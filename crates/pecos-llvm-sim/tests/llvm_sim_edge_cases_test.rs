//! Edge case and advanced feature tests for `llvm_sim()`

use pecos_llvm_sim::LlvmSim;
use std::io::Write;
use tempfile::NamedTempFile;

/// Check if LLVM tools are available
fn skip_if_no_llvm() -> bool {
    let has_llvm = if cfg!(windows) {
        std::env::var("PATH")
            .map(|paths| {
                paths
                    .split(';')
                    .any(|dir| std::path::Path::new(dir).join("clang.exe").exists())
            })
            .unwrap_or(false)
    } else {
        std::env::var("PATH")
            .map(|paths| {
                paths
                    .split(':')
                    .any(|dir| std::path::Path::new(dir).join("llc").exists())
            })
            .unwrap_or(false)
    };

    if has_llvm {
        false
    } else {
        println!("Skipping test: LLVM tools not available");
        true
    }
}

#[test]
fn test_llvm_sim_empty_circuit() {
    if skip_if_no_llvm() {
        return;
    }

    // Test with a circuit that does nothing
    let empty_ir = r#"
define void @main() #0 {
ret void
}
attributes #0 = { "EntryPoint" }
"#;

    let result = LlvmSim::new().llvm(empty_ir).run(10);

    // Empty circuit with no qubits should return an error
    assert!(result.is_err(), "Empty circuit should return an error");

    if let Err(e) = result {
        assert!(
            e.to_string().contains("no qubits allocated"),
            "Error should mention no qubits allocated"
        );
    }
}

#[test]
fn test_llvm_sim_large_shot_count() {
    if skip_if_no_llvm() {
        return;
    }

    // Simple single qubit circuit
    let simple_ir = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.r = constant [2 x i8] c"r\00"

define void @main() #0 {
call void @__quantum__qis__h__body(i64 0)
%r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.r, i32 0, i32 0))
ret void
}
attributes #0 = { "EntryPoint" }
"#;

    // Test with large shot count
    let start = std::time::Instant::now();
    let results = LlvmSim::new()
        .llvm(simple_ir)
        .seed(42)
        .workers(8) // Use multiple workers for speed
        .run(10000)
        .expect("Large shot count should work");

    let elapsed = start.elapsed();
    println!("10,000 shots completed in {:.3}s", elapsed.as_secs_f64());

    assert_eq!(results["r"].len(), 10000);

    // Check distribution
    let ones = results["r"].iter().filter(|&&v| v == 1).count();
    let ratio = ones as f64 / 10000.0;
    println!("Distribution: {:.2}% ones", ratio * 100.0);
    assert!(ratio > 0.45 && ratio < 0.55, "Should be roughly 50/50");
}

#[test]
fn test_llvm_sim_multiple_registers() {
    if skip_if_no_llvm() {
        return;
    }

    // Circuit with multiple named registers
    let multi_reg_ir = r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.a = constant [2 x i8] c"a\00"
@.str.b = constant [2 x i8] c"b\00"
@.str.c = constant [2 x i8] c"c\00"

define void @main() #0 {
; Three independent Hadamard measurements
call void @__quantum__qis__h__body(i64 0)
call void @__quantum__qis__h__body(i64 1)
call void @__quantum__qis__h__body(i64 2)

%r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
%r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
%r2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)

call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.a, i32 0, i32 0))
call void @__quantum__rt__result_record_output(i64 1, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.b, i32 0, i32 0))
call void @__quantum__rt__result_record_output(i64 2, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.c, i32 0, i32 0))

ret void
}
attributes #0 = { "EntryPoint" }
"#;

    let results = LlvmSim::new()
        .llvm(multi_reg_ir)
        .seed(42)
        .run(100)
        .expect("Multiple registers should work");

    // Should have three registers
    assert_eq!(results.len(), 3);
    assert!(results.contains_key("a"));
    assert!(results.contains_key("b"));
    assert!(results.contains_key("c"));

    // Each should have 100 values
    for (name, values) in &results {
        assert_eq!(values.len(), 100, "Register {name} should have 100 values");
    }
}

#[test]
fn test_llvm_sim_biased_depolarizing_noise() {
    if skip_if_no_llvm() {
        return;
    }

    // GHZ state - more sensitive to noise
    let ghz_ir = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.ghz = constant [4 x i8] c"ghz\00"

define void @main() #0 {
; Create GHZ state: |000⟩ + |111⟩
call void @__quantum__qis__h__body(i64 0)
call void @__quantum__qis__cx__body(i64 0, i64 1)
call void @__quantum__qis__cx__body(i64 1, i64 2)

; Measure all qubits
%r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
%r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
%r2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)

call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str.ghz, i32 0, i32 0))
call void @__quantum__rt__result_record_output(i64 1, i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str.ghz, i32 0, i32 0))
call void @__quantum__rt__result_record_output(i64 2, i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str.ghz, i32 0, i32 0))

ret void
}
attributes #0 = { "EntryPoint" }
"#;

    // Run with biased depolarizing noise
    let results = LlvmSim::new()
        .llvm(ghz_ir)
        .seed(42)
        .with_biased_depolarizing_noise(0.05) // 5% biased noise
        .run(1000)
        .expect("Biased noise simulation should work");

    // Count GHZ outcomes
    let ghz_values = &results["ghz"];
    let mut counts = std::collections::HashMap::new();
    for &val in ghz_values {
        *counts.entry(val).or_insert(0) += 1;
    }

    println!("GHZ state with 5% biased depolarizing noise:");
    for (outcome, count) in &counts {
        println!(
            "  {}: {} ({:.1}%)",
            outcome,
            count,
            f64::from(*count) / 10.0
        );
    }

    // Should see mostly 0 and 7 (000 and 111) but some errors
    let ghz_count = counts.get(&0).unwrap_or(&0) + counts.get(&7).unwrap_or(&0);
    assert!(
        ghz_count > 800,
        "GHZ states should dominate even with 5% noise"
    );
}

#[test]
fn test_llvm_sim_file_vs_string_equivalence() {
    if skip_if_no_llvm() {
        return;
    }

    let llvm_ir = r#"
declare void @__quantum__qis__x__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.x = constant [2 x i8] c"x\00"

define void @main() #0 {
call void @__quantum__qis__x__body(i64 0)
%r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.x, i32 0, i32 0))
ret void
}
attributes #0 = { "EntryPoint" }
"#;

    // Create a temp file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{llvm_ir}").expect("Failed to write LLVM IR");
    temp_file.flush().expect("Failed to flush");

    // Run from string
    let results_string = LlvmSim::new()
        .llvm(llvm_ir)
        .seed(42)
        .run(100)
        .expect("String source should work");

    // Run from file
    let results_file = LlvmSim::new()
        .llvm_file(temp_file.path())
        .seed(42)
        .run(100)
        .expect("File source should work");

    // Results should be identical
    assert_eq!(
        results_string, results_file,
        "String and file sources should produce identical results"
    );

    // All should be 1 (X gate flips |0⟩ to |1⟩)
    assert!(
        results_string["x"].iter().all(|&v| v == 1),
        "X gate should always produce |1⟩"
    );
}

#[test]
fn test_llvm_sim_extreme_noise() {
    if skip_if_no_llvm() {
        return;
    }

    // Simple circuit to test extreme noise
    let simple_ir = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.n = constant [2 x i8] c"n\00"

define void @main() #0 {
call void @__quantum__qis__h__body(i64 0)
call void @__quantum__qis__cx__body(i64 0, i64 1)
%r0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
%r1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.n, i32 0, i32 0))
call void @__quantum__rt__result_record_output(i64 1, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.n, i32 0, i32 0))
ret void
}
attributes #0 = { "EntryPoint" }
"#;

    // Test with 50% noise - should be almost random
    let results = LlvmSim::new()
        .llvm(simple_ir)
        .seed(42)
        .with_depolarizing_noise(0.5) // 50% error rate!
        .run(1000)
        .expect("Extreme noise should still work");

    // Count outcomes
    let mut counts = [0; 4];
    for &val in &results["n"] {
        counts[val as usize] += 1;
    }

    println!("Bell state with 50% depolarizing noise:");
    for (i, &count) in counts.iter().enumerate() {
        println!("  {}: {} ({:.1}%)", i, count, f64::from(count) / 10.0);
    }

    // With 50% noise, should be nearly uniform distribution
    for &count in &counts {
        assert!(
            count > 150 && count < 350,
            "With 50% noise, distribution should be nearly uniform"
        );
    }
}

#[test]
fn test_llvm_sim_builder_state_preservation() {
    if skip_if_no_llvm() {
        return;
    }

    // Build a configured simulation
    let mut sim = LlvmSim::new().llvm(r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.s = constant [2 x i8] c"s\00"

define void @main() #0 {
call void @__quantum__qis__h__body(i64 0)
%r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.s, i32 0, i32 0))
ret void
}
attributes #0 = { "EntryPoint" }
"#)
    .seed(12345)
    .workers(2)
    .with_depolarizing_noise(0.1)
    .verbose(false)
    .build()
    .expect("Build should succeed");

    // Run multiple times - configuration should be preserved
    let run1 = sim.run(50).expect("Run 1 should succeed");
    let run2 = sim.run(50).expect("Run 2 should succeed");
    let run3 = sim.run(50).expect("Run 3 should succeed");

    // All runs should have the same size
    assert_eq!(run1["s"].len(), 50);
    assert_eq!(run2["s"].len(), 50);
    assert_eq!(run3["s"].len(), 50);

    // Check stats
    let (total_shots, total_runs) = sim.stats();
    assert_eq!(total_shots, 150);
    assert_eq!(total_runs, 3);
}

#[test]
fn test_llvm_sim_zero_shots() {
    if skip_if_no_llvm() {
        return;
    }

    let simple_ir = r#"
define void @main() #0 {
ret void
}
attributes #0 = { "EntryPoint" }
"#;

    // Should handle 0 shots gracefully
    let results = LlvmSim::new()
        .llvm(simple_ir)
        .run(0)
        .expect("Zero shots should be handled");

    // Should return empty results
    assert!(
        results.is_empty() || results.values().all(std::vec::Vec::is_empty),
        "Zero shots should produce empty results"
    );
}
