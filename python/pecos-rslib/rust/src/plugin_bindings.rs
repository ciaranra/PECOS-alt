// Python bindings for Selene plugin compilation and HUGR to LLVM compilation

use pyo3::prelude::*;
use std::path::PathBuf;

/// Compile LLVM IR to a Selene plugin
///
/// This function takes LLVM IR (as a file path or string) and compiles it
/// into a Selene-compatible shared library plugin.
///
/// Args:
///     `llvm_source`: Path to LLVM IR file (.ll) or LLVM IR string
///     `output_dir`: Optional output directory for the plugin (defaults to temp dir)
///     name: Optional name for the plugin (defaults to "plugin")
///
/// Returns:
///     Path to the compiled plugin (.so file)
#[pyfunction]
#[pyo3(signature = (llvm_source, output_dir=None, name=None))]
pub fn compile_llvm_to_plugin(
    py: Python,
    llvm_source: String,
    output_dir: Option<String>,
    name: Option<String>,
) -> PyResult<String> {
    py.allow_threads(|| {
        use pecos_selene_plugins::plugin_builder::{LLVMSource, PluginBuildConfig, PluginBuilder};
        use std::fs;
        use std::path::Path;

        // Determine if llvm_source is a file path or IR string
        let source = if Path::new(&llvm_source).exists() {
            LLVMSource::IRFile(PathBuf::from(&llvm_source))
        } else {
            LLVMSource::IRString(llvm_source)
        };

        // Set up output directory
        let output_path = if let Some(dir) = output_dir {
            PathBuf::from(dir)
        } else {
            // Use temp directory
            std::env::temp_dir().join("selene_plugins")
        };

        // Create output directory if it doesn't exist
        fs::create_dir_all(&output_path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Set plugin name
        let plugin_name = name.unwrap_or_else(|| "plugin".to_string());

        // Build configuration
        let config = PluginBuildConfig {
            name: plugin_name,
            llvm_source: source,
            output_dir: output_path,
            verbose: false,
            link_flags: vec![],
            target_triple: None,
        };

        // Build the plugin
        let mut builder = PluginBuilder::new(config);
        let plugin_path = builder
            .build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(plugin_path.to_string_lossy().into_owned())
    })
}

/// Compile HUGR to LLVM IR
///
/// This function takes HUGR bytes (JSON format) and compiles them to LLVM IR
/// using the pecos-selene compiler.
///
/// Args:
///     `hugr_bytes`: HUGR program as JSON bytes
///
/// Returns:
///     LLVM IR as a string
#[pyfunction]
pub fn compile_hugr_to_llvm(hugr_bytes: &[u8]) -> PyResult<String> {
    #[cfg(feature = "hugr-013")]
    {
        use pecos_selene::hugr_to_llvm::compile_hugr_to_llvm as rust_compile_hugr_to_llvm;

        rust_compile_hugr_to_llvm(hugr_bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[cfg(not(feature = "hugr-013"))]
    {
        Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
            "compile_hugr_to_llvm requires pecos-rslib to be compiled with hugr-013 feature",
        ))
    }
}

/// Register plugin-related functions with the Python module
pub fn register_plugin_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_llvm_to_plugin, m)?)?;
    m.add_function(wrap_pyfunction!(compile_hugr_to_llvm, m)?)?;
    Ok(())
}
