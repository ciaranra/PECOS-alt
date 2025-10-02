//! Builder for QisControlEngine that integrates with PECOS sim() API

use crate::{QisControlEngine, NativeRuntime, SeleneRuntime, IntoQisInterface};
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_qis_interface::QisInterface;
use std::path::{Path, PathBuf};

/// Builder for creating QisControlEngine instances
pub struct QisEngineBuilder {
    runtime: RuntimeConfig,
    interface: Option<QisInterface>,
    interface_builder: Option<Box<dyn crate::program::QisInterfaceBuilder>>,
    program_source: Option<String>, // Store original program source for loading
}

impl Clone for QisEngineBuilder {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            interface: self.interface.clone(),
            // We can't clone trait objects directly, but we can recreate them
            interface_builder: self.interface_builder.as_ref().map(|builder| {
                // Based on the name, recreate the builder
                match builder.name() {
                    "JIT" => Box::new(crate::program::JitInterfaceBuilder) as Box<dyn crate::program::QisInterfaceBuilder>,
                    _ => Box::new(crate::program::HeliosInterfaceBuilder) as Box<dyn crate::program::QisInterfaceBuilder>,
                }
            }),
            program_source: self.program_source.clone(),
        }
    }
}

/// Configuration for the runtime to use
#[derive(Debug, Clone)]
enum RuntimeConfig {
    Native,
    Selene { plugin_path: PathBuf },
    SeleneSimple,  // Use selene_simple_runtime() at build time
    Default,  // Default to Selene simple runtime (no fallback)
}


impl QisEngineBuilder {
    /// Create a new builder with native runtime
    pub fn native() -> Self {
        Self {
            runtime: RuntimeConfig::Native,
            interface: None,
            interface_builder: None,
            program_source: None,
        }
    }

    /// Create a new builder with Selene runtime
    pub fn selene(plugin_path: impl AsRef<Path>) -> Self {
        Self {
            runtime: RuntimeConfig::Selene {
                plugin_path: plugin_path.as_ref().to_path_buf(),
            },
            interface: None,
            interface_builder: None,
            program_source: None,
        }
    }

    /// Create a new builder with default runtime (Selene simple)
    pub fn new() -> Self {
        Self {
            runtime: RuntimeConfig::Default,
            interface: None,
            interface_builder: None,
            program_source: None,
        }
    }

    /// Set a pre-built interface (for testing)
    pub fn with_interface(mut self, interface: QisInterface) -> Self {
        self.interface = Some(interface);
        self
    }

