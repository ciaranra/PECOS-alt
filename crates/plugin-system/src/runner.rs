use crate::discovery::PluginDiscovery;
use crate::plugin::{PluginInfo, PluginStyle, PluginType};
use crate::registry::PluginRegistry;
use crate::source::{PluginSource, PluginSourceConfig};
use processors::process::ProcessingStage;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum OutgoingMessage {
    Execute {
        operation: String,
        style: PluginStyle,
        args: Vec<i32>,
    },
    ListPlugins,
    Shutdown,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    Result { value: i32 },
    Error { message: String },
    PluginList { plugins: Vec<PythonPluginInfo> },
}

#[derive(Deserialize, Debug)]
pub struct PythonPluginInfo {
    pub name: String,
    pub style: String,
    pub description: String,
}

pub struct Runner {
    writer: BufWriter<std::process::ChildStdin>,
    reader: BufReader<std::process::ChildStdout>,
    shutdown: bool,
    pub registry: PluginRegistry,
}

impl Runner {
    /// Start a new Runner with the given plugin configuration
    pub fn start(config: PluginSourceConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let registry = if config.load_standard_plugins {
            PluginRegistry::new()
        } else {
            PluginRegistry::new_no_std()
        };
        Self::start_with_config(config, registry)
    }

    fn check_python_environment(python_cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Try to import plugin_python_service to check if it's installed
        let status = Command::new(python_cmd)
            .arg("-c")
            .arg("import plugin_python_service")
            .status()
            .map_err(|e| format!("Failed to run Python interpreter '{}': {}", python_cmd, e))?;

        if !status.success() {
            return Err(format!(
                "plugin_python_service package is not installed in the Python environment at '{}'. \
                Please install it using:\n\
                pip install plugin_python_service\n\
                or if you're developing the system:\n\
                cd crates/plugin-python-service && maturin develop",
                python_cmd
            )
            .into());
        }

        Ok(())
    }

    /// Start with a custom registry and plugin configuration
    fn start_with_config(
        config: PluginSourceConfig,
        mut registry: PluginRegistry,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!(
            "Runner received config with interpreter: {:?}",
            config.get_python_interpreter()
        );

        // Get the interpreter once at the start and use it consistently
        let python_cmd = config
            .python_interpreter
            .clone()
            .unwrap_or_else(|| "python".to_string());

        println!("Runner using Python interpreter path: {}", python_cmd);

        let interpreter_path = std::path::Path::new(&python_cmd);
        if !interpreter_path.exists() {
            return Err(format!(
                "Python interpreter not found at: {}",
                interpreter_path.display()
            )
            .into());
        }

        // Keep the venv path - don't canonicalize
        Self::check_python_environment(&python_cmd)?;

        // Track Python sources to pass to the Python runner
        let mut python_sources = Vec::new();

        // Process each plugin source
        for source in config.sources {
            match source {
                PluginSource::Python(path) => {
                    let path = std::fs::canonicalize(&path).map_err(|e| {
                        format!(
                            "Failed to resolve Python plugin path '{}': {}",
                            path.display(),
                            e
                        )
                    })?;
                    python_sources.push(path);
                }
                PluginSource::Rust(path) => {
                    let path = std::fs::canonicalize(&path).map_err(|e| {
                        format!(
                            "Failed to resolve Rust plugin path '{}': {}",
                            path.display(),
                            e
                        )
                    })?;
                    PluginDiscovery::discover_rust_plugins(&path, &mut registry)?;
                }
            }
        }

        // Set up Python runner
        let runner_script = PathBuf::from("crates")
            .join("plugin-python-service")
            .join("plugin_python_service")
            .join("service.py");

        // Log the exact path we're trying to use
        println!(
            "Looking for Python runner script at: {}",
            runner_script.display()
        );

        if !runner_script.exists() {
            return Err(format!(
                "Python runner script not found at: {}. Are you running from the project root?",
                runner_script.display()
            )
            .into());
        }

        // Use the canonical interpreter path for spawning
        let mut child = Command::new(&python_cmd)
            .arg(&runner_script)
            .args(python_sources.iter().map(|p| p.as_os_str()))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn Python runner process: {}", e))?;

        // Take ownership of stdin and stdout
        let stdin = child
            .stdin
            .take()
            .ok_or("Failed to capture child process stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture child process stdout")?;

        Ok(Self {
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
            shutdown: false,
            registry,
        })
    }

    pub fn start_no_std(plugin_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let registry = PluginRegistry::new_no_std();
        Self::start_with_registry(plugin_dir, registry)
    }

