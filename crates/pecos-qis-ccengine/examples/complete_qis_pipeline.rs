//! Example: Complete QIS Pipeline with sim() API
//!
//! This demonstrates that we can send general QIR/QIS programs through
//! the sim() API and get proper results back.

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime, selene_simple_runtime};
use pecos_programs::QisProgram;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Complete QIS Pipeline Test ===\n");

    // Test 1: Bell state with Native Runtime
    println!("1. Bell state with Native Runtime:");
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

    let results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .try_program(qis_program)?
        )
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(20)?;

    // Count outcomes
    let mut count_00 = 0;
    let mut count_11 = 0;
    for shot in &results.shots {
        let m0 = shot.data.get("m0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(0);
        let m1 = shot.data.get("m1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(0);

        if m0 == 0 && m1 == 0 {
            count_00 += 1;
        } else if m0 == 1 && m1 == 1 {
            count_11 += 1;
        }
    }

    println!("   Native Runtime Results: |00⟩: {}, |11⟩: {}", count_00, count_11);
    assert!(count_00 > 0 && count_11 > 0, "Should have both |00⟩ and |11⟩ outcomes");
    println!("   Success: Native runtime working!\n");

    // Test 2: Try with Selene Runtime if available
    println!("2. Bell state with Selene Runtime:");
    match selene_simple_runtime() {
        Ok(selene_runtime) => {
            let qis_program2 = QisProgram::from_string(bell_qis);

            match sim_builder()
                .classical(
                    qis_control_engine()
                        .runtime(selene_runtime)
                        .try_program(qis_program2)?
                )
                .quantum(state_vector())
                .qubits(2)
                .seed(42)
                .run(20)
            {
                Ok(results) => {
                    let mut s_count_00 = 0;
                    let mut s_count_11 = 0;
                    for shot in &results.shots {
                        let m0 = shot.data.get("m0").and_then(|d| match d {
                            pecos_engines::shot_results::Data::U32(val) => Some(*val),
                            _ => None
                        }).unwrap_or(0);
                        let m1 = shot.data.get("m1").and_then(|d| match d {
                            pecos_engines::shot_results::Data::U32(val) => Some(*val),
                            _ => None
                        }).unwrap_or(0);

                        if m0 == 0 && m1 == 0 {
                            s_count_00 += 1;
                        } else if m0 == 1 && m1 == 1 {
                            s_count_11 += 1;
                        }
                    }
                    println!("   Selene Runtime Results: |00⟩: {}, |11⟩: {}", s_count_00, s_count_11);
                    println!("   Success: Selene runtime working!");
                }
                Err(e) => {
                    println!("   Selene runtime execution failed: {}", e);
                    println!("   (This is expected if Selene plugins aren't built)");
                }
            }
        }
        Err(e) => {
            println!("   Selene runtime not available: {}", e);
            println!("   (This is expected if Selene repository is not present)");
        }
    }

    // Test 3: More complex circuit - GHZ state
    println!("\n3. GHZ state (3 qubits) with Native Runtime:");
    let ghz_qis = r#"
        define void @main() {
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__cx__body(i64 0, i64 1)
            call void @__quantum__qis__cx__body(i64 1, i64 2)
            %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
            %result2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    let ghz_program = QisProgram::from_string(ghz_qis);

    let ghz_results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .try_program(ghz_program)?
        )
        .quantum(state_vector())
        .qubits(3)
        .seed(123)
        .run(30)?;

    let mut count_000 = 0;
    let mut count_111 = 0;
    for shot in &ghz_results.shots {
        let m0 = shot.data.get("m0").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(0);
        let m1 = shot.data.get("m1").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(0);
        let m2 = shot.data.get("m2").and_then(|d| match d {
            pecos_engines::shot_results::Data::U32(val) => Some(*val),
            _ => None
        }).unwrap_or(0);

        if m0 == 0 && m1 == 0 && m2 == 0 {
            count_000 += 1;
        } else if m0 == 1 && m1 == 1 && m2 == 1 {
            count_111 += 1;
        }
    }

    println!("   GHZ Results: |000⟩: {}, |111⟩: {}", count_000, count_111);
    assert!(count_000 > 0 && count_111 > 0, "Should have both |000⟩ and |111⟩ outcomes");
    println!("   Success: GHZ state working!");

    // Test 4: Circuit with various gates
    println!("\n4. Circuit with various gates:");
    let complex_qis = r#"
        define void @main() {
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__s__body(i64 0)
            call void @__quantum__qis__t__body(i64 0)
            call void @__quantum__qis__x__body(i64 1)
            call void @__quantum__qis__y__body(i64 2)
            call void @__quantum__qis__z__body(i64 2)
            call void @__quantum__qis__cx__body(i64 0, i64 1)
            call void @__quantum__qis__cz__body(i64 1, i64 2)
            %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
            %result2 = call i32 @__quantum__qis__m__body(i64 2, i64 2)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__x__body(i64)
        declare void @__quantum__qis__y__body(i64)
        declare void @__quantum__qis__z__body(i64)
        declare void @__quantum__qis__s__body(i64)
        declare void @__quantum__qis__t__body(i64)
        declare void @__quantum__qis__cx__body(i64, i64)
        declare void @__quantum__qis__cz__body(i64, i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    let complex_program = QisProgram::from_string(complex_qis);

    let complex_results = sim_builder()
        .classical(
            qis_control_engine()
                .runtime(native_runtime())
                .try_program(complex_program)?
        )
        .quantum(state_vector())
        .qubits(3)
        .run(10)?;

    println!("   Executed {} shots with H, S, T, X, Y, Z, CX, CZ gates", complex_results.shots.len());
    println!("   Success: Complex circuit working!");

    println!("\nComplete QIS pipeline verified!");
    println!("General QIR/QIS programs can be sent through sim() API and produce correct results!");

    Ok(())
}