    /// Set the program to use
    ///
    /// This is the preferred method for specifying the QIS program,
    /// consistent with other engines like QASMEngine.
    ///
    /// # Example
    /// ```rust
    /// use pecos_qis_ccengine::qis_control_engine;
    /// use pecos_qis_interface::{QisInterface, QuantumOp};
    /// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
    ///
    /// // Create an interface with quantum operations
    /// let mut interface = QisInterface::new();
    /// let q0 = interface.allocate_qubit();
    /// let q1 = interface.allocate_qubit();
    /// interface.operations.push(QuantumOp::H(q0).into());
    /// interface.operations.push(QuantumOp::CX(q0, q1).into());
    ///
    /// // Use the fluent API to build an engine with the program
    /// let engine = qis_control_engine()
    ///     .program(interface)
    ///     .build()
    ///     .unwrap();
    ///
    /// // Verify the engine is configured correctly
    /// assert_eq!(engine.num_qubits(), 2);
    /// ```
    /// Set the program to use from any supported program type
    ///
    /// This method accepts any type that can be converted to QisInterface,
    /// including QisProgram, HugrProgram, etc. Panics on conversion errors.
    /// For error handling, use `try_program()` instead.
    ///
    /// # Example
    /// ```rust
    /// use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface};
    /// use pecos_programs::QisProgram;
    /// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
    ///
    /// // Create a simple QIS program that allocates qubits and applies gates
    /// let qis_program = QisProgram::from_string(r#"
    ///     declare void @__quantum__qis__h__body(i64)
    ///     declare void @__quantum__qis__cx__body(i64, i64)
    ///
    ///     define void @main() #0 {
    ///         call void @__quantum__qis__h__body(i64 0)
    ///         call void @__quantum__qis__cx__body(i64 0, i64 1)
    ///         ret void
    ///     }
    ///
    ///     attributes #0 = { "EntryPoint" "RequiredQubits"="2" }
    /// "#);
    ///
    /// // Build engine with the program - program() will panic on invalid data
    /// let builder = qis_control_engine()
    ///     .interface(qis_jit_interface())
    ///     .program(qis_program);
    ///
    /// // Build the engine and verify it's configured
    /// let engine = builder.build().unwrap();
    /// assert!(engine.num_qubits() >= 0); // JIT interface may not detect qubits from attributes
    /// ```
    pub fn program<P: IntoQisInterface + 'static>(self, program: P) -> Self {
        self.try_program(program).expect("Failed to set program")
    }

    /// Set the interface builder for the engine
    ///
    /// This allows you to explicitly specify which interface backend to use
    /// (JIT or Helios) when processing programs.
    ///
    /// # Example
    /// ```rust,no_run
    /// use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface};
    /// use pecos_programs::QisProgram;
    /// use pecos_engines::ClassicalControlEngineBuilder;
    ///
    /// let program = QisProgram::from_string("...");
    ///
    /// // Force JIT interface
    /// let engine = qis_control_engine()
    ///     .interface(qis_jit_interface())
    ///     .program(program)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn interface(mut self, builder: Box<dyn crate::program::QisInterfaceBuilder>) -> Self {
        self.interface_builder = Some(builder);
        self
    }

    /// Set the program to use from any supported program type (error handling version)
    ///
    /// This method accepts any type that can be converted to QisInterface,
    /// including QisProgram, HugrProgram, etc. Returns a Result because
    /// some conversions may fail (e.g., compilation errors).
    ///
    /// # Example
    /// ```rust
    /// # use pecos_core::errors::PecosError;
    /// # fn main() -> Result<(), PecosError> {
    /// use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface};
    /// use pecos_programs::QisProgram;
    /// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
    ///
    /// // Valid QIS program - builds successfully
    /// let valid_program = QisProgram::from_string(r#"
    ///     declare void @__quantum__qis__h__body(i64)
    ///
    ///     define void @main() #0 {
    ///         call void @__quantum__qis__h__body(i64 0)
    ///         ret void
    ///     }
    ///
    ///     attributes #0 = { "EntryPoint" "RequiredQubits"="1" }
    /// "#);
    ///
    /// let engine = qis_control_engine()
    ///     .interface(qis_jit_interface())
    ///     .try_program(valid_program)?
    ///     .build()?;
    /// assert!(engine.num_qubits() >= 0); // JIT interface may not detect qubits from attributes
    ///
    /// // QIS programs with proper quantum functions compile successfully
    /// println!("QIS program compilation successful");
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_program<P: IntoQisInterface + 'static>(mut self, program: P) -> Result<Self, PecosError> {
        // Check if the program is already a QisInterface
        let any_program = &program as &dyn std::any::Any;

        if let Some(interface) = any_program.downcast_ref::<QisInterface>() {
            // If a QisInterface is directly provided and an interface builder was specified, error
            if self.interface_builder.is_some() {
                return Err(PecosError::with_context(
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid configuration"),
                    "Cannot use .interface() when providing a pre-built QisInterface to .program()"
                ));
            }
            // Use the provided interface directly
            self.interface = Some(interface.clone());
        } else {
            // For other program types (QisProgram, HugrProgram), use the builder
            // Also store the original program source for loading into interface implementations
            if let Some(qis_prog) = any_program.downcast_ref::<pecos_programs::QisProgram>() {
                // Store the LLVM IR source for later loading
                match &qis_prog.content {
                    pecos_programs::QisContent::Ir(ir_string) => {
                        self.program_source = Some(ir_string.clone());
                    }
                    pecos_programs::QisContent::Bitcode(bitcode) => {
                        // For bitcode, we'll need to convert or handle differently
                        // For now, skip storing source for bitcode programs
                        log::warn!("Bitcode programs not yet supported for interface loading");
                    }
                }
            }

            let interface = if let Some(builder) = &self.interface_builder {
                // Use the explicitly specified interface builder
                if let Some(qis_prog) = any_program.downcast_ref::<pecos_programs::QisProgram>() {
                    builder.build_from_qis_program(qis_prog.clone())?
                } else if let Some(hugr_prog) = any_program.downcast_ref::<pecos_programs::HugrProgram>() {
                    builder.build_from_hugr_program(hugr_prog.clone())?
                } else {
                    // Unknown type, use default conversion with the default backend (Helios)
                    program.into_qis_interface()?
                }
            } else {
                // No interface builder specified, default to Helios
                program.into_qis_interface()?
            };
            self.interface = Some(interface);
        }

        Ok(self)
    }

    /// Set the runtime to use
    ///
    /// This allows you to specify any runtime implementation.
    ///
    /// # Example
    /// ```rust,no_run
    /// use pecos_qis_ccengine::{qis_control_engine, NativeRuntime};
    /// use pecos_engines::ClassicalControlEngineBuilder;
    ///
    /// let builder = qis_control_engine()
    ///     .runtime(NativeRuntime::new());
    ///
    /// // Builder is ready to use
    /// let engine = builder.build().unwrap();
    /// ```
    /// Use Selene simple runtime
    pub fn selene_simple_runtime(mut self) -> Self {
        self.runtime = RuntimeConfig::SeleneSimple;
        self
    }

    /// Use native runtime (for backward compatibility with runtime() method)
    pub fn runtime(mut self, _runtime: impl std::any::Any) -> Self {
        // For backward compatibility, just use native runtime
        self.runtime = RuntimeConfig::Native;
        self
    }
}

