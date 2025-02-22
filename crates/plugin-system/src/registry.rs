use crate::plugin::{PluginInfo, PluginStyle, PluginType};
use core::StructMetadata;
use processors::process::{CoProcessor, DrivingProcessor};
use processors::std_processors::{ArrayProcessor, BatchSummer, NumberDoubler};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
struct PluginCollection<T: ?Sized> {
    plugins: HashMap<(String, PluginType), Box<T>>,
}

impl<T: ?Sized> PluginCollection<T> {
    fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    fn insert(&mut self, name: String, plugin_type: PluginType, plugin: Box<T>) {
        self.plugins.insert((name, plugin_type), plugin);
    }

    // Return a reference to the Box for cases where we need it
    fn get_mut(&mut self, name: &str) -> Option<&mut Box<T>> {
        let rust_key = (name.to_string(), PluginType::Rust);
        let python_key = (name.to_string(), PluginType::Python);

        if self.plugins.contains_key(&rust_key) {
            self.plugins.get_mut(&rust_key)
        } else {
            self.plugins.get_mut(&python_key)
        }
    }

    // Return a reference to the Box for cases where we need it
    fn get_specific_mut(&mut self, name: &str, plugin_type: PluginType) -> Option<&mut Box<T>> {
        self.plugins.get_mut(&(name.to_string(), plugin_type))
    }
}

pub struct PluginRegistry {
    coprocessor_plugins: PluginCollection<dyn CoProcessor>,
    driving_processor_plugins: PluginCollection<dyn DrivingProcessor<Value, Value>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Default constructor includes standard plugins
    pub fn new() -> Self {
        let mut registry = Self::new_no_std();
        registry.register_standard_plugins();
        registry
    }

    /// Constructor without standard plugins
    pub fn new_no_std() -> Self {
        Self {
            coprocessor_plugins: PluginCollection::new(),
            driving_processor_plugins: PluginCollection::new(),
        }
    }

    // Method to register standard plugins
    pub fn register_standard_plugins(&mut self) {
        // Register built-in processors from processor-examples
        self.register_coprocessor(
            "NumberDoubler".to_string(),
            PluginType::Rust,
            Box::new(NumberDoubler),
        );
        self.register_driving_processor(
            "BatchSummer".to_string(),
            PluginType::Rust,
            Box::new(BatchSummer::new()),
        );
        self.register_driving_processor(
            "ArrayProcessor".to_string(),
            PluginType::Rust,
            Box::new(ArrayProcessor::new()),
        );
    }

    // Coprocessor Plugins
    pub fn register_coprocessor(
        &mut self,
        name: String,
        language: PluginType,
        processor: Box<dyn CoProcessor>,
    ) {
        println!("Registering {} coprocessor plugin: {}", language, name);
        self.coprocessor_plugins.insert(name, language, processor);
    }

    pub fn get_coprocessor(&mut self, name: &str) -> Option<&mut Box<dyn CoProcessor>> {
        self.coprocessor_plugins.get_mut(name)
    }

    pub fn get_coprocessor_in(
        &mut self,
        name: &str,
        language: PluginType,
    ) -> Option<&mut Box<dyn CoProcessor>> {
        self.coprocessor_plugins.get_specific_mut(name, language)
    }

    // Driving Processor Plugins
    pub fn register_driving_processor(
        &mut self,
        name: String,
        language: PluginType,
        processor: Box<dyn DrivingProcessor<Value, Value>>,
    ) {
        println!(
            "Registering {} driving processor plugin: {}",
            language, name
        );
        self.driving_processor_plugins
            .insert(name, language, processor);
    }

    pub fn get_driving_processor(
        &mut self,
        name: &str,
    ) -> Option<&mut Box<dyn DrivingProcessor<Value, Value>>> {
        self.driving_processor_plugins.get_mut(name)
    }

    pub fn get_driving_processor_in(
        &mut self,
        name: &str,
        language: PluginType,
    ) -> Option<&mut Box<dyn DrivingProcessor<Value, Value>>> {
        self.driving_processor_plugins
            .get_specific_mut(name, language)
    }

    // Utility methods
    pub fn len(&self) -> usize {
        self.coprocessor_plugins.plugins.len() + self.driving_processor_plugins.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn collect_plugin_info<T: StructMetadata + ?Sized>(
        plugins: &HashMap<(String, PluginType), Box<T>>,
        style: PluginStyle,
    ) -> Vec<PluginInfo> {
        plugins
            .iter()
            .map(|((_, plugin_type), plugin)| PluginInfo {
                name: plugin.name().to_string(),
                plugin_type: plugin_type.clone(),
                plugin_style: style.clone(),
                description: plugin.description().to_string(),
            })
            .collect()
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut plugins = Vec::new();

        plugins.extend(Self::collect_plugin_info(
            &self.coprocessor_plugins.plugins,
            PluginStyle::CoProcessor,
        ));
        plugins.extend(Self::collect_plugin_info(
            &self.driving_processor_plugins.plugins,
            PluginStyle::DrivingProcessor,
        ));

        plugins.sort_by(|a, b| a.name.cmp(&b.name));
        plugins.dedup_by(|a, b| a.name == b.name && a.plugin_type == b.plugin_type);
        plugins
    }
}
