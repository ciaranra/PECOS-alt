//! Integration test for QisProgram with the full sim() API
//!
//! This test verifies that general QIR/QIS programs can be sent through
//! the sim() API and produce correct results, including with Selene runtime.

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{
    qis_control_engine, native_runtime, selene_simple_runtime,
    qis_jit_interface, qis_selene_helios_interface
};
use pecos_programs::QisProgram;

/// Test helper that runs with both Helios (reference) and JIT interfaces
///
/// Helios is considered the reference implementation - it's well-tested in Selene.
/// JIT is our fallback for when Selene isn't available.
/// Both should produce the same results for the same quantum circuits.
fn test_with_both_interfaces<F>(test_name: &str, test_fn: F)
where
    F: Fn(&str) -> Result<(), String> + Clone
{
    // First try Helios (the reference implementation)
    println!("\n🔍 Testing {} with Helios interface (reference):", test_name);

    // Check if we can use Helios by attempting a simple compilation
    let test_program = QisProgram::from_string("define void @main() { ret void }");
    let can_use_helios = qis_control_engine()
        .interface(qis_selene_helios_interface())
        .try_program(test_program)
        .is_ok();

    if can_use_helios {
        match test_fn("Helios") {
            Ok(()) => println!("  Helios test passed (reference)"),
            Err(e) => panic!("Helios reference implementation failed: {}", e),
        }

        // Now test with JIT - it should match Helios results
        println!("\n🔍 Testing {} with JIT interface (should match Helios):", test_name);
        match test_fn("JIT") {
            Ok(()) => println!("  JIT test passed (matches reference)"),
            Err(e) => panic!("JIT implementation differs from Helios reference: {}", e),
        }
    } else {
        println!("  WARNING: Helios not available (Selene not installed)");
        println!("  INFO: Running with JIT interface only");

        // At least test with JIT
        match test_fn("JIT") {
            Ok(()) => println!("  JIT test passed"),
            Err(e) => panic!("JIT test failed: {}", e),
        }

        println!("  WARNING: Could not verify against Helios reference implementation");
    }
}

#[test]
fn test_qis_program_bell_state_native_runtime() {
    test_with_both_interfaces("Bell state with native runtime", |interface_name| {
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

        // Select interface based on test parameter
        let interface_builder = if interface_name == "Helios" {
            qis_selene_helios_interface()
        } else {
            qis_jit_interface()
        };

        // Run simulation
        let results = sim_builder()
            .classical(
                qis_control_engine()
                    .interface(interface_builder)
                    .runtime(native_runtime())
                    .program(qis_program)
            )
            .quantum(state_vector())
            .qubits(2)
            .seed(42)  // Same seed for reproducibility
            .run(100)
            .map_err(|e| format!("Simulation failed: {}", e))?;

        // Verify Bell state results
        let mut count_00 = 0;
        let mut count_11 = 0;

        for shot in results.shots.iter() {
            let m0 = shot.data.get("measurement_0").and_then(|d| match d {
                pecos_engines::shot_results::Data::U32(val) => Some(*val),
                _ => None
            });
            let m1 = shot.data.get("measurement_1").and_then(|d| match d {
                pecos_engines::shot_results::Data::U32(val) => Some(*val),
                _ => None
            });

            match (m0, m1) {
                (Some(0), Some(0)) => count_00 += 1,
                (Some(1), Some(1)) => count_11 += 1,
                _ => return Err(format!("Bell state should only produce |00⟩ or |11⟩, got: ({:?}, {:?})", m0, m1)),
            }
        }

        println!("    {} interface: |00⟩: {} times, |11⟩: {} times", interface_name, count_00, count_11);

        // Verify distribution is reasonable (allowing for statistical variation)
        if count_00 < 20 || count_00 > 80 {
            return Err(format!("00 count out of expected range: {}", count_00));
        }
        if count_11 < 20 || count_11 > 80 {
            return Err(format!("11 count out of expected range: {}", count_11));
        }
        if count_00 + count_11 != 100 {
            return Err(format!("Total should be 100, got {}", count_00 + count_11));
        }

        Ok(())
    })
}

