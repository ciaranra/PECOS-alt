//! Examples of using Selene runtime plugins with QisControlEngine
//!
//! This example demonstrates how to use the Selene runtime plugins:
//! - Simple runtime: Basic runtime implementation
//! - Soft RZ runtime: Runtime with soft RZ gate support

use pecos_qis_ccengine::{
    qis_control_engine,
    find_selene_runtime,
    selene_simple_runtime,
    selene_soft_rz_runtime,
    selene_runtime_auto,
};
use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Selene Runtime Plugins Usage Examples ===\n");

    // Check which runtime plugins are available
    println!("Checking available Selene runtime plugins:");
    check_runtime_availability();
    println!();

    // Demonstrate usage patterns for each runtime
    demo_simple_runtime()?;
    println!();
    demo_soft_rz_runtime()?;
    println!();
    demo_runtime_auto()?;

    println!("\n=== All examples completed ===");
    Ok(())
}

fn check_runtime_availability() {
    let runtimes = [
        ("simple_runtime", "Selene Simple Runtime"),
        ("soft_rz_runtime", "Selene Soft RZ Runtime"),
    ];

    for (name, description) in &runtimes {
        if let Some(path) = find_selene_runtime(name) {
            println!("  [Available] {}: {} at {}", name, description, path.display());
        } else {
            println!("  [Not Found] {}: {}", name, description);
        }
    }
}

fn demo_simple_runtime() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Simple Runtime ---");

    match selene_simple_runtime() {
        Ok(runtime) => {
            let engine = qis_control_engine().runtime(runtime).build()?;
            println!("Successfully created engine with Selene simple runtime");
            println!("Engine has {} qubits", engine.num_qubits());
        }
        Err(e) => {
            println!("Could not load simple runtime: {}", e);
            println!("This is expected if the Selene repository hasn't been built");
            println!("To build: cd ../selene && cargo build --release");
        }
    }

    Ok(())
}

fn demo_soft_rz_runtime() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Soft RZ Runtime ---");

    match selene_soft_rz_runtime() {
        Ok(runtime) => {
            let engine = qis_control_engine().runtime(runtime).build()?;
            println!("Successfully created engine with Selene soft RZ runtime");
            println!("Engine has {} qubits", engine.num_qubits());
            println!("This runtime provides more accurate RZ gate modeling");
        }
        Err(e) => {
            println!("Could not load soft RZ runtime: {}", e);
            println!("This is expected if the Selene repository hasn't been built");
        }
    }

    Ok(())
}

fn demo_runtime_auto() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Runtime Auto-Discovery ---");

    println!("Attempting to auto-discover 'simple_runtime'...");
    match selene_runtime_auto("simple_runtime") {
        Ok(runtime) => {
            let engine = qis_control_engine().runtime(runtime).build()?;
            println!("Successfully auto-discovered and loaded runtime");
            println!("Engine has {} qubits", engine.num_qubits());
        }
        Err(e) => {
            println!("Could not auto-discover runtime: {}", e);
            println!("The runtime would be downloaded if GitHub releases were configured");
        }
    }

    Ok(())
}