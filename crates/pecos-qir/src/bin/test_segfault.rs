use pecos_qir::engine::QirEngine;
use pecos_engines::engine_system::{ClassicalEngine, ControlEngine, EngineStage};
use std::path::Path;
use std::env;

fn main() {
    // Initialize logging
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "debug"); }
    }
    let _ = env_logger::try_init();
    
    println!("Starting QIR test...");
    
    // Use the bell.ll example file
    let qir_file = Path::new("examples/qir/bell.ll");
    
    if !qir_file.exists() {
        eprintln!("QIR file not found: {:?}", qir_file);
        return;
    }
    
    // Create QIR engine
    let mut engine = QirEngine::new(qir_file.to_path_buf());
    
    println!("Compiling QIR program...");
    if let Err(e) = engine.compile() {
        eprintln!("Failed to compile: {}", e);
        return;
    }
    
    println!("Starting engine...");
    match engine.start(()) {
        Ok(EngineStage::NeedsProcessing(commands)) => {
            println!("Got {} bytes of quantum operations", commands.into_bytes().len());
            println!("Would normally process these through quantum engine");
        }
        Ok(EngineStage::Complete(shot)) => {
            println!("Program completed with shot data: {:?}", shot);
        }
        Err(e) => {
            eprintln!("Failed to start engine: {}", e);
            return;
        }
    }
    
    println!("Test completed successfully!");
    println!("Exiting program...");
    
    // Explicitly drop the engine to see if that's where the segfault happens
    println!("Dropping engine...");
    drop(engine);
    println!("Engine dropped successfully");
}