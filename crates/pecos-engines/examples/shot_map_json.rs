use pecos_core::errors::PecosError;
use pecos_engines::prelude::*;

fn main() -> Result<(), PecosError> {
    // Create a ShotVec with some quantum measurement results
    let mut shot_vec = ShotVec::new();

    // Simulate 5 shots
    for i in 0..5 {
        let mut shot = Shot::default();

        // Quantum measurements
        shot.add_register("q0", i % 2, 1);
        shot.add_register("q1", (i / 2) % 2, 1);

        // Additional data of different types
        shot.data.insert("iteration".to_string(), Data::U32(i));
        shot.data
            .insert("success".to_string(), Data::Bool(i % 3 == 0));
        shot.data
            .insert("phase".to_string(), Data::F64(f64::from(i) * 0.5));
        shot.data
            .insert("label".to_string(), Data::String(format!("shot_{i}")));

        shot_vec.shots.push(shot);
    }

    // Convert to ShotMap
    let shot_map = shot_vec.try_as_shot_map()?;

    println!("=== ShotMap JSON Serialization Demo ===\n");

    // 1. Convert to JSON Value (preserves BitVec format)
    let json_value = shot_map.to_json();
    println!("1. As serde_json::Value:");
    println!("{json_value}\n");

    // 2. Convert to compact JSON string using .to_string()
    println!("2. As compact JSON string:");
    println!("{json_value}\n");

    // 3. Convert to pretty JSON string
    println!("3. As pretty JSON:");
    let pretty_json = serde_json::to_string_pretty(&json_value)
        .map_err(|e| PecosError::Processing(format!("JSON serialization failed: {e}")))?;
    println!("{pretty_json}\n");

    // 4. Direct serialization (since we derived Serialize)
    println!("4. Direct serde serialization:");
    let direct_json = serde_json::to_string_pretty(&shot_map)
        .map_err(|e| PecosError::Processing(format!("JSON serialization failed: {e}")))?;
    println!("{direct_json}\n");

    // 5. Deserialize back from JSON
    println!("5. Round-trip test:");
    let deserialized: ShotMap = serde_json::from_str(&direct_json)
        .map_err(|e| PecosError::Processing(format!("JSON deserialization failed: {e}")))?;
    println!("Deserialized successfully!");
    println!(
        "Original shots: {}, Deserialized shots: {}",
        shot_map.num_shots(),
        deserialized.num_shots()
    );

    // 6. Working with specific data types in JSON
    println!("\n6. Extracting specific types from JSON:");
    if let Some(iterations) = json_value.get("iteration") {
        println!("Iterations: {iterations}");
    }

    // 7. Simplified JSON output (BitVecs as integers)
    println!("\n7. Simplified JSON (BitVecs as integers):");
    let simple_json = shot_map.to_simple_json();
    println!("{simple_json}\n");

    // Pretty print the simplified JSON
    println!("8. Simplified JSON (pretty printed):");
    let simple_pretty = serde_json::to_string_pretty(&simple_json)
        .map_err(|e| PecosError::Processing(format!("JSON serialization failed: {e}")))?;
    println!("{simple_pretty}");

    Ok(())
}
