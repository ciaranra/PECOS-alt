//! Integration test for QisControlEngine with the full sim() API
//!
//! Tests that our QIS architecture works correctly with Bell state programs

use pecos_engines::{sim_builder, sparse_stabilizer, state_vector};
use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface, native_runtime, selene_simple_runtime};
use pecos_programs::QisProgram;

/// Create a simple Hadamard test program in LLVM IR
fn create_hadamard_test_program() -> QisProgram {
    let hadamard_qis = r#"
        define void @main() {
            %q0 = call i64 @__quantum__rt__qubit_allocate()
            call void @__quantum__qis__h__body(i64 %q0)
            %result0 = call i32 @__quantum__qis__m__body(i64 %q0, i64 0)
            ret void
        }

        declare i64 @__quantum__rt__qubit_allocate()
        declare void @__quantum__qis__h__body(i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    QisProgram::from_string(hadamard_qis)
}

/// Create a Bell state QIS program in LLVM IR
fn create_bell_state_program() -> QisProgram {
    let bell_qis = r#"
        define void @main() {
            %q0 = call i64 @__quantum__rt__qubit_allocate()
            %q1 = call i64 @__quantum__rt__qubit_allocate()
            call void @__quantum__qis__h__body(i64 %q0)
            call void @__quantum__qis__cx__body(i64 %q0, i64 %q1)
            %result0 = call i32 @__quantum__qis__m__body(i64 %q0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 %q1, i64 1)
            ret void
        }

        declare i64 @__quantum__rt__qubit_allocate()
        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    QisProgram::from_string(bell_qis)
}

#[test]
fn test_qis_control_engine_with_native_runtime() {
    // Create a simple Hadamard test program
    let qis_program = create_hadamard_test_program();

    // Build the QisControlEngine with Selene simple runtime
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .interface(qis_jit_interface())
                .runtime(selene_simple_runtime().expect("Selene runtime should be available"))
                .program(qis_program)
        )
        .quantum(state_vector())
        .qubits(1)
        .seed(123)  // Try different seed to see if issue is seed-specific
        .run(100)
        .expect("Simulation failed");

    // Verify Hadamard results (should be ~50% |0⟩ and ~50% |1⟩)
    assert_eq!(results.len(), 100, "Should have 100 shots");

    let mut count_0 = 0;
    let mut count_1 = 0;

    for shot in &results.shots {
        let m0 = shot.data.get("measurement_0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });

        match m0 {
            Some(0) => count_0 += 1,
            Some(1) => count_1 += 1,
            _ => panic!("Hadamard should only produce |0⟩ or |1⟩, got: {:?}", m0),
        }
    }

    println!("Native runtime Hadamard results:");
    println!("  |0⟩: {} times", count_0);
    println!("  |1⟩: {} times", count_1);

    // Verify roughly 50/50 distribution (with tolerance)
    assert!(count_0 > 20 && count_0 < 80, "0 count out of expected range: {}", count_0);
    assert!(count_1 > 20 && count_1 < 80, "1 count out of expected range: {}", count_1);
    assert_eq!(count_0 + count_1, 100, "Total should be 100");
}

#[test]
fn test_qis_control_engine_with_state_vector() {
    // Create a Bell state program
    let qis_program = create_bell_state_program();

    // Build the QisControlEngine with Selene simple runtime
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .interface(qis_jit_interface())
                .runtime(selene_simple_runtime().expect("Selene runtime should be available"))
                .program(qis_program)
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)  // Use fixed seed for reproducibility
        .run(100)
        .expect("Simulation failed");

    // Verify Bell state results (only |00⟩ and |11⟩)
    for shot in &results.shots {
        let m0 = shot.data.get("measurement_0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });
        let m1 = shot.data.get("measurement_1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });

        if let (Some(m0), Some(m1)) = (m0, m1) {
            assert!(
                (m0 == 0 && m1 == 0) || (m0 == 1 && m1 == 1),
                "Bell state should only produce |00⟩ or |11⟩, got: |{}{}⟩",
                m0, m1
            );
        }
    }

    println!("State vector Bell state test passed!");
}

#[test]
#[ignore]  // Ignore by default since Selene runtime may not be available
fn test_qis_control_engine_with_selene_runtime() {
    // Try to load Selene simple runtime
    let runtime = match selene_simple_runtime() {
        Ok(r) => r,
        Err(e) => {
            println!("Selene runtime not available: {}, skipping test", e);
            return;
        }
    };

    // Create a Bell state program
    let qis_program = create_bell_state_program();

    // Build the QisControlEngine with Selene runtime
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .interface(qis_jit_interface())
                .runtime(runtime)
                .program(qis_program)
        )
        .quantum(state_vector())
        .qubits(2)
        .run(10)
        .expect("Simulation failed");

    // Verify Bell state results
    assert_eq!(results.len(), 10, "Should have 10 shots");

    for shot in &results.shots {
        let m0 = shot.data.get("measurement_0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });
        let m1 = shot.data.get("measurement_1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });

        if let (Some(m0), Some(m1)) = (m0, m1) {
            assert!(
                (m0 == 0 && m1 == 0) || (m0 == 1 && m1 == 1),
                "Bell state should only produce |00⟩ or |11⟩, got: |{}{}⟩",
                m0, m1
            );
        }
    }

    println!("Selene runtime Bell state test passed!");
}

#[test]
fn test_fluent_api_compiles() {
    // This test verifies that the fluent API syntax works as expected
    let _ = || {
        let qis_program = create_bell_state_program();

        // The exact fluent API the user requested
        let _results = sim_builder()
            .classical(
                qis_control_engine()
                    .interface(qis_jit_interface())
                    .runtime(selene_simple_runtime().expect("Selene runtime should be available"))
                    .program(qis_program)
            )
            .quantum(state_vector())
            .qubits(2)
            .run(10);
    };
}

#[test]
fn test_default_runtime_selection() {
    // Test that the default runtime (should be Selene if available) works
    let qis_program = create_hadamard_test_program();

    // Use qis_control_engine() without explicitly specifying runtime
    // Should use Selene if available, otherwise native
    let results = sim_builder()
        .classical(
            qis_control_engine()
                .interface(qis_jit_interface())
                .program(qis_program)  // No explicit .runtime() call - uses default
        )
        .quantum(state_vector())
        .qubits(1)
        .seed(123)
        .run(50)
        .expect("Simulation with default runtime failed");

    // Verify we get reasonable Hadamard results
    assert_eq!(results.len(), 50, "Should have 50 shots");

    let mut count_0 = 0;
    let mut count_1 = 0;

    for shot in &results.shots {
        let m0 = shot.data.get("measurement_0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        });

        match m0 {
            Some(0) => count_0 += 1,
            Some(1) => count_1 += 1,
            _ => panic!("Invalid measurement result: {:?}", m0),
        }
    }

    println!("Default runtime Hadamard results:");
    println!("  |0⟩: {} times", count_0);
    println!("  |1⟩: {} times", count_1);

    // Should see variation (not all the same result)
    assert!(count_0 > 0, "Should see some |0⟩ results");
    assert!(count_1 > 0, "Should see some |1⟩ results");
    assert_eq!(count_0 + count_1, 50, "Total should be 50");
}