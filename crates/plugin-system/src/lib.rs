//! Plugin System Core Library
//! Provides a flexible system for loading and executing python-plugins in Rust and Python.
pub mod config;
pub mod discovery;
pub mod plugin;
pub mod registry;
pub mod runner;
pub mod source;

// Re-export public types and traits
pub use plugin::{Plugin, PluginInfo, PluginStyle, PluginType, Processor};
pub use processors::process::{
    CoProcessor, DrivingProcessor, DynCoProcessor, DynDrivingProcessor, ProcessingStage,
    ProcessingSystem, ProcessorStage,
};
pub use registry::PluginRegistry;
pub use runner::Runner;

// A prelude for convenient imports
pub mod prelude {
    pub use crate::plugin::{Plugin, PluginInfo, PluginStyle, PluginType};
    pub use processors::process::{CoProcessor, DrivingProcessor, ProcessingStage};
}

// Remove unused code that was generating warnings
// Commented out get_rust_doc_comment as it's not being used
/*
fn get_rust_doc_comment<T: ?Sized>() -> Option<String> {
    None
}
*/
