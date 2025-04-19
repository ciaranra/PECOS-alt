use crate::PluginRegistry;
use pecos_core::StructMetadata;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt;

pub trait RustPlugin {
    fn register(registry: &mut PluginRegistry);
}

// Core traits without PyO3 dependencies
pub trait Plugin: Send + Sync + StructMetadata + Any {
    fn execute(&self, args: &[i32]) -> i32;
}

pub trait Processor: Send + Sync + StructMetadata + Any {
    fn process(&self, a: u32, b: u32) -> u32;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginType {
    Rust,
    Python,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PluginStyle {
    CoProcessor,
    DrivingProcessor,
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginType::Rust => write!(f, "Rust"),
            PluginType::Python => write!(f, "Python"),
        }
    }
}

impl fmt::Display for PluginStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginStyle::CoProcessor => write!(f, "CoProcessor"),
            PluginStyle::DrivingProcessor => write!(f, "DrivingProcessor"),
        }
    }
}

pub struct PluginInfo {
    pub name: String,
    pub plugin_type: PluginType,
    pub plugin_style: PluginStyle,
    pub description: String,
}
