//! Builder for SeleneSimpleRuntimeEngine
//!
//! This builder configures and creates SeleneSimpleRuntimeEngine instances,
//! handling the loading of Selene's simple runtime library.

use crate::selene_simple_runtime_engine::SeleneSimpleRuntimeEngine;
use pecos_core::prelude::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_programs::{Program, SeleneInterfaceProgram};
use std::path::{Path, PathBuf};

/// Builder for creating SeleneSimpleRuntimeEngine instances
#[derive(Clone)]
pub struct SeleneSimpleRuntimeEngineBuilder {
    /// Path to the Selene simple runtime library
    runtime_library_path: Option<PathBuf>,

    /// The program to execute
    program: Option<SeleneInterfaceProgram>,

    /// Number of qubits
    num_qubits: Option<usize>,
}

impl SeleneSimpleRuntimeEngineBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            runtime_library_path: None,
            program: None,
            num_qubits: None,
        }
    }

    /// Try to find Selene runtime via Python import
    fn find_via_python_selene(lib_name: &str) -> Option<PathBuf> {
        use std::process::Command;

        // Try to get Selene's dist directory via Python
        let python_code = r#"
try:
    from selene_sim import dist_dir
    print(dist_dir)
except:
    pass
"#;

        if let Ok(output) = Command::new("python3").arg("-c").arg(python_code).output()
            && output.status.success()
            && let Ok(dist_str) = String::from_utf8(output.stdout)
        {
            let dist_path = PathBuf::from(dist_str.trim());
            let lib_path = dist_path.join("lib").join(lib_name);
            if lib_path.exists() {
                return Some(lib_path);
            }
        }

        // Try alternative Python command
        if let Ok(output) = Command::new("python").arg("-c").arg(python_code).output()
            && output.status.success()
            && let Ok(dist_str) = String::from_utf8(output.stdout)
        {
            let dist_path = PathBuf::from(dist_str.trim());
            let lib_path = dist_path.join("lib").join(lib_name);
            if lib_path.exists() {
                return Some(lib_path);
            }
        }

        None
    }

    /// Set the path to the Selene simple runtime library
    pub fn runtime_library<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.runtime_library_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Use the default Selene simple runtime library location
    pub fn default_runtime(mut self) -> Self {
        // Try to find the runtime library using multiple strategies

        // First, check environment variable
        if let Ok(path) = std::env::var("SELENE_RUNTIME_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                log::info!("Found Selene runtime via SELENE_RUNTIME_PATH: {:?}", path);
                self.runtime_library_path = Some(path);
                return self;
            }
        }

        // Try to use our ByteMessageSimulator plugin first
        // Look for it in the target directory
        let plugin_names = if cfg!(target_os = "windows") {
            vec!["pecos_selene_plugins.dll", "selene_simple_runtime.dll"]
        } else if cfg!(target_os = "macos") {
            vec![
                "libpecos_selene_plugins.dylib",
                "libselene_simple_runtime.dylib",
            ]
        } else {
            vec!["libpecos_selene_plugins.so", "libselene_simple_runtime.so"]
        };

        // Check in various target directories
        for plugin_name in &plugin_names {
            // Check debug build
            let debug_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/debug")
                .join(plugin_name);
            if debug_path.exists() {
                log::info!("Found runtime plugin in debug build: {:?}", debug_path);
                self.runtime_library_path = Some(debug_path);
                return self;
            }

            // Check release build
            let release_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/release")
                .join(plugin_name);
            if release_path.exists() {
                log::info!("Found runtime plugin in release build: {:?}", release_path);
                self.runtime_library_path = Some(release_path);
                return self;
            }
        }

        // Fall back to finding selene_simple_runtime via Python
        let lib_name = plugin_names.last().unwrap();

        // Try to find via Python import of selene
        if let Some(path) = Self::find_via_python_selene(lib_name) {
            log::info!("Found Selene runtime via Python selene package: {:?}", path);
            self.runtime_library_path = Some(path);
            return self;
        }

        // Build a list of search paths
        let mut search_paths = Vec::new();

        // Current working directory and parent directories
        if let Ok(cwd) = std::env::current_dir() {
            search_paths.push(cwd.clone());
            search_paths.push(cwd.join("target/release"));
            search_paths.push(cwd.join("target/debug"));

            // Check parent directories for Selene repo
            let mut parent = cwd.parent();
            while let Some(p) = parent {
                let selene_target = p.join("selene/target");
                if selene_target.exists() {
                    search_paths.push(selene_target.join("release"));
                    search_paths.push(selene_target.join("debug"));
                }
                parent = p.parent();
            }
        }

        // Python virtual environment paths
        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            let venv_path = PathBuf::from(venv);
            // Try multiple Python versions
            for version in &[
                "python3.12",
                "python3.11",
                "python3.10",
                "python3.9",
                "python3.8",
            ] {
                search_paths.push(venv_path.join(format!(
                    "lib/{}/site-packages/selene_simple_runtime_plugin/_dist/lib",
                    version
                )));
            }
        }

        // Common installation directories
        search_paths.push(PathBuf::from("/usr/local/lib"));
        search_paths.push(PathBuf::from("/usr/lib"));
        search_paths.push(PathBuf::from("/opt/selene/lib"));

        // Home directory locations
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            search_paths.push(home_path.join(".local/lib"));
            search_paths.push(home_path.join(".cache/selene"));
        }

        // Search for the library
        for dir in search_paths {
            let full_path = dir.join(lib_name);
            if full_path.exists() {
                log::info!("Found Selene runtime at: {:?}", full_path);
                self.runtime_library_path = Some(full_path);
                return self;
            }
        }

        // If not found, use a default that will error at build time with helpful message
        log::warn!(
            "Could not find Selene simple runtime library. Set SELENE_RUNTIME_PATH environment variable or ensure selene is installed."
        );
        self.runtime_library_path = Some(PathBuf::from(lib_name));
        self
    }

    /// Set the program to execute
    pub fn program(mut self, program: impl Into<Program>) -> Self {
        let prog = program.into();
        if let Program::SeleneInterface(selene_interface) = prog {
            self.program = Some(selene_interface);
        }
        // Ignore other program types - they're not for this engine
        self
    }

    /// Set a SeleneInterfaceProgram directly
    pub fn selene_interface_program(mut self, program: SeleneInterfaceProgram) -> Self {
        self.program = Some(program);
        self
    }

    /// Set the number of qubits
    pub fn qubits(mut self, n: usize) -> Self {
        self.num_qubits = Some(n);
        self
    }

    /// Alias for qubits
    pub fn num_qubits(self, n: usize) -> Self {
        self.qubits(n)
    }

    /// Set the runtime plugin path (for backward compatibility)
    pub fn plugin<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.runtime_library_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set optimization flag (for API compatibility - currently ignored)
    pub fn optimize(self, _optimize: bool) -> Self {
        // Note: This method is provided for API compatibility with existing tests.
        // The Selene simple runtime doesn't have configurable optimization settings,
        // so this parameter is ignored.
        self
    }

    /// Set verbose flag (for API compatibility - currently ignored)
    pub fn verbose(self, _verbose: bool) -> Self {
        // Note: This method is provided for API compatibility with existing tests.
        // The Selene simple runtime doesn't have configurable verbose settings,
        // so this parameter is ignored.
        self
    }
}

