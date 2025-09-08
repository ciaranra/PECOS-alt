//! Native Selene wrapper for PECOS
//!
//! This module provides a clean wrapper around Selene's natural workflow,
//! making it feel native to PECOS while using Selene as intended.

#![cfg(feature = "python")]

use anyhow::{Context, Result, anyhow};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Wrapper around Selene's natural execution model
pub struct SeleneNativeWrapper {
    /// Working directory for Selene builds
    work_dir: TempDir,
    /// Cached selene instance
    instance: Option<SeleneInstance>,
}

/// Represents a built Selene instance ready for execution
struct SeleneInstance {
    executable_path: PathBuf,
    artifact_dir: PathBuf,
    runtime_plugin: PathBuf,
}

impl SeleneNativeWrapper {
    /// Create a new Selene wrapper
    pub fn new() -> Result<Self> {
        let work_dir = TempDir::new().context("Failed to create temp directory for Selene")?;

        Ok(Self {
            work_dir,
            instance: None,
        })
    }

    /// Compile HUGR to executable using Selene's natural workflow
    pub fn compile_hugr(&mut self, hugr_bytes: &[u8]) -> Result<()> {
        // Use Selene's Python API through PyO3
        Python::with_gil(|py| {
            // Import selene_sim.build
            let build_module = py.import("selene_sim.build")?;
            let build_fn = build_module.getattr("build")?;

            // Create HUGR source
            let hugr_path = self.work_dir.path().join("program.hugr");
            std::fs::write(&hugr_path, hugr_bytes)?;

            // Build using Selene's natural API
            let instance = build_fn.call1((hugr_path,))?;

            // Extract paths from the instance
            let executable_path: String = instance.getattr("executable")?.extract()?;
            let artifact_dir: String = instance.getattr("artifact_dir")?.extract()?;

            self.instance = Some(SeleneInstance {
                executable_path: PathBuf::from(executable_path),
                artifact_dir: PathBuf::from(artifact_dir),
                runtime_plugin: self.get_runtime_plugin()?,
            });

            Ok(())
        })
    }

    /// Compile LLVM IR to executable using Selene's natural workflow
    pub fn compile_llvm_ir(&mut self, llvm_ir: &str) -> Result<()> {
        Python::with_gil(|py| {
            // Import selene_sim.build
            let build_module = py.import("selene_sim.build")?;
            let build_fn = build_module.getattr("build")?;

            // Create LLVM IR file
            let llvm_path = self.work_dir.path().join("program.ll");
            std::fs::write(&llvm_path, llvm_ir)?;

            // Build using Selene's natural API
            // This will use HeliosLLVMIRFileKind and compile through the natural pipeline
            let kwargs = PyDict::new(py);
            kwargs.set_item("verbose", false)?;

            let instance = build_fn.call((llvm_path,), Some(kwargs))?;

            // Store the instance for later execution
            self.cache_instance(instance)?;

            Ok(())
        })
    }

    /// Run the compiled program with quantum simulation
    pub fn run(&self, shots: usize, seed: Option<u64>) -> Result<Vec<HashMap<String, bool>>> {
        let instance = self
            .instance
            .as_ref()
            .ok_or_else(|| anyhow!("No compiled program available"))?;

        Python::with_gil(|py| {
            // Import necessary modules
            let quest = py.import("selene_sim")?.getattr("Quest")?;

            // Create simulator
            let simulator = quest.call0()?;
            if let Some(s) = seed {
                simulator.call_method1("seed", (s,))?;
            }

            // Run shots
            let results = instance.run_shots(simulator, shots)?;

            // Convert results to PECOS format
            self.convert_results(results)
        })
    }

    /// Get the ByteMessage runtime plugin for PECOS integration
    fn get_runtime_plugin(&self) -> Result<PathBuf> {
        // Use our ByteMessageSimulator as the runtime plugin
        // This provides the quantum operation interface that PECOS expects
        let plugin_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow!("Failed to get exe directory"))?
            .join("libpecos_selene_runtime.so");

        if !plugin_path.exists() {
            // Build it if needed
            self.build_runtime_plugin()?;
        }

        Ok(plugin_path)
    }

    fn build_runtime_plugin(&self) -> Result<()> {
        // This would compile our ByteMessageSimulator as a Selene runtime plugin
        // For now, we assume it's pre-built
        Ok(())
    }

    fn cache_instance(&mut self, py_instance: &PyAny) -> Result<()> {
        // Extract necessary paths from Python instance
        let executable_path: String = py_instance.getattr("executable")?.extract()?;
        let artifact_dir: String = py_instance.getattr("artifact_dir")?.extract()?;

        self.instance = Some(SeleneInstance {
            executable_path: PathBuf::from(executable_path),
            artifact_dir: PathBuf::from(artifact_dir),
            runtime_plugin: self.get_runtime_plugin()?,
        });

        Ok(())
    }

    fn convert_results(&self, py_results: &PyAny) -> Result<Vec<HashMap<String, bool>>> {
        // Convert Selene results to PECOS format
        let results: Vec<HashMap<String, bool>> = Vec::new();

        // TODO: Implement proper result conversion

        Ok(results)
    }
}

// Python interop is imported at the top of the file

impl SeleneInstance {
    fn run_shots(&self, simulator: &PyAny, shots: usize) -> Result<PyObject> {
        Python::with_gil(|py| {
            // Call the instance's run_shots method
            let run_shots = py.eval(
                &format!("lambda inst, sim, n: inst.run_shots(sim, n_qubits=10, n_shots=n)"),
                None,
                None,
            )?;

            let result =
                run_shots.call1((self.executable_path.to_str().unwrap(), simulator, shots))?;
            Ok(result.into())
        })
    }
}
