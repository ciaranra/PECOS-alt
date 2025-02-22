use crate::source::{PluginSource, PluginSourceConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_load_standard_plugins() -> bool {
    true
}

// TODO: Maybe the user should be explicit...
fn default_python_interpreter() -> String {
    "python".to_string() // Default to system Python
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    #[serde(default = "default_load_standard_plugins")]
    pub load_standard_plugins: bool,
    #[serde(default = "default_python_interpreter")]
    pub python_interpreter: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            load_standard_plugins: default_load_standard_plugins(),
            python_interpreter: default_python_interpreter(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginEntry {
    pub r#type: String,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginConfig {
    #[serde(default)]
    pub system: SystemConfig,
    pub plugins: Vec<PluginEntry>,
}

impl PluginConfig {
    /// Load configuration from a TOML file
    pub fn from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        println!("Loading plugin config from: {}", path.display());
        let content = std::fs::read_to_string(path)?;
        let mut config: PluginConfig = toml::from_str(&content)?;

        let config_dir = path
            .parent()
            .ok_or("Config file must be in a directory")?
            .to_path_buf();

        // Make Python interpreter path relative to config file
        if !config.system.python_interpreter.is_empty() {
            let interpreter_path = config_dir.join(&config.system.python_interpreter);
            println!("Resolving interpreter path: {}", interpreter_path.display());

            if interpreter_path.exists() {
                // Don't follow symlinks - use the venv path directly
                config.system.python_interpreter = interpreter_path
                    .to_str()
                    .ok_or("Invalid Python interpreter path")?
                    .to_string();
                println!("Using venv interpreter at: {}", interpreter_path.display());
            } else {
                return Err(format!(
                    "Python interpreter not found at: {} (resolved from {})",
                    interpreter_path.display(),
                    config.system.python_interpreter
                )
                .into());
            }
        }

        // Update paths to be relative to config file location
        for plugin in &mut config.plugins {
            if plugin.path.is_relative() {
                plugin.path = config_dir.join(&plugin.path);
            }
            let abs_path = std::fs::canonicalize(&plugin.path).map_err(|e| {
                format!(
                    "Failed to resolve plugin path '{}': {}",
                    plugin.path.display(),
                    e
                )
            })?;

            if !abs_path.exists() {
                return Err(format!(
                    "Plugin path does not exist: {} (resolved from {})",
                    abs_path.display(),
                    plugin.path.display()
                )
                .into());
            }
            println!("Found plugin directory: {}", abs_path.display());
            plugin.path = abs_path;
        }

        Ok(config)
    }

    /// Convert into a PluginSourceConfig
    pub fn into_source_config(self) -> PluginSourceConfig {
        println!(
            "Converting PluginConfig with interpreter: {}",
            self.system.python_interpreter
        );

        let mut config = if self.system.load_standard_plugins {
            PluginSourceConfig::new()
        } else {
            PluginSourceConfig::new_no_std()
        };

        for plugin in self.plugins {
            let plugin_source = match plugin.r#type.to_lowercase().as_str() {
                "python" => PluginSource::python(plugin.path),
                "rust" => PluginSource::rust(plugin.path),
                _ => {
                    println!("Warning: Unknown plugin type: {}", plugin.r#type);
                    continue;
                }
            };
            config = config.add_source(plugin_source);
        }

        // Set Python interpreter
        if !self.system.python_interpreter.is_empty() {
            config = config.with_python_interpreter(Some(self.system.python_interpreter));
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config_str = r#"
            [system]
            load_standard_plugins = true
            python_interpreter = "../../plugin-python-service/.venv/bin/python"

            [[plugins]]
            type = "python"
            path = "../../../examples/python-plugins"

            [[plugins]]
            type = "rust"
            path = "../../../examples/rust-plugins"
        "#;

        let config: PluginConfig = toml::from_str(config_str).unwrap();
        assert!(config.system.load_standard_plugins);
        assert_eq!(config.plugins.len(), 2);
        assert_eq!(config.plugins[0].r#type, "python");
        assert_eq!(config.plugins[1].r#type, "rust");
    }

    #[test]
    fn test_default_system_config() {
        let config_str = r#"
            [[plugins]]
            type = "python"
            path = "examples/python-plugins"
        "#;

        let config: PluginConfig = toml::from_str(config_str).unwrap();
        // Should be true by default
        assert!(config.system.load_standard_plugins);
    }
}
