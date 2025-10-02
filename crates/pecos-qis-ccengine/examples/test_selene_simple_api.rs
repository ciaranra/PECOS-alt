//! Test that selene_simple_runtime() works with the sim() API

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, selene_simple_runtime};
use pecos_programs::QisProgram;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    println!("Testing selene_simple_runtime() with sim() API...\n");

    // Test with selene_simple_runtime()
    match selene_simple_runtime() {
        Ok(runtime) => {
            println!("SUCCESS: Successfully loaded selene_simple_runtime()");

            let results = sim_builder()
                .classical(
                    qis_control_engine()
                        .runtime(runtime)
                        .try_program(qis_program)?
                )
                .quantum(state_vector())
                .qubits(2)
                .seed(42)
                .run(10)?;

            println!("SUCCESS: Simulation completed successfully!");
            println!("Got {} shots", results.len());

            // Count outcomes
            let mut count_00 = 0;
            let mut count_11 = 0;

            for shot in &results.shots {
                let m0 = shot.data.get("m0").and_then(|d| match d {
                    pecos_engines::shot_results::Data::U32(val) => Some(*val),
                    _ => None
                }).unwrap_or(99);

                let m1 = shot.data.get("m1").and_then(|d| match d {
                    pecos_engines::shot_results::Data::U32(val) => Some(*val),
                    _ => None
                }).unwrap_or(99);

                if m0 == 0 && m1 == 0 {
                    count_00 += 1;
                } else if m0 == 1 && m1 == 1 {
                    count_11 += 1;
                }
            }

            println!("\nBell state results:");
            println!("  |00⟩: {} times", count_00);
            println!("  |11⟩: {} times", count_11);

            if count_00 > 0 && count_11 > 0 {
                println!("\nSUCCESS: selene_simple_runtime() works correctly with sim() API!");
                println!("The exact syntax works:");
                println!("  sim_builder()");
                println!("    .classical(");
                println!("        qis_control_engine()");
                println!("            .runtime(selene_simple_runtime()?)");
                println!("            .try_program(qis_program)?");
                println!("    )");
                println!("    .quantum(state_vector())");
                println!("    .qubits(2)");
                println!("    .run(10)");
            } else {
                println!("\nFAILED: Bell state distribution incorrect");
            }
        }
        Err(e) => {
            println!("FAILED: Could not load selene_simple_runtime(): {}", e);
            println!("This is expected if Selene repository is not available");
        }
    }

    Ok(())
}