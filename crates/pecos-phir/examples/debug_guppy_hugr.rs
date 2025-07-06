//! Debug PMIR compilation with guppy HUGR files

use pecos_phir::{InputFormat, PhirConfig, Pipeline};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Path to bell state HUGR
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let hugr_path = workspace_root.join("pecos/tests/test_data/hugr/bell_state.hugr");

    // Read the bell state HUGR
    let hugr_bytes = fs::read(&hugr_path)?;
    println!(
        "Read {} bytes of HUGR data from {}",
        hugr_bytes.len(),
        hugr_path.display()
    );

    // Convert to string for JSON input (simplified)
    let hugr_json = String::from_utf8_lossy(&hugr_bytes);

    // Try to compile with debug output
    let config = PhirConfig {
        debug: true,
        ..Default::default()
    };

    println!("\n=== Attempting PMIR compilation ===");

    let pipeline = Pipeline::new(config);
    let result: Result<(), _> = pipeline.compile_and_execute(&hugr_json, InputFormat::HUGR);

    match result {
        Ok(()) => {
            println!("PMIR pipeline execution completed successfully");
        }
        Err(e) => {
            println!("Failed to execute PMIR pipeline: {e:?}");
            println!("This is expected since parsers are not yet implemented.");
        }
    }

    Ok(())
}