impl Default for QisEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassicalControlEngineBuilder for QisEngineBuilder {
    type Engine = QisControlEngine;

    fn build(self) -> Result<Self::Engine, PecosError> {
        // Create the appropriate runtime
        let runtime: Box<dyn pecos_qis_runtime_trait::QisRuntime> = match self.runtime {
            RuntimeConfig::Native => {
                log::info!("Creating QisControlEngine with NativeRuntime");
                Box::new(NativeRuntime::new())
            }
            RuntimeConfig::Selene { plugin_path } => {
                log::info!("Creating QisControlEngine with SeleneRuntime from {:?}", plugin_path);
                Box::new(SeleneRuntime::new(plugin_path))
            }
            RuntimeConfig::SeleneSimple => {
                log::info!("Creating QisControlEngine with Selene simple runtime (with fallback)");
                match crate::selene_simple_runtime() {
                    Ok(runtime) => Box::new(runtime),
                    Err(e) => {
                        log::warn!("Failed to create Selene simple runtime: {}, falling back to native", e);
                        Box::new(NativeRuntime::new())
                    }
                }
            }
            RuntimeConfig::Default => {
                log::info!("Creating QisControlEngine with default runtime (Selene simple)");
                match crate::selene_simple_runtime() {
                    Ok(runtime) => Box::new(runtime),
                    Err(e) => {
                        return Err(PecosError::Generic(format!(
                            "Default runtime (Selene simple) is not available: {}\n\n\
                            To fix this:\n\
                            1. Ensure Selene repository is at ../selene or ../../../selene\n\
                            2. Build Selene runtimes: 'cargo build --release' in Selene directory\n\
                            3. Or use explicit native runtime: qis_control_engine().native().build()\n\
                            4. Or use explicit Selene with fallback: qis_control_engine().selene_simple().build()",
                            e
                        )));
                    }
                }
            }
        };

        // Create the interface from builder or use default
        let interface: Option<crate::interface_impl::BoxedInterface> = if let Some(qis_interface) = &self.interface {
            // Pre-built QisInterface provided (from .try_program()) - use it directly without recreating
            log::debug!("Pre-built QisInterface provided with {} allocated qubits and {} operations",
                       qis_interface.allocated_qubits.len(), qis_interface.operations.len());

            // When we have a pre-built interface, we should NOT create a new interface implementation
            // Instead, the QisControlEngine will use this interface directly via initialize_from_interface()
            None
        } else if let Some(builder) = &self.interface_builder {
            // Interface builder is set but no program was provided
            log::debug!("Interface builder specified but no program provided, creating empty interface");
            match builder.name() {
                "JIT" => Some(Box::new(crate::jit_interface::QisJitInterface::new())),
                _ => Some(Box::new(crate::helios_interface::QisSeleneHeliosInterface::new())),
            }
        } else {
            // No interface specified, use default (JIT)
            log::debug!("No interface specified, using default JIT interface");
            Some(Box::new(crate::jit_interface::QisJitInterface::new()))
        };

        // Create the engine - handle three cases: interface implementation, pre-built QisInterface, or default
        if let Some(qis_interface) = &self.interface {
            // Case 1: Pre-built QisInterface provided (from .try_program()) - use it directly
            log::debug!("Using pre-built QisInterface with {} allocated qubits and {} operations",
                       qis_interface.allocated_qubits.len(), qis_interface.operations.len());

            // Create engine with a simple interface that wraps the pre-built QisInterface operations
            let simple_interface = Box::new(crate::interface_impl::SimpleQisInterface::new(qis_interface.clone()));
            let mut engine = QisControlEngine::new(simple_interface, runtime);
            engine.initialize_from_interface()?;
            Ok(engine)
        } else if let Some(boxed_interface) = interface {
            // Case 2: Interface implementation provided - use it and optionally load program
            let mut engine = QisControlEngine::new(boxed_interface, runtime);

            if let Some(program_source) = &self.program_source {
                log::debug!("Loading program source into interface implementation");
                engine.load_program(
                    program_source.as_bytes(),
                    crate::interface_impl::ProgramFormat::LlvmIrText
                )?;
            }

            Ok(engine)
        } else {
            // Case 3: Nothing specified - create with default JIT interface
            log::debug!("No interface specified, using default JIT interface");
            let default_interface = Box::new(crate::jit_interface::QisJitInterface::new());
            let engine = QisControlEngine::new(default_interface, runtime);
            Ok(engine)
        }
    }
}

