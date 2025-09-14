use anyhow::{Context, Result, anyhow};
/// Plugin builder for creating Selene-compatible plugins from LLVM programs
/// This replicates Selene's Python build process in Rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Convert a string to `PascalCase`
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Represents a plugin build configuration
#[derive(Debug, Clone)]
pub struct PluginBuildConfig {
    /// Name of the plugin
    pub name: String,
    /// LLVM IR or bitcode source
    pub llvm_source: LLVMSource,
    /// Output directory for artifacts
    pub output_dir: PathBuf,
    /// Verbose output
    pub verbose: bool,
    /// Additional link flags
    pub link_flags: Vec<String>,
    /// Target triple (optional, defaults to host)
    pub target_triple: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LLVMSource {
    /// LLVM IR text file (.ll)
    IRFile(PathBuf),
    /// LLVM IR text string
    IRString(String),
    /// LLVM bitcode file (.bc)
    BitcodeFile(PathBuf),
    /// LLVM bitcode bytes
    BitcodeBytes(Vec<u8>),
}

/// Builder for creating Selene-compatible plugins from LLVM programs
pub struct PluginBuilder {
    config: PluginBuildConfig,
    temp_dir: Option<TempDir>,
}

impl PluginBuilder {
    /// Create a new plugin builder
    #[must_use]
    pub fn new(config: PluginBuildConfig) -> Self {
        Self {
            config,
            temp_dir: None,
        }
    }

    /// Build the plugin library
    ///
    /// # Errors
    ///
    /// Returns an error if compilation or linking fails
    pub fn build(&mut self) -> Result<PathBuf> {
        // Create output directory
        fs::create_dir_all(&self.config.output_dir).context("Failed to create output directory")?;

        // Create temporary directory for intermediate files
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
        self.temp_dir = Some(temp_dir);

        // Get path to LLVM source file
        let llvm_file = self.prepare_llvm_source()?;

        // Compile LLVM to object file
        let object_file = self.compile_to_object(&llvm_file)?;

        // Create plugin wrapper
        let wrapper_file = self.create_plugin_wrapper()?;

        // Compile wrapper
        let wrapper_object = self.compile_wrapper(&wrapper_file)?;

        // Link everything into a shared library
        let plugin_lib = self.link_plugin(&object_file, &wrapper_object)?;

        Ok(plugin_lib)
    }

    /// Prepare LLVM source file from various input formats
    fn prepare_llvm_source(&self) -> Result<PathBuf> {
        let temp_dir = self.temp_dir.as_ref().unwrap().path();

        match &self.config.llvm_source {
            LLVMSource::IRFile(path) | LLVMSource::BitcodeFile(path) => Ok(path.clone()),
            LLVMSource::IRString(ir) => {
                let ir_path = temp_dir.join("program.ll");
                fs::write(&ir_path, ir).context("Failed to write LLVM IR to file")?;
                Ok(ir_path)
            }
            LLVMSource::BitcodeBytes(bytes) => {
                let bc_path = temp_dir.join("program.bc");
                fs::write(&bc_path, bytes).context("Failed to write LLVM bitcode to file")?;
                Ok(bc_path)
            }
        }
    }

    /// Compile LLVM source to object file
    fn compile_to_object(&self, llvm_file: &Path) -> Result<PathBuf> {
        let temp_dir = self.temp_dir.as_ref().unwrap().path();
        let object_file = temp_dir.join("program.o");

        if self.config.verbose {
            log::info!("Compiling LLVM to object file: {}", object_file.display());
        }

        let mut cmd = Command::new("clang");
        cmd.arg("-c").arg(llvm_file).arg("-o").arg(&object_file);

        if let Some(target) = &self.config.target_triple {
            cmd.arg("-target").arg(target);
        }

        let output = cmd.output().context("Failed to execute clang")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to compile LLVM: {}", stderr));
        }

        Ok(object_file)
    }

