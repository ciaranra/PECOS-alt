//! Edge case and advanced feature tests for LLVM simulation unified API

use pecos_engines::{
    BiasedDepolarizingNoise, ClassicalControlEngineBuilder, DepolarizingNoise, sim_builder,
};
use pecos_qis_sim::qis_engine;
use pecos_programs::QisProgram;
use std::io::Write;
use tempfile::NamedTempFile;

mod common;
use common::get_register_i64;

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
fn test_qis_sim_empty_circuit() {
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

    let result = qis_engine()
        .program(QisProgram::from_string(empty_ir))
        .to_sim()
        .qubits(1)
        .run(10);

    // Empty circuit with no operations should succeed but produce no meaningful results
    // Note: The unified API requires specifying qubits, so this is actually valid now
    match result {
        Ok(_) => println!("Empty circuit succeeded"),
        Err(e) => {
            println!("Empty circuit failed with error: {e}");
            panic!("Empty circuit should succeed with unified API");
        }
    }
}

#[test]
fn test_qis_sim_large_shot_count() {
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
    let shot_vec = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(simple_ir)))
        .seed(42)
        .workers(8) // Use multiple workers for speed
        .qubits(1)
        .run(10000)
        .expect("Large shot count should work");

    let elapsed = start.elapsed();
    println!("10,000 shots completed in {:.3}s", elapsed.as_secs_f64());

    assert_eq!(shot_vec.len(), 10000);

    // Convert to ShotMap for analysis
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let r_values = get_register_i64(&shot_map, "r").expect("Should have r register");

    // Check distribution
    let ones = r_values.iter().filter(|&&v| v == 1).count();
    let ratio = f64::from(u32::try_from(ones).unwrap_or(u32::MAX)) / 10000.0;
    println!("Distribution: {:.2}% ones", ratio * 100.0);
    assert!(ratio > 0.45 && ratio < 0.55, "Should be roughly 50/50");
}

#[test]
fn test_qis_sim_multiple_registers() {
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

    let shot_vec = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(multi_reg_ir)))
        .seed(42)
        .qubits(3)
        .run(100)
        .expect("Multiple registers should work");

    // Convert to ShotMap
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let registers = shot_map.register_names();

    // Should have three registers
    assert_eq!(registers.len(), 3);
    assert!(registers.contains(&"a"));
    assert!(registers.contains(&"b"));
    assert!(registers.contains(&"c"));

    // Each should have 100 values
    assert_eq!(shot_map.num_shots(), 100);
}

#[test]
fn test_qis_sim_biased_depolarizing_noise() {
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
    let shot_vec = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(ghz_ir)))
        .seed(42)
        .noise(BiasedDepolarizingNoise { p: 0.05 }) // 5% biased noise
        .qubits(3)
        .run(1000)
        .expect("Biased noise simulation should work");

    // Convert to ShotMap and get GHZ values
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let ghz_values = get_register_i64(&shot_map, "ghz").expect("Should have ghz register");

    // Count GHZ outcomes
    let mut counts = std::collections::HashMap::new();
    for &val in &ghz_values {
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
fn test_qis_sim_file_vs_string_equivalence() {
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
    let shot_vec_string = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(llvm_ir)))
        .seed(42)
        .qubits(1)
        .run(100)
        .expect("String source should work");

    // Run from file
    let shot_vec_file = sim_builder()
        .classical(qis_engine().program(QisProgram::from_file(temp_file.path()).unwrap()))
        .seed(42)
        .qubits(1)
        .run(100)
        .expect("File source should work");

    // Convert to ShotMaps for comparison
    let shot_map_string = shot_vec_string.try_as_shot_map().expect("Should convert");
    let shot_map_file = shot_vec_file.try_as_shot_map().expect("Should convert");

    // Get x values from both
    let x_string = get_register_i64(&shot_map_string, "x").expect("Should have x register");
    let x_file = get_register_i64(&shot_map_file, "x").expect("Should have x register");

    // Results should be identical
    assert_eq!(
        x_string, x_file,
        "String and file sources should produce identical results"
    );

    // All should be 1 (X gate flips |0⟩ to |1⟩)
    assert!(
        x_string.iter().all(|&v| v == 1),
        "X gate should always produce |1⟩"
    );
}

#[test]
fn test_qis_sim_extreme_noise() {
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
    let shot_vec = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(simple_ir)))
        .seed(42)
        .noise(DepolarizingNoise { p: 0.5 }) // 50% error rate!
        .qubits(2)
        .run(1000)
        .expect("Extreme noise should still work");

    // Convert to ShotMap and get n values
    let shot_map = shot_vec
        .try_as_shot_map()
        .expect("Should convert to ShotMap");
    let n_values = get_register_i64(&shot_map, "n").expect("Should have n register");

    // Count outcomes
    let mut counts = [0; 4];
    for &val in &n_values {
        counts[usize::try_from(val).unwrap_or(0)] += 1;
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
fn test_qis_sim_builder_state_preservation() {
    if skip_if_no_llvm() {
        return;
    }

    // Build a configured simulation
    let sim = sim_builder()
        .classical(qis_engine().program(QisProgram::from_string(r#"
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.s = private unnamed_addr constant [2 x i8] c"s\00", align 1

define void @main() #0 {
call void @__quantum__qis__h__body(i64 0)
%r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.s, i32 0, i32 0))
ret void
}
attributes #0 = { "EntryPoint" }
"#)))
        .seed(12345)
    .workers(2)
    .noise(DepolarizingNoise { p: 0.1 })
    .verbose(false)
    .qubits(1)
    .build()
    .expect("Build should succeed");

    // Run multiple times - configuration should be preserved
    let mut sim = sim;
    let run1 = sim.run(50).expect("Run 1 should succeed");
    let run2 = sim.run(50).expect("Run 2 should succeed");
    let run3 = sim.run(50).expect("Run 3 should succeed");

    // All runs should have the same size
    assert_eq!(run1.len(), 50);
    assert_eq!(run2.len(), 50);
    assert_eq!(run3.len(), 50);

    // MonteCarloEngine doesn't have a stats() method anymore
    // Just verify the runs completed successfully
}

#[test]
fn test_qis_sim_zero_shots() {
    if skip_if_no_llvm() {
        return;
    }

    let simple_ir = r#"
define void @main() #0 {
ret void
}
attributes #0 = { "EntryPoint" }
"#;

    // Should panic when trying to run with 0 shots
    let result = std::panic::catch_unwind(|| {
        sim_builder()
            .classical(qis_engine().program(QisProgram::from_string(simple_ir)))
            .qubits(1)
            .run(0)
    });

    assert!(result.is_err(), "Running with 0 shots should panic");
}
