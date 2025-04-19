use std::path::PathBuf;

/// Configuration for plugin discovery and loading
#[derive(Debug, Clone)]
pub struct PluginSourceConfig {
    /// List of plugin sources to load from
    pub sources: Vec<PluginSource>,
    /// Whether to load standard/internal plugins
    pub load_standard_plugins: bool,
    /// Python interpreter to use
    pub python_interpreter: Option<String>,
}

impl Default for PluginSourceConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            load_standard_plugins: true,
            python_interpreter: None,
        }
    }
}

impl PluginSourceConfig {
    /// Create a new configuration with standard plugins enabled
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new configuration without standard plugins
    pub fn new_no_std() -> Self {
        Self {
            sources: Vec::new(),
            load_standard_plugins: false,
            python_interpreter: None,
        }
    }

    /// Add a plugin source
    pub fn add_source(mut self, source: PluginSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Set the Python interpreter
    pub fn with_python_interpreter(mut self, interpreter: Option<String>) -> Self {
        self.python_interpreter = interpreter;
        self
    }

    pub fn get_python_interpreter(&self) -> Option<&str> {
        self.python_interpreter.as_deref()
    }
}

impl From<crate::config::PluginConfig> for PluginSourceConfig {
    fn from(config: crate::config::PluginConfig) -> Self {
        let mut source_config = if config.system.load_standard_plugins {
            Self::new()
        } else {
            Self::new_no_std()
        };

        // Add plugin sources
        for plugin in config.plugins {
            let plugin_source = match plugin.r#type.to_lowercase().as_str() {
                "python" => PluginSource::python(plugin.path),
                "rust" => PluginSource::rust(plugin.path),
                _ => continue,
            };
            source_config = source_config.add_source(plugin_source);
        }

        println!(
            "Converting config with interpreter: {}",
            config.system.python_interpreter
        );
        // Add Python interpreter if specified
        if !config.system.python_interpreter.is_empty() {
            source_config = source_config
                .with_python_interpreter(Some(config.system.python_interpreter.clone()));
            println!(
                "Set interpreter in source config to: {:?}",
                source_config.python_interpreter
            );
        }

        source_config
    }
}

/// Represents a source of plugins
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// Python plugins loaded from a directory
    Python(PathBuf),
    /// Rust plugins loaded from a directory
    Rust(PathBuf),
}

impl PluginSource {
    /// Create a new Python plugin source
    pub fn python<P: Into<PathBuf>>(path: P) -> Self {
        Self::Python(path.into())
    }

    /// Create a new Rust plugin source
    pub fn rust<P: Into<PathBuf>>(path: P) -> Self {
        Self::Rust(path.into())
    }
}