    /// Create the plugin wrapper that implements `RuntimeInterface`
    #[allow(clippy::too_many_lines)]
    fn create_plugin_wrapper(&self) -> Result<PathBuf> {
        let temp_dir = self.temp_dir.as_ref().unwrap().path();
        let wrapper_path = temp_dir.join("plugin_wrapper.rs");

        // Convert name to PascalCase for Rust types
        let type_name = to_pascal_case(&self.config.name);

        let wrapper_code = format!(
            r#"
use selene_core::{{
    runtime::{{BatchOperation, RuntimeInterface, interface::RuntimeInterfaceFactory}},
    utils::MetricValue,
    time::Instant,
    export_runtime_plugin,
}};
use anyhow::{{Result, anyhow}};
use std::sync::Arc;

// External LLVM functions
extern "C" {{
    fn setup() -> i32;
    fn teardown() -> i32;
    fn get_tc() -> f64;
    fn get_next_operations(buf: *mut u8, len: usize) -> usize;
}}

/// Plugin implementation for {type_name}
pub struct {type_name}Plugin {{
    n_qubits: u64,
    shot_id: u64,
    initialized: bool,
}}

impl RuntimeInterface for {type_name}Plugin {{
    fn exit(&mut self) -> Result<()> {{
        if self.initialized {{
            unsafe {{
                if teardown() != 0 {{
                    return Err(anyhow!("teardown() failed"));
                }}
            }}
            self.initialized = false;
        }}
        Ok(())
    }}

    fn get_next_operations(&mut self) -> Result<Option<BatchOperation>> {{
        if !self.initialized {{
            return Ok(None);
        }}

        // Allocate buffer for operations
        let mut buffer = vec![0u8; 4096];
        let bytes_written = unsafe {{
            get_next_operations(buffer.as_mut_ptr(), buffer.len())
        }};

        if bytes_written == 0 {{
            return Ok(None);
        }}

        buffer.truncate(bytes_written);

        // Parse the buffer into BatchOperation
        // This is a simplified version - real implementation would parse the actual format
        // Create an empty batch operation
        let start_time = unsafe {{ std::mem::zeroed::<Instant>() }};
        Ok(Some(BatchOperation::new(vec![], start_time, Default::default())))
    }}

    fn shot_start(&mut self, shot_id: u64, _seed: u64) -> Result<()> {{
        self.shot_id = shot_id;
        if !self.initialized {{
            unsafe {{
                if setup() != 0 {{
                    return Err(anyhow!("setup() failed"));
                }}
            }}
            self.initialized = true;
        }}
        Ok(())
    }}

    fn shot_end(&mut self) -> Result<()> {{
        Ok(())
    }}

    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {{
        if nth_metric == 0 {{
            let tc = unsafe {{ get_tc() }};
            Ok(Some(("tc".to_string(), MetricValue::F64(tc))))
        }} else {{
            Ok(None)
        }}
    }}

    fn qalloc(&mut self) -> Result<u64> {{
        // Delegate to LLVM program via external call
        Ok(u64::MAX) // No free qubits
    }}

    fn qfree(&mut self, _qubit_id: u64) -> Result<()> {{
        Ok(())
    }}

    fn rxy_gate(&mut self, _qubit_id: u64, _theta: f64, _phi: f64) -> Result<()> {{
        Ok(())
    }}

    fn rzz_gate(&mut self, _qubit_id_1: u64, _qubit_id_2: u64, _theta: f64) -> Result<()> {{
        Ok(())
    }}

    fn rz_gate(&mut self, _qubit_id: u64, _theta: f64) -> Result<()> {{
        Ok(())
    }}

    fn measure(&mut self, _qubit_id: u64) -> Result<u64> {{
        Ok(0)
    }}

    fn measure_leaked(&mut self, _qubit_id: u64) -> Result<u64> {{
        Ok(0)
    }}

    fn reset(&mut self, _qubit_id: u64) -> Result<()> {{
        Ok(())
    }}

    fn force_result(&mut self, _result_id: u64) -> Result<()> {{
        Ok(())
    }}

    fn get_bool_result(&mut self, _result_id: u64) -> Result<Option<bool>> {{
        Ok(None)
    }}

    fn get_u64_result(&mut self, _result_id: u64) -> Result<Option<u64>> {{
        Ok(None)
    }}

    fn set_bool_result(&mut self, _result_id: u64, _result: bool) -> Result<()> {{
        Ok(())
    }}

    fn set_u64_result(&mut self, _result_id: u64, _result: u64) -> Result<()> {{
        Ok(())
    }}

    fn increment_future_refcount(&mut self, _future: u64) -> Result<()> {{
        Ok(())
    }}

    fn decrement_future_refcount(&mut self, _future: u64) -> Result<()> {{
        Ok(())
    }}

    fn local_barrier(&mut self, _qubits: &[u64], _sleep_ns: u64) -> Result<()> {{
        Ok(())
    }}

    fn global_barrier(&mut self, _sleep_ns: u64) -> Result<()> {{
        Ok(())
    }}

    fn custom_call(&mut self, _tag: u64, _data: &[u8]) -> Result<u64> {{
        Err(anyhow!("Custom calls not supported"))
    }}
}}

/// Factory for creating plugin instances
#[derive(Default)]
pub struct {type_name}PluginFactory;

impl RuntimeInterfaceFactory for {type_name}PluginFactory {{
    type Interface = {type_name}Plugin;

    fn init(
        self: Arc<Self>,
        n_qubits: u64,
        _start: Instant,
        _args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {{
        Ok(Box::new({type_name}Plugin {{
            n_qubits,
            shot_id: 0,
            initialized: false,
        }}))
    }}
}}

// Export the plugin
export_runtime_plugin!(crate::{type_name}PluginFactory);
"#
        );

        fs::write(&wrapper_path, wrapper_code).context("Failed to write plugin wrapper")?;

        Ok(wrapper_path)
    }

    /// Compile the Rust wrapper
    fn compile_wrapper(&self, wrapper_file: &Path) -> Result<PathBuf> {
        let temp_dir = self.temp_dir.as_ref().unwrap().path();
        let wrapper_object = temp_dir.join("wrapper.o");

        if self.config.verbose {
            log::info!("Compiling plugin wrapper: {}", wrapper_object.display());
        }

        // Create a temporary Cargo project
        let cargo_dir = temp_dir.join("plugin_crate");
        fs::create_dir_all(&cargo_dir)?;

        // Write Cargo.toml
        let cargo_toml = format!(
            r#"
[package]
name = "{}-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
selene-core = {{ path = "{}/Repos/cl_projects/gup/selene/selene-core" }}
anyhow = "1.0"
"#,
            self.config.name,
            env!("HOME")
        );

        fs::write(cargo_dir.join("Cargo.toml"), cargo_toml)?;

        // Copy wrapper to src/lib.rs
        let src_dir = cargo_dir.join("src");
        fs::create_dir_all(&src_dir)?;
        fs::copy(wrapper_file, src_dir.join("lib.rs"))?;

        // Build with cargo
        let output = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&cargo_dir)
            .output()
            .context("Failed to execute cargo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to compile wrapper: {}", stderr));
        }

        // Extract the compiled library
        let lib_name = format!("lib{}_plugin.so", self.config.name.replace('-', "_"));
        let lib_path = cargo_dir.join("target/release").join(&lib_name);

        Ok(lib_path)
    }

    /// Link the LLVM object and wrapper into final plugin
    fn link_plugin(&self, _object_file: &Path, wrapper_lib: &Path) -> Result<PathBuf> {
        let plugin_name = format!("lib{}_plugin.so", self.config.name);
        let plugin_path = self.config.output_dir.join(&plugin_name);

        if self.config.verbose {
            log::info!("Linking plugin: {}", plugin_path.display());
        }

        // For now, just copy the wrapper library as it already includes everything
        // In a real implementation, we'd link the LLVM object file with the wrapper
        fs::copy(wrapper_lib, &plugin_path).context("Failed to copy plugin library")?;

        Ok(plugin_path)
    }
}

/// Build a plugin from HUGR
///
/// # Errors
///
/// Currently unimplemented - will always panic with todo!()
pub fn build_plugin_from_hugr(
    name: &str,
    hugr_bytes: &[u8],
    output_dir: &Path,
    verbose: bool,
) -> Result<PathBuf> {
    use crate::hugr_compiler::compile_hugr_to_llvm;

    // Compile HUGR to LLVM
    let llvm_ir = compile_hugr_to_llvm(hugr_bytes)?;

    // Build plugin
    let config = PluginBuildConfig {
        name: name.to_string(),
        llvm_source: LLVMSource::IRString(llvm_ir),
        output_dir: output_dir.to_path_buf(),
        verbose,
        link_flags: vec![],
        target_triple: None,
    };

    let mut builder = PluginBuilder::new(config);
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_builder_creation() {
        let config = PluginBuildConfig {
            name: "test_plugin".to_string(),
            llvm_source: LLVMSource::IRString(String::new()),
            output_dir: PathBuf::from("/tmp/test"),
            verbose: false,
            link_flags: vec![],
            target_triple: None,
        };

        let builder = PluginBuilder::new(config);
        assert_eq!(builder.config.name, "test_plugin");
    }
}
