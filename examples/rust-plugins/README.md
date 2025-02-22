# Rust Examples

Examples of external Rust plugins for the plugin system.

## Current Examples
- `custom-processors/` - Shows how to create a custom processor that multiplies numbers by a configurable factor

## Creating a New Plugin

1. Create a new crate in this directory
2. Add these dependencies to your `Cargo.toml`:
```toml
[dependencies]
serde_json = "1"
plugin-system = { path = "../../../crates/plugin-system" }
plugin-system-macros = { path = "../../../crates/plugin-system-macros" }
```

3. Implement either `CoProcessor` or `DrivingProcessor`:
```rust
use plugin_system::prelude::*;
use plugin_system_macros::PluginImpl;
use serde_json::{json, Value};

#[derive(Debug, Clone, PluginImpl)]
pub struct MyProcessor;

impl CoProcessor for MyProcessor {
    fn process(&mut self, input: Value) -> Value {
        // Your processing logic here
        json!({ "result": "processed" })
    }

    fn clone_box(&self) -> Box<dyn CoProcessor> {
        Box::new(self.clone())
    }
}
```

4. Register your plugin:
```rust
pub struct MyPlugins;

impl RustPlugin for MyPlugins {
    fn register(registry: &mut PluginRegistry) {
        registry.register_coprocessor(
            "MyProcessor".to_string(),
            PluginType::Rust,
            Box::new(MyProcessor::new())
        );
    }
}
```