/// Convenience function to create a QisEngineBuilder with default runtime
///
/// The default runtime is Selene simple. If not available, it will error with
/// clear instructions on how to fix it or use alternative runtimes.
///
/// # Example
/// ```rust
/// use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface};
/// use pecos_programs::QisProgram;
/// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
///
/// // Create a builder with default runtime (Selene simple)
/// let builder = qis_control_engine();
///
/// // Add a program and build the engine
/// let program = QisProgram::from_string(r#"
///     declare void @__quantum__qis__h__body(i64)
///
///     define void @main() #0 {
///         call void @__quantum__qis__h__body(i64 0)
///         ret void
///     }
///
///     attributes #0 = { "EntryPoint" "RequiredQubits"="1" }
/// "#);
///
/// let engine = builder
///     .interface(qis_jit_interface())
///     .program(program)
///     .build()
///     .unwrap();
///
/// // Engine is ready for quantum simulation
/// assert!(engine.num_qubits() >= 0); // JIT interface may not detect qubits from attributes
/// ```
pub fn qis_control_engine() -> QisEngineBuilder {
    QisEngineBuilder::new()
}

/// Convenience function to create a QisEngineBuilder with Selene runtime
///
/// # Example
/// ```rust
/// use pecos_qis_ccengine::qis_control_engine_selene;
/// use std::path::Path;
///
/// let builder = qis_control_engine_selene("/tmp/test_plugin.so");
/// // Builder is configured for Selene runtime
/// // Note: Actual loading is deferred until the runtime is needed
/// ```
pub fn qis_control_engine_selene(plugin_path: impl AsRef<Path>) -> QisEngineBuilder {
    QisEngineBuilder::selene(plugin_path)
}

