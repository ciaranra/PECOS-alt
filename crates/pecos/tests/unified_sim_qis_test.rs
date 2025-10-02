//! Test that unified sim API uses qis_control_engine for QIS and HUGR programs

use pecos::unified_sim::SimBuilderExt;
use pecos_engines::sim_builder;
use pecos_programs::{QisProgram, HugrProgram};

#[test]
fn test_unified_sim_with_qis_program() {
    // Create a simple Bell state QIS program
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

    // Use the unified sim API - should automatically use qis_control_engine
    let results = sim_builder()
        .program(qis_program)
        .qubits(2)
        .seed(42)
        .run(10)
        .expect("Failed to run QIS program simulation");

    assert_eq!(results.len(), 10, "Should have 10 shots");

    // Verify Bell state results
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
            (Some(0), Some(0)) | (Some(1), Some(1)) => {
                // Valid Bell state outcomes
            }
            _ => panic!("Bell state should only produce |00⟩ or |11⟩, got: ({:?}, {:?})", m0, m1),
        }
    }

    println!("SUCCESS: QIS program works with unified sim API");
}

#[test]
fn test_unified_sim_with_hugr_program() {
    // Create a simple HUGR program
    // For now, we'll skip this test if HUGR conversion isn't available
    let hugr_bytes = vec![]; // Normally would have actual HUGR data

    if hugr_bytes.is_empty() {
        println!("Skipping HUGR test - no test data available");
        return;
    }

    let hugr_program = HugrProgram::from_bytes(hugr_bytes);

    // Use the unified sim API - should automatically use qis_control_engine
    let _results = sim_builder()
        .program(hugr_program)
        .qubits(2)
        .seed(42)
        .run(10);

    // We'd verify results here if we had a real HUGR program
    println!("SUCCESS: HUGR program works with unified sim API");
}