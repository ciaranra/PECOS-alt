//! Builder for QisControlEngine that integrates with PECOS sim() API

use crate::{QisControlEngine, NativeRuntime, SeleneRuntime, IntoQisInterface};
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_qis_interface::QisInterface;
use pecos_qis_runtime_trait::QisRuntime;
use std::path::{Path, PathBuf};

/// Builder for creating QisControlEngine instances
#[derive(Debug)]
pub struct QisEngineBuilder {
    runtime: RuntimeConfig,
    interface: Option<QisInterface>,
}

/// Configuration for the runtime to use
enum RuntimeConfig {
    Native,
    Selene { plugin_path: PathBuf },
    Custom(Box<dyn QisRuntime>),
}

impl std::fmt::Debug for RuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeConfig::Native => write!(f, "Native"),
            RuntimeConfig::Selene { plugin_path } => f.debug_struct("Selene")
                .field("plugin_path", plugin_path)
                .finish(),
            RuntimeConfig::Custom(_) => write!(f, "Custom(<runtime>)"),
        }
    }
}

impl QisEngineBuilder {
    /// Create a new builder with native runtime
    pub fn native() -> Self {
        Self {
            runtime: RuntimeConfig::Native,
            interface: None,
        }
    }

    /// Create a new builder with Selene runtime
    pub fn selene(plugin_path: impl AsRef<Path>) -> Self {
        Self {
            runtime: RuntimeConfig::Selene {
                plugin_path: plugin_path.as_ref().to_path_buf(),
            },
            interface: None,
        }
    }

    /// Use the default runtime (native)
    pub fn new() -> Self {
        Self::native()
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
    /// use pecos_qis_interface::QisInterface;
    /// use pecos_engines::ClassicalControlEngineBuilder;
    ///
    /// let interface = QisInterface::new();
    /// let builder = qis_control_engine()
    ///     .program(interface);
    ///
    /// // Builder is ready to use
    /// let engine = builder.build().unwrap();
    /// ```
    pub fn program(mut self, interface: QisInterface) -> Self {
        self.interface = Some(interface);
        self
    }

    /// Set the program to use from any supported program type
    ///
    /// This method accepts any type that can be converted to QisInterface,
    /// including QisProgram, HugrProgram, etc. Returns a Result because
    /// some conversions may fail (e.g., compilation errors).
    ///
    /// # Example
    /// ```rust
    /// use pecos_qis_ccengine::qis_control_engine;
    /// use pecos_programs::{QisProgram, HugrProgram};
    /// use pecos_engines::ClassicalControlEngineBuilder;
    ///
    /// // With QisProgram (when implemented)
    /// let qis_program = QisProgram::from_string("define void @main() { ret void }");
    /// // let builder = qis_control_engine().try_program(qis_program)?;
    ///
    /// // With HugrProgram (when implemented)
    /// let hugr_bytes = vec![/* HUGR data */];
    /// let hugr_program = HugrProgram::from_bytes(hugr_bytes);
    /// // let builder = qis_control_engine().try_program(hugr_program)?;
    /// ```
    pub fn try_program<P: IntoQisInterface>(mut self, program: P) -> Result<Self, PecosError> {
        let interface = program.into_qis_interface()?;
        self.interface = Some(interface);
        Ok(self)
    }

    /// Set the runtime to use
    ///
    /// This allows you to specify any runtime implementation.
    ///
    /// # Example
    /// ```rust
    /// use pecos_qis_ccengine::{qis_control_engine, NativeRuntime};
    /// use pecos_engines::ClassicalControlEngineBuilder;
    ///
    /// let builder = qis_control_engine()
    ///     .runtime(NativeRuntime::new());
    ///
    /// // Builder is ready to use
    /// let engine = builder.build().unwrap();
    /// ```
    pub fn runtime<R: QisRuntime + 'static>(mut self, runtime: R) -> Self {
        self.runtime = RuntimeConfig::Custom(Box::new(runtime));
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
            RuntimeConfig::Custom(runtime) => {
                log::info!("Creating QisControlEngine with custom runtime");
                runtime
            }
        };

        // Create the engine
        let mut engine = QisControlEngine::new(runtime);

        // Load interface if we have one
        if let Some(interface) = self.interface {
            log::info!("Loading interface with {} operations into engine", interface.operations.len());
            engine.load_interface(interface)?;
        } else {
            log::debug!("No interface provided to builder");
        }

        Ok(engine)
    }
}

/// Convenience function to create a QisEngineBuilder with native runtime
///
/// # Example
/// ```rust
/// use pecos_qis_ccengine::qis_control_engine;
/// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
///
/// let builder = qis_control_engine();
/// let engine = builder.build().unwrap();
/// assert_eq!(engine.num_qubits(), 0);
/// ```
pub fn qis_control_engine() -> QisEngineBuilder {
    QisEngineBuilder::native()
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