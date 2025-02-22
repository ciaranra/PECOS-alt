use plugin_system::plugin::RustPlugin;
use plugin_system::prelude::*;
use plugin_system::PluginRegistry;

pub mod processors;

// Implementation of the registration trait
pub struct CustomPlugins;

impl RustPlugin for CustomPlugins {
    fn register(registry: &mut PluginRegistry) {
        registry.register_coprocessor(
            "CustomNumberMultiplier".to_string(),
            PluginType::Rust,
            Box::new(processors::NumberMultiplier::new(3)),
        );
        // Register other custom processors...
    }
}

// Export the registration function
#[no_mangle]
pub extern "C" fn register_plugin(registry: &mut PluginRegistry) {
    CustomPlugins::register(registry);
}
