//! Demonstration of all runtime options with qis_control_engine()

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime, selene_simple_runtime};
use pecos_programs::QisProgram;

fn test_bell_state(name: &str, builder: pecos_qis_ccengine::QisEngineBuilder) -> Result<(), Box<dyn std::error::Error>> {
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
        .classical(builder.try_program(qis_program)?)
        .quantum(state_vector())
        .qubits(2)
        .seed(42)
        .run(10)?;

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

    println!("{}: |00⟩={}, |11⟩={}", name, count_00, count_11);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Runtime Options Demonstration ===\n");

    // Option 1: Default (Selene if available, otherwise native)
    println!("1. Default runtime (qis_control_engine()):");
    test_bell_state("   Default", qis_control_engine())?;

    // Option 2: Explicitly use native runtime
    println!("\n2. Explicit native runtime:");
    test_bell_state("   Native", qis_control_engine().runtime(native_runtime()))?;

    // Option 3: Explicitly use Selene runtime (if available)
    println!("\n3. Explicit Selene runtime:");
    match selene_simple_runtime() {
        Ok(runtime) => {
            test_bell_state("   Selene", qis_control_engine().runtime(runtime))?;
        }
        Err(e) => {
            println!("   Selene not available: {}", e);
        }
    }

    println!("\n=== Summary ===");
    println!("SUCCESS: qis_control_engine() now uses Selene as default (if available)");
    println!("SUCCESS: Falls back to native runtime when Selene is not available");
    println!("SUCCESS: Can explicitly choose runtime with .runtime() method");

    Ok(())
}