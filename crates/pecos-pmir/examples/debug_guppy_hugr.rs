//! Debug PMIR compilation with guppy HUGR files

use pecos_pmir::{compile_hugr_bytes_via_pmir, PmirConfig, binary_hugr_to_json, hugr_to_past_ron, hugr_to_pmir_mlir};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Path to bell state HUGR
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let hugr_path = workspace_root.join("pecos/tests/test_data/hugr/bell_state.hugr");
    
    // Read the bell state HUGR
    let hugr_bytes = fs::read(&hugr_path)?;
    println!("Read {} bytes of HUGR data from {:?}", hugr_bytes.len(), hugr_path);
    
    // Convert to JSON to examine
    let hugr_json = binary_hugr_to_json(&hugr_bytes)?;
    println!("\nHUGR JSON (first 500 chars):\n{}", &hugr_json[..500.min(hugr_json.len())]);
    
    // Try to compile with debug output
    let config = PmirConfig {
        debug_output: true,
        ..Default::default()
    };
    
    println!("\n=== Attempting PMIR compilation ===");
    
    // First, try to get PAST representation
    println!("\n=== Generating PAST representation ===");
    match hugr_to_past_ron(&hugr_json) {
        Ok(past_ron) => {
            println!("PAST RON generated successfully");
            let preview_len = 1000.min(past_ron.len());
            println!("PAST RON (first {} chars):\n{}", preview_len, &past_ron[..preview_len]);
            
            // Save full PAST for inspection
            fs::write("debug_past.ron", &past_ron)?;
            println!("\nFull PAST saved to debug_past.ron");
            
            // Try to get PMIR
            println!("\n=== Generating PMIR representation ===");
            match hugr_to_pmir_mlir(&hugr_json, &config) {
                Ok(pmir) => {
                    println!("PMIR MLIR generated successfully");
                    println!("PMIR MLIR:\n{}", pmir);
                    
                    // Save to file for inspection
                    fs::write("debug_pmir.mlir", &pmir)?;
                    println!("\nSaved PMIR to debug_pmir.mlir");
                    
                    // Now try the full compilation
                    println!("\n=== Attempting full MLIR -> LLVM compilation ===");
                    match compile_hugr_bytes_via_pmir(&hugr_bytes, &config) {
                        Ok(llvm_ir) => {
                            println!("\nSuccess! LLVM IR generated:");
                            println!("{}", &llvm_ir[..500.min(llvm_ir.len())]);
                        }
                        Err(e) => {
                            println!("\nMLIR to LLVM compilation failed: {:?}", e);
                        }
                    }
                }
                Err(e) => println!("Failed to generate PMIR: {:?}", e),
            }
        }
        Err(e) => println!("Failed to generate PAST: {:?}", e),
    }
    
    Ok(())
}