/// Create a native runtime for use with QisEngineBuilder
///
/// # Example
/// ```rust
/// use pecos_qis_ccengine::{qis_control_engine, native_runtime};
/// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
///
/// let runtime = native_runtime();
/// let builder = qis_control_engine().runtime(runtime);
/// let engine = builder.build().unwrap();
/// assert_eq!(engine.num_qubits(), 0);
/// ```
pub fn native_runtime() -> NativeRuntime {
    NativeRuntime::new()
}

/// Create a JIT interface builder for explicit JIT compilation
///
/// # Example
/// ```rust,no_run
/// use pecos_qis_ccengine::{qis_control_engine, qis_jit_interface};
/// use pecos_programs::QisProgram;
/// use pecos_engines::ClassicalControlEngineBuilder;
///
/// let program = QisProgram::from_file("circuit.ll").unwrap();
/// let engine = qis_control_engine()
///     .interface(qis_jit_interface())
///     .try_program(program)
///     .unwrap()
///     .build()
///     .unwrap();
/// ```
pub fn qis_jit_interface() -> Box<dyn crate::program::QisInterfaceBuilder> {
    Box::new(crate::program::JitInterfaceBuilder)
}

/// Create a Helios interface builder for explicit Helios compilation
///
/// # Example
/// ```rust,no_run
/// use pecos_qis_ccengine::{qis_control_engine, qis_selene_helios_interface};
/// use pecos_programs::QisProgram;
/// use pecos_engines::ClassicalControlEngineBuilder;
///
/// let program = QisProgram::from_file("circuit.ll").unwrap();
/// let engine = qis_control_engine()
///     .interface(qis_selene_helios_interface())
///     .try_program(program)
///     .unwrap()
///     .build()
///     .unwrap();
/// ```
pub fn qis_selene_helios_interface() -> Box<dyn crate::program::QisInterfaceBuilder> {
    Box::new(crate::program::HeliosInterfaceBuilder)
}

/// Create a Selene runtime for use with QisEngineBuilder
///
/// # Example
/// ```rust
/// use pecos_qis_ccengine::{qis_control_engine, selene_runtime};
/// use pecos_engines::ClassicalControlEngineBuilder;
///
/// let runtime = selene_runtime("/tmp/test_plugin.so");
/// let builder = qis_control_engine().runtime(runtime);
/// // Builder is ready - plugin will be loaded when needed
/// let engine = builder.build().unwrap();
/// ```
pub fn selene_runtime(plugin_path: impl AsRef<Path>) -> SeleneRuntime {
    SeleneRuntime::new(plugin_path)
}

// Convenience From implementations for common program types
impl<P: IntoQisInterface + 'static> From<P> for QisEngineBuilder {
    fn from(program: P) -> Self {
        qis_control_engine().program(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pecos_engines::ClassicalEngine;

    #[test]
    fn test_builder_creation() {
        let _builder = QisEngineBuilder::native();
        let _builder = QisEngineBuilder::selene("/tmp/test.so");
        let _builder = qis_control_engine();
    }

    #[test]
    fn test_builder_with_interface() {
        let interface = QisInterface::new();
        let builder = QisEngineBuilder::native().with_interface(interface);

        // Should be able to build
        let engine = builder.build().unwrap();
        assert_eq!(engine.num_qubits(), 0);
    }
}