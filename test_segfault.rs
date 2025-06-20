use pecos_qir::engine::QirEngine;
use std::path::Path;

fn main() {
    env_logger::init();
    
    println!("Starting QIR test...");
    
    // Use the bell.ll example file
    let qir_file = Path::new("examples/qir/bell.ll");
    
    if !qir_file.exists() {
        eprintln!("QIR file not found: {:?}", qir_file);
        return;
    }
    
    // Create QIR engine
    let mut engine = QirEngine::new(qir_file.to_path_buf());
    
    println!("Initializing engine...");
    if let Err(e) = engine.initialize() {
        eprintln!("Failed to initialize engine: {}", e);
        return;
    }
    
    println!("Running quantum program...");
    if let Err(e) = engine.perform_measurement() {
        eprintln!("Failed to perform measurement: {}", e);
        return;
    }
    
    println!("Getting quantum operations...");
    match engine.get_quantum_operations() {
        Ok(Some(ops)) => {
            println!("Got {} bytes of quantum operations", ops.into_bytes().len());
        }
        Ok(None) => {
            println!("No quantum operations generated");
        }
        Err(e) => {
            eprintln!("Failed to get quantum operations: {}", e);
            return;
        }
    }
    
    println!("Test completed successfully!");
    println!("Exiting program...");
}