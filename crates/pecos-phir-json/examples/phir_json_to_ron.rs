use pecos_phir_json::{phir_json_to_ron, phir_json_to_module};
use pecos_phir::ModuleRonExt;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <input.phir.json> [output.ron]", args[0]);
        eprintln!("\nThis tool converts PHIR-JSON files to PHIR-RON format.");
        eprintln!("If no output file is specified, prints to stdout.");
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    let output_path = args.get(2);
    
    // Step 1: Read the .phir.json file
    println!("Reading PHIR-JSON from: {}", input_path);
    let json_content = fs::read_to_string(input_path)?;
    
    // Step 2: Convert using streaming (no intermediate AST)
    println!("Processing PHIR-JSON...");
    
    // Convert to PHIR-RON text (streaming conversion)
    println!("\nConverting to PHIR-RON text...");
    let ron_text = phir_json_to_ron(&json_content)?;
    
    if let Some(output) = output_path {
        // Write RON to file
        fs::write(output, &ron_text)?;
        println!("Wrote PHIR-RON to: {}", output);
    } else {
        // Print RON to stdout
        println!("\nPHIR-RON output:\n");
        println!("{}", ron_text);
    }
    
    // Convert to PHIR Module (this internally goes through RON again)
    println!("\nConverting to PHIR Module (via RON)...");
    let module = phir_json_to_module(&json_content)?;
    println!("Successfully created PHIR Module: {}", module.name);
    
    // We can also serialize the module back to RON for verification
    let module_ron = module.to_ron()?;
    println!("\nModule serialized back to RON has {} characters", module_ron.len());
    
    // Note: The conversion path is always:
    // PHIR-JSON → PHIR-RON (text) → PHIR Module (in memory)
    println!("\nConversion path: PHIR-JSON → PHIR-RON → PHIR Module");
    
    Ok(())
}