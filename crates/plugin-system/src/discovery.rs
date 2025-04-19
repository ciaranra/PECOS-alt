use crate::registry::PluginRegistry;
use libloading::{Library, Symbol};
use std::ffi::OsStr;
use std::path::Path;

// Add type alias to simplify the complex type
type PluginRegistrationFn = fn(&mut PluginRegistry) -> Result<(), Box<dyn std::error::Error>>;

pub struct PluginDiscovery;

impl PluginDiscovery {
    pub fn discover_rust_plugins(
        path: &Path,
        registry: &mut PluginRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.path().extension() == Some(OsStr::new("rlib")) {
                // Unsafe block needed for loading dynamic libraries
                unsafe {
                    let lib = Library::new(entry.path())?;
                    let register: Symbol<PluginRegistrationFn> = lib.get(b"register_plugin")?;
                    register(registry)?;
                }
            }
        }
        Ok(())
    }
}