impl Default for SeleneSimpleRuntimeEngineBuilder {
    fn default() -> Self {
        Self::new().default_runtime()
    }
}

impl ClassicalControlEngineBuilder for SeleneSimpleRuntimeEngineBuilder {
    type Engine = SeleneSimpleRuntimeEngine;

    fn build(self) -> Result<Self::Engine, PecosError> {
        let runtime_path = self.runtime_library_path.ok_or_else(|| {
            PecosError::Input(
                "SeleneSimpleRuntimeEngineBuilder requires a runtime library path".to_string(),
            )
        })?;

        if !runtime_path.exists() {
            return Err(PecosError::Resource(format!(
                "Selene runtime library not found at {:?}",
                runtime_path
            )));
        }

        let num_qubits = self.num_qubits.unwrap_or(10);

        let mut engine = SeleneSimpleRuntimeEngine::new(runtime_path, num_qubits)?;

        if let Some(program) = self.program {
            engine = engine.with_program(program);
        }

        Ok(engine)
    }
}

/// Create a new SeleneSimpleRuntimeEngine builder
///
/// This is the main entry point for creating SeleneSimpleRuntimeEngine instances.
///
/// # Example
///
/// ```rust,no_run
/// use pecos_selene::selene_simple_runtime;
/// use pecos_programs::SeleneInterfaceProgram;
/// use pecos_engines::ClassicalControlEngineBuilder;
///
/// // Load a compiled Selene Interface plugin
/// let plugin_bytes = std::fs::read("quantum_plugin.so").unwrap();
/// let program = SeleneInterfaceProgram::from_bytes(plugin_bytes);
///
/// // Create engine with default runtime location
/// let engine = selene_simple_runtime()
///     .default_runtime()
///     .selene_interface_program(program)
///     .qubits(5)
///     .build()
///     .unwrap();
/// ```
pub fn selene_simple_runtime() -> SeleneSimpleRuntimeEngineBuilder {
    SeleneSimpleRuntimeEngineBuilder::new()
}
