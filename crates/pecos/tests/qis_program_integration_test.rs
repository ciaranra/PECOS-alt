//! Integration test for QisProgram with the full sim() API
//!
//! This test verifies that general QIR/QIS programs can be sent through
//! the sim() API and produce correct results, including with Selene runtime.

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime, selene_simple_runtime};
use pecos_programs::QisProgram;

#[test]
fn test_qis_program_bell_state_native_runtime() {
    // Create a Bell state QIS program in LLVM IR
    let bell_qis = r#"
        define void @main() {
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__cx__body(i64 0, i64 1)
            %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    let qis_program = QisProgram::from_string(bell_qis);

    // Run simulation with native runtime
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .try_program(qis_program)
                .expect("Failed to load QIS program")
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(100)
        .expect("Simulation failed");

    // Verify Bell state results
    let mut count_00 = 0;
    let mut count_11 = 0;

    for (idx, shot) in results.shots.iter().enumerate() {
        println!("Shot {}: data = {:?}", idx, shot.data);

        let m0 = shot.data.get("m0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });
        let m1 = shot.data.get("m1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });

        println!("  Extracted: m0={:?}, m1={:?}", m0, m1);

        match (m0, m1) {
            (Some(0), Some(0)) => count_00 += 1,
            (Some(1), Some(1)) => count_11 += 1,
            _ => panic!("Bell state should only produce |00⟩ or |11⟩, got: ({:?}, {:?})", m0, m1),
        }
    }

    println!("QisProgram Bell state with native runtime:");
    println!("  |00⟩: {} times", count_00);
    println!("  |11⟩: {} times", count_11);

    assert!(count_00 > 20 && count_00 < 80, "00 count out of expected range: {}", count_00);
    assert!(count_11 > 20 && count_11 < 80, "11 count out of expected range: {}", count_11);
    assert_eq!(count_00 + count_11, 100, "Total should be 100");
}

#[test]
#[ignore] // Ignore by default since Selene runtime may not be available
fn test_qis_program_bell_state_selene_runtime() {
    // Try to load Selene simple runtime
    let runtime = match selene_simple_runtime() {
        Ok(r) => r,
        Err(e) => {
            println!("Selene runtime not available: {}, skipping test", e);
            return;
        }
    };

    // Create a Bell state QIS program
    let bell_qis = r#"
        define void @main() {
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__cx__body(i64 0, i64 1)
            %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    let qis_program = QisProgram::from_string(bell_qis);

    // Run simulation with Selene runtime
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(runtime)
                .try_program(qis_program)
                .expect("Failed to load QIS program")
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(10)
        .expect("Simulation failed with Selene runtime");

    // Verify Bell state results
    let mut count_00 = 0;
    let mut count_11 = 0;

    for shot in &results.shots {
        let m0 = shot.data.get("m0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });
        let m1 = shot.data.get("m1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });

        match (m0, m1) {
            (Some(0), Some(0)) => count_00 += 1,
            (Some(1), Some(1)) => count_11 += 1,
            _ => panic!("Bell state should only produce |00⟩ or |11⟩"),
        }
    }

    println!("QisProgram Bell state with Selene runtime:");
    println!("  |00⟩: {} times", count_00);
    println!("  |11⟩: {} times", count_11);

    assert!(count_00 > 0 || count_11 > 0, "Should have some valid measurements");
    assert_eq!(count_00 + count_11, 10, "Total should be 10");
}

#[test]
fn test_qis_program_complex_circuit() {
    // Test with a more complex quantum circuit
    let complex_qis = r#"
        define void @main() {
            ; Three qubit GHZ state
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__cx__body(i64 0, i64 1)
            call void @__quantum__qis__cx__body(i64 1, i64 2)

            ; Apply some single qubit gates
            call void @__quantum__qis__s__body(i64 0)
            call void @__quantum__qis__t__body(i64 1)
            call void @__quantum__qis__z__body(i64 2)

            ; Measure all qubits
            %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
            %result2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare void @__quantum__qis__s__body(i64)
        declare void @__quantum__qis__t__body(i64)
        declare void @__quantum__qis__z__body(i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    let qis_program = QisProgram::from_string(complex_qis);

    // Run simulation with native runtime and state vector (not stabilizer, since we have non-Clifford gates)
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .try_program(qis_program)
                .expect("Failed to load complex QIS program")
        )
        .quantum(state_vector())
        .qubits(3)
        .run(20)
        .expect("Simulation failed");

    assert_eq!(results.len(), 20, "Should have 20 shots");

    // Just verify we get valid measurement results
    for shot in &results.shots {
        for i in 0..3 {
            let key = format!("m{}", i);
            assert!(
                shot.data.contains_key(&key),
                "Shot should contain measurement result for qubit {}",
                i
            );
        }
    }

    println!("Complex QIS circuit executed successfully with {} shots", results.len());
}

#[test]
fn test_fluent_api_syntax() {
    // Verify the exact syntax requested by the user works
    let qis_program = QisProgram::from_string(r#"
        define void @main() {
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__cx__body(i64 0, i64 1)
            %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#);

    // The exact API the user requested should compile and work
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .try_program(qis_program)
                .expect("Failed to load QIS program")
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(10)
        .expect("Failed to run simulation");

    assert_eq!(results.len(), 10, "Should have 10 shots");
    println!("Fluent API syntax works correctly!");
}

#[test]
#[ignore] // Ignore since Selene may not be available
fn test_selene_runtime_availability() {
    // Test that we can check for Selene runtime availability
    match selene_simple_runtime() {
        Ok(_) => {
            println!("✓ Selene simple runtime is available");

            // Also test the fluent API with Selene
            let qis_program = QisProgram::from_string(r#"
                define void @main() {
                    call void @__quantum__qis__h__body(i64 0)
                    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
                    ret void
                }

                declare void @__quantum__qis__h__body(i64)
                declare i32 @__quantum__qis__m__body(i64, i64)
            "#);

            let results = sim_builder()
                .classical(
                    qis_control_engine()
                        .runtime(selene_simple_runtime().unwrap())
                        .try_program(qis_program)
                        .expect("Failed to load QIS program")
                )
                .quantum(state_vector())
                .qubits(1)
                .run(5)
                .expect("Failed with Selene runtime");

            println!("✓ Selene runtime works with QisProgram");
            assert_eq!(results.len(), 5);
        }
        Err(e) => {
            println!("ℹ Selene runtime not available: {}", e);
            println!("  This is expected if Selene repository is not present");
        }
    }
}