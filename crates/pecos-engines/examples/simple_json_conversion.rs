// Example showing the simplified JSON conversion for ShotVec

use bitvec::prelude::*;
use pecos_engines::shot_results::{Data, Shot, ShotVec};

fn main() {
    // Create a ShotVec with various data types
    let mut shot_vec = ShotVec::new();

    for i in 0..3 {
        let mut shot = Shot::default();

        // Integer types - will be decimal numbers in JSON
        shot.data.insert("count".to_string(), Data::U32(i));

        // BitVec - will be converted to decimal
        let mut bv = BitVec::<u8, Lsb0>::new();
        bv.push(true); // bit 0
        bv.push(i % 2 == 0); // bit 1
        shot.data.insert("bits".to_string(), Data::BitVec(bv));

        // Float - will be a number
        shot.data
            .insert("phase".to_string(), Data::F64(f64::from(i) * 0.5));

        // String - will stay as string
        shot.data
            .insert("label".to_string(), Data::String(format!("shot_{i}")));

        shot_vec.shots.push(shot);
    }

    // The simple conversion - using compact JSON format
    let json = shot_vec.to_compact_json();

    println!("Simple JSON conversion:");
    println!("{json}");

    // Show how the Data::to_value_string() method works
    println!("\nDemonstrating to_value_string():");
    println!("U32(42): {}", Data::U32(42).to_value_string());

    let mut bv = BitVec::<u8, Lsb0>::new();
    bv.push(true);
    bv.push(false);
    bv.push(true);
    println!("BitVec(101): {}", Data::BitVec(bv).to_value_string()); // Should be 5

    println!(
        "F64(3.14): {}",
        Data::F64(std::f64::consts::PI).to_value_string()
    );
    println!(
        "String(\"hello\"): {}",
        Data::String("hello".to_string()).to_value_string()
    );
}