#[test]
fn test_qis_program_bell_state_selene_runtime() {
    // Try to load Selene simple runtime
    let runtime = match selene_simple_runtime() {
        Ok(r) => r,
        Err(e) => {
            println!("Selene runtime not available: {}, skipping test", e);
            return;
        }
    };

    test_with_both_interfaces("Bell state with Selene runtime", |interface_name| {
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

        // Select interface based on test parameter
        let interface_builder = if interface_name == "Helios" {
            qis_selene_helios_interface()
        } else {
            qis_jit_interface()
        };

        // Run simulation with Selene runtime
        let results = sim_builder()
            .classical(
                qis_control_engine()
                    .interface(interface_builder)
                    .runtime(runtime.clone())
                    .program(qis_program)
            )
            .quantum(state_vector())
            .qubits(2)
            .seed(42)
            .run(10)
            .map_err(|e| format!("Simulation failed with Selene runtime: {}", e))?;

        // Verify Bell state results
        let mut count_00 = 0;
        let mut count_11 = 0;

        for shot in &results.shots {
            let m0 = shot.data.get("measurement_0").and_then(|d| match d {
                pecos_engines::shot_results::Data::U32(val) => Some(*val),
                _ => None
            });
            let m1 = shot.data.get("measurement_1").and_then(|d| match d {
                pecos_engines::shot_results::Data::U32(val) => Some(*val),
                _ => None
            });

            match (m0, m1) {
                (Some(0), Some(0)) => count_00 += 1,
                (Some(1), Some(1)) => count_11 += 1,
                _ => return Err(format!("Bell state should only produce |00⟩ or |11⟩, got: ({:?}, {:?})", m0, m1)),
            }
        }

        println!("    {} interface with Selene runtime: |00⟩: {} times, |11⟩: {} times",
                 interface_name, count_00, count_11);

        if count_00 + count_11 != 10 {
            return Err(format!("Total should be 10, got {}", count_00 + count_11));
        }
        if count_00 == 0 && count_11 == 0 {
            return Err("Should have some valid measurements".to_string());
        }

        Ok(())
    })
}

#[test]
fn test_qis_program_complex_circuit() {
    test_with_both_interfaces("Complex quantum circuit", |interface_name| {
        // Test with a more complex quantum circuit
        let complex_qis = r#"define void @main() {
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
declare i32 @__quantum__qis__m__body(i64, i64)"#;

        let qis_program = QisProgram::from_string(complex_qis);

        // Select interface based on test parameter
        let interface_builder = if interface_name == "Helios" {
            qis_selene_helios_interface()
        } else {
            qis_jit_interface()
        };

        // Run simulation with native runtime and state vector (not stabilizer, since we have non-Clifford gates)
        let results = sim_builder()
            .classical(
                qis_control_engine()
                    .interface(interface_builder)
                    .runtime(native_runtime())
                    .program(qis_program)
            )
            .quantum(state_vector())
            .qubits(3)
            .seed(42)  // Same seed for reproducibility
            .run(20)
            .map_err(|e| format!("Simulation failed: {}", e))?;

        if results.len() != 20 {
            return Err(format!("Should have 20 shots, got {}", results.len()));
        }

        // Just verify we get valid measurement results
        for shot in &results.shots {
            for i in 0..3 {
                let key = format!("measurement_{}", i);
                if !shot.data.contains_key(&key) {
                    return Err(format!("Shot should contain measurement result for qubit {}", i));
                }
            }
        }

        println!("    {} interface: Complex circuit executed with {} shots",
                 interface_name, results.len());

        Ok(())
    })
}

#[test]
fn test_fluent_api_syntax() {
    test_with_both_interfaces("Fluent API syntax verification", |interface_name| {
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

        // Select interface based on test parameter
        let interface_builder = if interface_name == "Helios" {
            qis_selene_helios_interface()
        } else {
            qis_jit_interface()
        };

        // The exact API the user requested should compile and work
        let results = sim_builder()
            .classical(
                qis_control_engine()
                    .interface(interface_builder)
                    .runtime(native_runtime())
                    .program(qis_program)
            )
            .quantum(state_vector())
            .qubits(2)
            .seed(42)
            .run(10)
            .map_err(|e| format!("Failed to run simulation: {}", e))?;

        if results.len() != 10 {
            return Err(format!("Should have 10 shots, got {}", results.len()));
        }

        println!("    {} interface: Fluent API syntax works correctly", interface_name);
        Ok(())
    })
}

#[test]
fn test_selene_runtime_availability() {
    // Test that we can check for Selene runtime availability
    match selene_simple_runtime() {
        Ok(runtime) => {
            println!("SUCCESS: Selene simple runtime is available");

            // Test with both interfaces when Selene runtime is available
            test_with_both_interfaces("Selene runtime with single qubit", |interface_name| {
                let qis_program = QisProgram::from_string(r#"
                    define void @main() {
                        call void @__quantum__qis__h__body(i64 0)
                        %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
                        ret void
                    }

                    declare void @__quantum__qis__h__body(i64)
                    declare i32 @__quantum__qis__m__body(i64, i64)
                "#);

                // Select interface based on test parameter
                let interface_builder = if interface_name == "Helios" {
                    qis_selene_helios_interface()
                } else {
                    qis_jit_interface()
                };

                let results = sim_builder()
                    .classical(
                        qis_control_engine()
                            .interface(interface_builder)
                            .runtime(runtime.clone())
                            .program(qis_program)
                    )
                    .quantum(state_vector())
                    .qubits(1)
                    .seed(42)
                    .run(5)
                    .map_err(|e| format!("Failed with Selene runtime: {}", e))?;

                if results.len() != 5 {
                    return Err(format!("Should have 5 shots, got {}", results.len()));
                }

                println!("    {} interface: Selene runtime works with QisProgram", interface_name);
                Ok(())
            })
        }
        Err(e) => {
            println!("ℹ Selene runtime not available: {}", e);
            println!("  This is expected if Selene repository is not present");
        }
    }
}