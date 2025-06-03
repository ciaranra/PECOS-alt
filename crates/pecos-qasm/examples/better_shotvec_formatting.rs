// Example of a more robust approach to ShotVec formatting

use pecos_engines::shot_results::{Data, Shot, ShotVec};
use std::collections::HashSet;

trait RobustShotVecFormatter {
    /// Get all unique register names across ALL shots
    fn get_all_register_names(&self) -> Vec<String>;

    /// Format with type information preserved
    fn to_typed_json(&self) -> serde_json::Value;
}

impl RobustShotVecFormatter for ShotVec {
    fn get_all_register_names(&self) -> Vec<String> {
        let mut names = HashSet::new();
        for shot in &self.shots {
            for key in shot.data.keys() {
                if !key.starts_with('_') {
                    // Skip metadata
                    names.insert(key.clone());
                }
            }
        }
        let mut sorted: Vec<_> = names.into_iter().collect();
        sorted.sort();
        sorted
    }

    fn to_typed_json(&self) -> serde_json::Value {
        use serde_json::{Map, Value};

        let register_names = self.get_all_register_names();
        let mut result = Map::new();

        for name in register_names {
            let values: Vec<Value> = self
                .shots
                .iter()
                .map(|shot| match shot.data.get(&name) {
                    Some(data) => match data {
                        Data::U32(v) => json!({
                            "type": "u32",
                            "value": v,
                            "binary": format!("{:032b}", v)
                        }),
                        Data::BitVec(bv) => {
                            let mut binary = String::new();
                            for i in (0..bv.len()).rev() {
                                binary.push(if bv[i] { '1' } else { '0' });
                            }
                            json!({
                                "type": "bitvec",
                                "width": bv.len(),
                                "binary": binary
                            })
                        }
                        Data::String(s) => json!({
                            "type": "string",
                            "value": s
                        }),
                        Data::F64(f) => json!({
                            "type": "f64",
                            "value": f
                        }),
                        _ => json!({
                            "type": "other",
                            "string": data.to_string()
                        }),
                    },
                    None => Value::Null,
                })
                .collect();

            result.insert(name, Value::Array(values));
        }

        Value::Object(result)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a challenging ShotVec
    let mut shot_vec = ShotVec::new();

    let mut shot1 = Shot::default();
    shot1.data.insert("reg_a".to_string(), Data::U32(5));
    shot_vec.shots.push(shot1);

    let mut shot2 = Shot::default();
    shot2
        .data
        .insert("reg_b".to_string(), Data::String("hello".to_string()));
    shot2
        .data
        .insert("reg_c".to_string(), Data::F64(std::f64::consts::PI));
    shot_vec.shots.push(shot2);

    let mut shot3 = Shot::default();
    shot3.data.insert("reg_a".to_string(), Data::U32(7));
    shot3
        .data
        .insert("reg_b".to_string(), Data::String("world".to_string()));
    shot_vec.shots.push(shot3);

    println!("=== Better ShotVec Formatting ===\n");

    println!(
        "All register names: {:?}",
        shot_vec.get_all_register_names()
    );

    println!("\nTyped JSON format:");
    let typed = shot_vec.to_typed_json();
    println!("{}", serde_json::to_string_pretty(&typed)?);

    // This example focuses on custom formatting for ShotVec
    // The QASMResults type provides QASM-specific formatting
    println!("\n\nNote: For QASM-specific binary formatting, use QASMResults type.");

    Ok(())
}

use serde_json::json;
