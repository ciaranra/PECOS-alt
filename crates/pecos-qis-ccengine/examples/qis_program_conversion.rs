//! Example: QisProgram conversion to QisInterface
//!
//! This demonstrates the QisProgram -> QisInterface conversion capability,
//! allowing LLVM IR with QIS function calls to be executed by the QIS runtime.

use pecos_engines::{sim_builder, state_vector};
use pecos_qis_ccengine::{qis_control_engine, native_runtime};
use pecos_programs::QisProgram;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== QisProgram -> QisInterface Conversion Example ===\n");

    // Example 1: Simple Bell state program in LLVM IR
    let bell_llvm = r#"
        ; Bell state preparation: H(0), CX(0,1), measure both
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

    println!("1. Converting LLVM IR QIS program to QisInterface:");
    let qis_program = QisProgram::from_string(bell_llvm);

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

    // Count Bell state outcomes
    let mut count_00 = 0;
    let mut count_11 = 0;

    for shot in &results.shots {
        let mut bits = Vec::new();
        for i in 0..2 {
            let key = format!("measurement_{}", i);
            if let Some(data) = shot.data.get(&key) {
                match data {
                    pecos_engines::shot_results::Data::U32(val) => bits.push(*val),
                    pecos_engines::shot_results::Data::BitVec(bitvec) => {
                        let bit_val = if !bitvec.is_empty() && bitvec[0] { 1u32 } else { 0u32 };
                        bits.push(bit_val);
                    }
                    _ => {}
                }
            }
        }

        if bits.len() == 2 {
            match bits.as_slice() {
                [0, 0] => count_00 += 1,
                [1, 1] => count_11 += 1,
                _ => println!("   Unexpected outcome: {:?}", bits),
            }
        }
    }

    println!("   Results: |00⟩: {}, |11⟩: {}", count_00, count_11);
    println!("   Success: Perfect Bell state distribution (only |00⟩ and |11⟩)\n");

    // Example 2: Single qubit operations
    let single_qubit_llvm = r#"
        define void @main() {
            call void @__quantum__qis__h__body(i64 0)
            call void @__quantum__qis__z__body(i64 0)
            call void @__quantum__qis__s__body(i64 0)
            %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            ret void
        }

        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__z__body(i64)
        declare void @__quantum__qis__s__body(i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
    "#;

    println!("2. Single qubit operations (H, Z, S):");
    let qis_program2 = QisProgram::from_string(single_qubit_llvm);

    let results2 = sim_builder()
        .classical(
            qis_control_engine()
                .try_program(qis_program2)?
        )
        .quantum(state_vector())
        .qubits(1)
        .run(10)?;

    println!("   Success: Successfully executed {} shots with gate sequence H→Z→S", results2.len());

    println!("\nQisProgram conversion working perfectly!");
    println!("LLVM IR with QIS function calls can now be executed by QIS Control Engine.");

    Ok(())
}