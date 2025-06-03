//! Comprehensive example of `ShotMap` display and formatting options
//!
//! This example demonstrates:
//! - Different `BitVec` display formats (binary, decimal, hex, bool array)
//! - Simple API for common formats (.`bitvec_binary()`, .`bitvec_decimal()`, .`bitvec_hex()`)
//! - Custom display options
//! - How formatting only affects `BitVec` types while other types remain unchanged

use pecos_engines::{
    BitVecDisplayFormat, ShotMapDisplayExt, ShotMapDisplayOptions,
    shot_results::{Data, Shot, ShotVec},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a ShotVec with mixed data types to show formatting behavior
    let mut shot_vec = ShotVec::new();

    for i in 0..3 {
        let mut shot = Shot::default();

        // Add BitVec data - this will be affected by formatting options
        shot.add_register("qubits", 5 + i, 3); // 3-bit register
        shot.add_register("ancilla", i % 2, 1); // 1-bit register

        // Add other data types - these are NOT affected by BitVec formatting
        shot.data.insert("count".to_string(), Data::U32(i));
        shot.data
            .insert("phase".to_string(), Data::F64(0.25 * f64::from(i)));
        shot.data
            .insert("label".to_string(), Data::String(format!("shot_{i}")));

        shot_vec.shots.push(shot);
    }

    let shot_map = shot_vec.try_as_shot_map()?;

    println!("=== Default Display (Decimal) ===");
    println!("{}", shot_map.display());

    println!("\n=== Binary Format (Simple API) ===");
    println!("{}", shot_map.display().bitvec_binary());

    println!("\n=== Hexadecimal Format (Simple API) ===");
    println!("{}", shot_map.display().bitvec_hex());

    println!("\n=== Boolean Array Format ===");
    println!("{}", shot_map.display().bitvec_bool_array());

    println!("\n=== Using BitVecDisplayFormat Enum ===");
    println!(
        "{}",
        shot_map
            .display()
            .bitvec_format(BitVecDisplayFormat::Binary)
    );

    println!("\n=== Custom Display Options ===");
    let custom_options = ShotMapDisplayOptions {
        bitvec_format: BitVecDisplayFormat::Hexadecimal,
        max_shots: Some(2),
        sort_registers: true,
        indent: "    ".to_string(),
    };
    println!("{}", shot_map.display_with(custom_options));

    println!("\n=== Chained Options ===");
    println!(
        "{}",
        shot_map.display().bitvec_hex().max_shots(2).indent("  -> ")
    );

    println!("\n=== Note: Non-BitVec Types Unchanged ===");
    println!("Notice how 'count', 'phase', and 'label' display the same way");
    println!("regardless of the BitVec format setting. Only BitVec data changes.");

    Ok(())
}
