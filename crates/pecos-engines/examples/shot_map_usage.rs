// Copyright 2025 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! Example demonstrating various ways to access and use data in a `ShotMap`.
//!
//! This example shows:
//! 1. Using the `get()` method and pattern matching on `DataVec`
//! 2. Using the extract methods for specific types
//! 3. Direct access to typed vectors via pattern matching
//! 4. Iterating over values
//! 5. Working with `BitVec` data specifically

use bitvec::prelude::*;
use pecos_core::errors::PecosError;
use pecos_engines::{Data, DataVec, Shot, ShotMap, ShotVec};
use std::collections::BTreeMap;

fn main() -> Result<(), PecosError> {
    let shot_vec = create_sample_data();
    let shot_map = shot_vec.try_as_shot_map()?;

    println!("=== ShotMap Access Examples ===");
    println!("Shots: {}", shot_map.num_shots());
    println!("Registers: {:?}\n", shot_map.register_names());

    demonstrate_get_method(&shot_map);
    demonstrate_extract_methods(&shot_map);
    demonstrate_bitvec_operations(&shot_map);
    demonstrate_analysis(&shot_map);
    demonstrate_json_conversion(&shot_map);

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Create sample quantum measurement data
fn create_sample_data() -> ShotVec {
    let mut shot_vec = ShotVec::new();

    for i in 0..10 {
        let mut shot = Shot::default();
        let i_u32 = u32::try_from(i).expect("index fits in u32");

        // Qubit measurements as BitVec
        let mut qubits = BitVec::<u8, Lsb0>::new();
        qubits.push(i % 2 == 0);
        qubits.push(i % 3 == 0);
        qubits.push(i % 5 == 0);
        shot.data.insert("qubits".to_string(), Data::BitVec(qubits));

        // Classical register with specified width
        shot.add_register("creg", i_u32, 4);

        // Pure U32 value
        shot.data.insert("counter".to_string(), Data::U32(i_u32));

        // Phase measurement
        shot.data.insert(
            "phase".to_string(),
            Data::F64(f64::from(i) * 0.1 * std::f64::consts::PI),
        );

        // Success flag
        shot.data
            .insert("success".to_string(), Data::Bool(i % 4 == 0));

        shot_vec.shots.push(shot);
    }

    shot_vec
}

/// Demonstrate using `get()` method with pattern matching
fn demonstrate_get_method(shot_map: &ShotMap) {
    println!("1. Using get() with pattern matching:");
    if let Some(data_vec) = shot_map.get("qubits") {
        match data_vec {
            DataVec::BitVec(bitvecs) => {
                println!("  Found BitVec data with {} measurements", bitvecs.len());
                for (i, bv) in bitvecs.iter().take(3).enumerate() {
                    println!("    Shot {i}: {bv:?}");
                }
            }
            _ => println!("  Unexpected data type"),
        }
    }
}

/// Demonstrate type-specific extract methods
fn demonstrate_extract_methods(shot_map: &ShotMap) {
    println!("\n2. Using extract methods:");

    // Extract U32 values
    match shot_map.try_u32s("counter") {
        Ok(counters) => {
            println!("  Counters: {counters:?}");
            #[allow(clippy::cast_precision_loss)]
            let avg = f64::from(counters.iter().sum::<u32>()) / (counters.len() as f64);
            println!("  Average counter value: {avg:.2}");
        }
        Err(e) => println!("  Error extracting counters: {e}"),
    }

    // Extract F64 values
    if let Ok(phases) = shot_map.try_f64s("phase") {
        #[allow(clippy::cast_precision_loss)]
        let avg_phase = phases.iter().sum::<f64>() / (phases.len() as f64);
        println!("  Average phase: {avg_phase:.4} radians");
    }

    // Extract Bool values
    if let Ok(success_flags) = shot_map.try_bools("success") {
        let success_count = success_flags.iter().filter(|&&x| x).count();
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let success_rate = (f64::from(success_count as u32) / (success_flags.len() as f64)) * 100.0;
        println!("  Success rate: {success_rate:.1}%");
    }
}

/// Demonstrate BitVec-specific operations
fn demonstrate_bitvec_operations(shot_map: &ShotMap) {
    println!("\n3. BitVec-specific operations:");

    // Get BitVec data multiple ways
    if let Ok(qubit_vecs) = shot_map.try_bitvecs("qubits") {
        println!("  Direct BitVec access: {} measurements", qubit_vecs.len());
    }

    // Convert to decimal values
    if let Ok(decimal_values) = shot_map.try_bits_as_decimal("qubits") {
        println!("  As decimal: {:?}", &decimal_values[..3]);
    }

    // Convert to binary strings
    if let Ok(binary_strings) = shot_map.try_bits_as_binary("qubits") {
        println!("  As binary: {:?}", &binary_strings[..3]);
    }

    // Convert to hex strings
    if let Ok(hex_strings) = shot_map.try_bits_as_hex("qubits") {
        println!("  As hex: {:?}", &hex_strings[..3]);
    }
}

/// Demonstrate measurement outcome analysis
fn demonstrate_analysis(shot_map: &ShotMap) {
    println!("\n4. Analyzing measurement outcomes:");

    // Create measurement histogram
    if let Ok(measurements) = shot_map.try_bits_as_decimal("creg") {
        let mut histogram = BTreeMap::new();
        for value in &measurements {
            *histogram.entry(value.clone()).or_insert(0) += 1;
        }

        println!("  Measurement histogram:");
        let mut sorted: Vec<_> = histogram.into_iter().collect();
        sorted.sort_by_key(|(k, _)| k.clone());
        for (outcome, count) in sorted.iter().take(5) {
            println!("    |{outcome}⟩: {count} times");
        }
    }

    // Analyze individual qubit statistics
    if let Ok(qubit_vecs) = shot_map.try_bitvecs("qubits") {
        println!("\n  Individual qubit statistics:");
        for qubit_idx in 0..3 {
            let ones_count = qubit_vecs
                .iter()
                .filter(|bv| bv.get(qubit_idx).is_some_and(|b| *b))
                .count();
            #[allow(clippy::cast_precision_loss)]
            let prob = (ones_count as f64) / (qubit_vecs.len() as f64);
            println!("    Qubit {qubit_idx}: P(|1⟩) = {prob:.3}");
        }
    }
}

/// Demonstrate JSON conversion capabilities
fn demonstrate_json_conversion(shot_map: &ShotMap) {
    println!("\n5. JSON conversion:");

    let json = shot_map.to_json();
    println!(
        "  Standard JSON (truncated): {}",
        serde_json::to_string(&json)
            .unwrap()
            .chars()
            .take(100)
            .collect::<String>()
    );

    let simple_json = shot_map.to_simple_json();
    println!(
        "  Simple JSON (truncated): {}",
        serde_json::to_string(&simple_json)
            .unwrap()
            .chars()
            .take(100)
            .collect::<String>()
    );
}
