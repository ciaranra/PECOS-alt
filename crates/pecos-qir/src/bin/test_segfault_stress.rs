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
    
    println!("Starting QIR stress test...");
    
    // Use the bell.ll example file
    let qir_file = Path::new("examples/qir/bell.ll");
    
    if !qir_file.exists() {
        eprintln!("QIR file not found: {:?}", qir_file);
        return;
    }
    
    // Test 1: Create and drop multiple engines
    println!("\nTest 1: Create and drop multiple engines");
    for i in 0..3 {
        println!("  Iteration {}", i);
        let mut engine = QirEngine::new(qir_file.to_path_buf());
        
        if let Err(e) = engine.compile() {
            eprintln!("  Failed to compile: {}", e);
            continue;
        }
        
        match engine.start(()) {
            Ok(EngineStage::NeedsProcessing(commands)) => {
                println!("  Got {} bytes of commands", commands.into_bytes().len());
            }
            Ok(EngineStage::Complete(_)) => {
                println!("  Program completed");
            }
            Err(e) => {
                eprintln!("  Failed to start engine: {}", e);
            }
        }
        
        println!("  Dropping engine {}", i);
        drop(engine);
        println!("  Engine {} dropped", i);
    }
    
    // Test 2: Clone engines
    println!("\nTest 2: Clone engines");
    {
        let mut engine1 = QirEngine::new(qir_file.to_path_buf());
        if let Err(e) = engine1.compile() {
            eprintln!("Failed to compile: {}", e);
            return;
        }
        
        println!("  Cloning engine...");
        let mut engine2 = engine1.clone();
        
        println!("  Running both engines...");
        let _ = engine1.start(());
        let _ = engine2.start(());
        
        println!("  Dropping cloned engines...");
    }
    
    // Test 3: Reset multiple times
    println!("\nTest 3: Reset multiple times");
    {
        let mut engine = QirEngine::new(qir_file.to_path_buf());
        if let Err(e) = engine.compile() {
            eprintln!("Failed to compile: {}", e);
            return;
        }
        
        for i in 0..3 {
            println!("  Reset iteration {}", i);
            let _ = engine.start(());
            let _ = ClassicalEngine::reset(&mut engine);
        }
        
        println!("  Final drop after resets...");
    }
    
    println!("\nAll tests completed successfully!");
    println!("Exiting program...");
    
    // Force a flush to ensure all output is written
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    
    println!("About to exit main()...");
}