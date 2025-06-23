//! Example demonstrating all Data types supported by Shot and `ShotMap`
//!
//! This example creates shots with every Data enum variant and shows
//! how to extract each type using the corresponding try_* method.

use num_bigint::BigInt;
use pecos_core::errors::PecosError;
use pecos_engines::prelude::*;

fn main() -> Result<(), PecosError> {
    let shot_vec = create_all_types_data();
    let shot_map = shot_vec.try_as_shot_map()?;

    println!("=== Demonstrating all Data types ===");
    println!("Shots: {}", shot_map.num_shots());
    println!("Registers: {}\n", shot_map.num_registers());

    demonstrate_unsigned_integers(&shot_map);
    demonstrate_signed_integers(&shot_map);
    demonstrate_floating_point(&shot_map);
    demonstrate_other_types(&shot_map);
    demonstrate_complex_types(&shot_map);

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Create sample data with all supported types
fn create_all_types_data() -> ShotVec {
    let mut shot_vec = ShotVec::new();

    for i in 0u32..3 {
        let mut shot = Shot::default();

        // Unsigned integers - safe casts since i is 0-2
        shot.data.insert(
            "u8_val".to_string(),
            Data::U8(u8::try_from(i).expect("i < 3")),
        );
        shot.data.insert(
            "u16_val".to_string(),
            Data::U16(u16::try_from(i).expect("i < 3") * 1000),
        );
        shot.data
            .insert("u32_val".to_string(), Data::U32(i * 1_000_000));
        shot.data.insert(
            "u64_val".to_string(),
            Data::U64(u64::from(i) * 1_000_000_000),
        );

        // Signed integers - safe casts since i is 0-2
        shot.data.insert(
            "i8_val".to_string(),
            Data::I8(i8::try_from(i).expect("i < 3") - 1),
        );
        shot.data.insert(
            "i16_val".to_string(),
            Data::I16(i16::try_from(i).expect("i < 3") * 1000 - 1000),
        );
        shot.data.insert(
            "i32_val".to_string(),
            Data::I32(i32::try_from(i).expect("i < 3") * 1_000_000 - 1_000_000),
        );
        shot.data.insert(
            "i64_val".to_string(),
            Data::I64(i64::from(i) * 1_000_000_000 - 1_000_000_000),
        );

        // Floating point - precision loss is acceptable for example values
        // The cast from u32 to f32 is fine here since i is only 0-2
        #[allow(clippy::cast_precision_loss)]
        shot.data
            .insert("f32_val".to_string(), Data::F32(i as f32 * 0.1));
        shot.data.insert(
            "f64_val".to_string(),
            Data::F64(f64::from(i) * std::f64::consts::PI),
        );

        // Other basic types
        shot.data
            .insert("string_val".to_string(), Data::String(format!("shot_{i}")));
        shot.data
            .insert("bool_val".to_string(), Data::Bool(i % 2 == 0));
        shot.data.insert(
            "bigint_val".to_string(),
            Data::BigInt(BigInt::from(i) * BigInt::from(u128::MAX)),
        );

        // Binary data types - safe cast since i is 0-2
        shot.data.insert(
            "bytes_val".to_string(),
            Data::Bytes(vec![u8::try_from(i).expect("i < 3"); 4]),
        );

        // BitVec
        shot.add_register("bitvec_val", i, 8);

        // JSON
        let json_value = serde_json::json!({
            "index": i,
            "metadata": {"type": "example", "version": 1}
        });
        shot.data
            .insert("json_val".to_string(), Data::Json(json_value));

        shot_vec.shots.push(shot);
    }

    shot_vec
}

/// Demonstrate extraction of unsigned integer types
fn demonstrate_unsigned_integers(shot_map: &ShotMap) {
    println!("1. Unsigned Integer Types:");

    if let Ok(vals) = shot_map.try_u8s("u8_val") {
        println!("  U8 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_u16s("u16_val") {
        println!("  U16 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_u32s("u32_val") {
        println!("  U32 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_u64s("u64_val") {
        println!("  U64 values: {vals:?}");
    }
}

/// Demonstrate extraction of signed integer types
fn demonstrate_signed_integers(shot_map: &ShotMap) {
    println!("\n2. Signed Integer Types:");

    if let Ok(vals) = shot_map.try_i8s("i8_val") {
        println!("  I8 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_i16s("i16_val") {
        println!("  I16 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_i32s("i32_val") {
        println!("  I32 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_i64s("i64_val") {
        println!("  I64 values: {vals:?}");
    }
}

/// Demonstrate extraction of floating point types
fn demonstrate_floating_point(shot_map: &ShotMap) {
    println!("\n3. Floating Point Types:");

    if let Ok(vals) = shot_map.try_f32s("f32_val") {
        println!("  F32 values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_f64s("f64_val") {
        println!("  F64 values: {vals:?}");
        // Precision loss is acceptable here - we're calculating an average for display
        // with only 4 decimal places, and the number of shots is small (3)
        #[allow(clippy::cast_precision_loss)]
        let avg = vals.iter().sum::<f64>() / (vals.len() as f64);
        println!("  F64 average: {avg:.4}");
    }
}

/// Demonstrate extraction of other basic types
fn demonstrate_other_types(shot_map: &ShotMap) {
    println!("\n4. Other Basic Types:");

    if let Ok(vals) = shot_map.try_strings("string_val") {
        println!("  String values: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_bools("bool_val") {
        println!("  Bool values: {vals:?}");
        let true_count = vals.iter().filter(|&&b| b).count();
        println!("  True count: {}/{}", true_count, vals.len());
    }

    if let Ok(vals) = shot_map.try_bigints("bigint_val") {
        println!("  BigInt values (first): {}", vals[0]);
    }
}

/// Demonstrate extraction of complex types
fn demonstrate_complex_types(shot_map: &ShotMap) {
    println!("\n5. Complex Types:");

    // Bytes
    if let Ok(vals) = shot_map.try_bytes("bytes_val") {
        println!("  Bytes values (first): {:?}", vals[0]);
    }

    // BitVec - multiple extraction methods
    if let Ok(vals) = shot_map.try_bitvecs("bitvec_val") {
        println!("  BitVec values (count): {}", vals.len());
    }

    if let Ok(vals) = shot_map.try_bits_as_decimal("bitvec_val") {
        println!("  BitVec as decimal: {vals:?}");
    }

    if let Ok(vals) = shot_map.try_bits_as_binary("bitvec_val") {
        println!("  BitVec as binary: {vals:?}");
    }

    // JSON
    if let Ok(vals) = shot_map.try_jsons("json_val") {
        println!(
            "  JSON values (first): {}",
            serde_json::to_string(&vals[0]).unwrap_or_else(|_| "JSON error".to_string())
        );
    }
}