    fn start_with_registry(
        plugin_dir: &str,
        mut registry: PluginRegistry,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load example external Rust plugins if in development
        #[cfg(debug_assertions)]
        if let Ok(plugins_path) = std::fs::canonicalize("examples/rust-plugins") {
            PluginDiscovery::discover_rust_plugins(&plugins_path, &mut registry)?;
        }

        // Set up Python runner
        let runner_script = PathBuf::from("crates")
            .join("plugin-python-service")
            .join("plugin_python_service")
            .join("service.py");

        // Canonicalize the plugin directory path
        let plugin_dir = std::fs::canonicalize(plugin_dir)?;

        // Spawn Python process
        let mut child = Command::new("python")
            .arg(&runner_script)
            .arg(&plugin_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Take ownership of stdin and stdout
        let stdin = child
            .stdin
            .take()
            .ok_or("Failed to capture child process stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture child process stdout")?;

        Ok(Self {
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
            shutdown: false,
            registry,
        })
    }

    // Convenience method for backward compatibility
    pub fn start_with_python_plugins(
        python_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = PluginSourceConfig::new().add_source(PluginSource::python(python_path));
        Self::start(config)
    }

    // Convenience method for backward compatibility
    pub fn start_no_std_with_python_plugins(
        python_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = PluginSourceConfig::new_no_std().add_source(PluginSource::python(python_path));
        Self::start(config)
    }

    pub fn execute(
        &mut self,
        operation: &str,
        style: PluginStyle,
        args: &[i32],
    ) -> Result<i32, Box<dyn std::error::Error>> {
        if self.shutdown {
            return Err("Runner is shut down".into());
        }

        // First try Rust python-plugins
        match style {
            PluginStyle::CoProcessor => {
                if let Some(processor) = self.registry.get_coprocessor(operation) {
                    println!("Executing Rust coprocessor plugin: {}", operation);
                    let input = json!({ "numbers": args });
                    let output = processor.process(input);
                    if let Some(result) = output["result"].as_i64() {
                        return Ok(result as i32);
                    }
                    return Err("Coprocessor didn't return expected format".into());
                }
            }
            PluginStyle::DrivingProcessor => {
                if let Some(processor) = self.registry.get_driving_processor(operation) {
                    println!("Executing Rust driving processor plugin: {}", operation);
                    let input = json!({ "numbers": args });
                    let mut stage = processor.start(input);
                    while let ProcessingStage::NeedsCoprocessing(_) = stage {
                        stage = processor.continue_processing(json!({"numbers": []}));
                    }
                    if let ProcessingStage::Complete(output) = stage {
                        if let Some(result) = output["result"].as_i64() {
                            return Ok(result as i32);
                        }
                    }
                    return Err("DrivingProcessor didn't return expected format".into());
                }
            }
        }

        // If no Rust plugin found, try Python python-plugins
        let message = OutgoingMessage::Execute {
            operation: operation.to_string(),
            style,
            args: args.to_vec(),
        };

        serde_json::to_writer(&mut self.writer, &message)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        let mut response = String::with_capacity(128);
        self.reader.read_line(&mut response)?;

        let result: IncomingMessage = serde_json::from_str(&response)?;
        match result {
            IncomingMessage::Result { value } => Ok(value),
            IncomingMessage::Error { message } => Err(message.into()),
            _ => Err("Unexpected response type".into()),
        }
    }

    pub fn list_plugins(&mut self) -> Result<Vec<PluginInfo>, Box<dyn std::error::Error>> {
        // Get Rust python-plugins - reserve space for both Rust and estimated Python python-plugins
        let mut plugins = Vec::with_capacity(self.registry.len() * 2);
        plugins.extend(self.registry.list_plugins());

        let message = OutgoingMessage::ListPlugins;
        serde_json::to_writer(&mut self.writer, &message)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        let mut response = String::with_capacity(256);
        self.reader.read_line(&mut response)?;

        let result: IncomingMessage = serde_json::from_str(&response)?;
        match result {
            IncomingMessage::PluginList {
                plugins: python_plugins,
            } => {
                plugins.reserve(python_plugins.len()); // Reserve exact space needed
                for p in python_plugins {
                    // Validate style once before creating the PluginInfo
                    let style = match p.style.as_str() {
                        "coprocessor" => PluginStyle::CoProcessor,
                        "driving_processor" => PluginStyle::DrivingProcessor,
                        _ => return Err("Unknown plugin style".into()),
                    };

                    plugins.push(PluginInfo {
                        name: p.name,
                        plugin_type: PluginType::Python,
                        plugin_style: style,
                        description: p.description,
                    });
                }
                Ok(plugins)
            }
            IncomingMessage::Error { message } => Err(message.into()),
            _ => Err("Unexpected response type".into()),
        }
    }

    pub fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.shutdown {
            return Ok(());
        }

        let message = OutgoingMessage::Shutdown;
        serde_json::to_writer(&mut self.writer, &message)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;

        std::thread::sleep(Duration::from_millis(100));

        self.shutdown = true;
        Ok(())
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        if let Err(e) = self.shutdown() {
            eprintln!("Error during shutdown: {}", e);
        }
    }
}
