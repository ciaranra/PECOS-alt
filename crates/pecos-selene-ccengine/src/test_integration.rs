//! Test integration with Selene simple_runtime plugin

use crate::runtime_plugin::RuntimePlugin;
use anyhow::Result;
use std::path::Path;

/// Test loading the simple_runtime plugin
pub fn test_load_simple_runtime() -> Result<()> {
    let plugin_path = "/home/ciaranra/Repos/cl_projects/gup/selene/target/release/libselene_simple_runtime.so";

    println!("Attempting to load simple_runtime plugin from: {}", plugin_path);

    if !Path::new(plugin_path).exists() {
        return Err(anyhow::anyhow!("Plugin file not found: {}", plugin_path));
    }

    let plugin = RuntimePlugin::load(plugin_path)?;
    println!("✅ Successfully loaded simple_runtime plugin!");

    // Try to initialize a runtime instance
    println!("Initializing runtime with 5 qubits...");
    let instance = plugin.init(5)?;
    println!("✅ Successfully initialized runtime instance!");

    // Try to start a shot
    println!("Starting shot 0 with seed 42...");
    plugin.shot_start(instance, 0, 42)?;
    println!("✅ Successfully started shot!");

    // Try to allocate a qubit
    println!("Allocating a qubit...");
    let qubit = plugin.qalloc(instance)?;
    println!("✅ Successfully allocated qubit: {}", qubit);

    // Try to apply an RZ gate
    println!("Applying RZ(π/4) gate to qubit {}...", qubit);
    plugin.rz_gate(instance, qubit, std::f64::consts::PI / 4.0)?;
    println!("✅ Successfully applied RZ gate!");

    // Try to measure the qubit
    println!("Measuring qubit {}...", qubit);
    let result_id = plugin.measure(instance, qubit)?;
    println!("✅ Successfully queued measurement, result_id: {}", result_id);

    // Try to free the qubit
    println!("Freeing qubit {}...", qubit);
    plugin.qfree(instance, qubit)?;
    println!("✅ Successfully freed qubit!");

    // End the shot
    println!("Ending shot...");
    plugin.shot_end(instance, 0, 42)?;
    println!("✅ Successfully ended shot!");

    // Cleanup
    println!("Cleaning up runtime...");
    plugin.exit(instance)?;
    println!("✅ Successfully cleaned up runtime!");

    println!("\n🎉 All tests passed! Simple runtime plugin is working correctly.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_runtime_basic_operations() {
        match test_load_simple_runtime() {
            Ok(()) => println!("✅ Integration test passed!"),
            Err(e) => {
                println!("❌ Integration test failed: {}", e);
                panic!("Integration test failed: {}", e);
            }
        }
    }
}