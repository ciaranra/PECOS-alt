mod tests;

use crate::tests::*;
use plugin_system::{config::PluginConfig, Runner};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Look for plugins.toml in the same directory as the executable
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins.toml");

    // Load plugin configuration
    let config = PluginConfig::from_file(config_path)?;

    // Create and manage the Runner with configuration
    let mut runner = Runner::start(config.into_source_config())?;

    println!("\nRunning Processing System Demo...");
    test_processing_system()?;

    println!("\nRunning Processing System Demo 2...");
    test_processing_system2(&mut runner)?;

    // List all available plugins
    println!("\n\nAvailable Plugins:");
    let plugins = runner.list_plugins()?;
    for plugin in plugins {
        println!(
            "- {} ({:?} / {:?}): {}",
            plugin.name, plugin.plugin_type, plugin.plugin_style, plugin.description
        );
    }

    // Ensure clean shutdown with logging
    println!("\nInitiating shutdown...");
    runner.shutdown()?;
    println!("Shutdown complete.");

    Ok(())
}
