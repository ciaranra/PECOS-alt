//! Builder for QisControlEngine that integrates with PECOS sim() API

use crate::{QisControlEngine, IntoQisInterface};
use pecos_core::errors::PecosError;
use pecos_engines::ClassicalControlEngineBuilder;
use pecos_qis_ffi::OperationCollector;

/// Builder for creating QisControlEngine instances
pub struct QisEngineBuilder {
    runtime: Option<Box<dyn crate::runtime::QisRuntime>>,
    interface: Option<OperationCollector>,
    interface_builder: Option<Box<dyn crate::program::QisInterfaceBuilder>>,
    program_source: Option<String>, // Store original program source for loading
}

impl Clone for QisEngineBuilder {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.as_ref().map(|r| dyn_clone::clone_box(&**r)),
            interface: self.interface.clone(),
            // Clone the interface builder if present
            interface_builder: self.interface_builder.as_ref().map(|b| dyn_clone::clone_box(&**b)),
            program_source: self.program_source.clone(),
        }
    }
}


impl QisEngineBuilder {
    /// Create a new builder without a runtime (user must call .runtime())
    pub fn new() -> Self {
        Self {
            runtime: None,
            interface: None,
            interface_builder: None,
            program_source: None,
        }
    }

    /// Set a pre-built interface (for testing)
    pub fn with_interface(mut self, interface: OperationCollector) -> Self {
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
    /// use pecos_qis_core::qis_control_engine;
    /// use pecos_qis_ffi::{OperationCollector, QuantumOp};
    /// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
    /// use pecos_qis_native::native_runtime;
    ///
    /// // Create an interface with quantum operations
    /// let mut interface = OperationCollector::new();
    /// let q0 = interface.allocate_qubit();
    /// let q1 = interface.allocate_qubit();
    /// interface.operations.push(QuantumOp::H(q0).into());
    /// interface.operations.push(QuantumOp::CX(q0, q1).into());
    ///
    /// // Use the fluent API to build an engine with the program
    /// let engine = qis_control_engine()
    ///     .runtime(native_runtime())
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
    /// use pecos_qis_core::qis_control_engine;
    /// use pecos_qis_ffi::{OperationCollector, QuantumOp};
    /// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
    /// use pecos_qis_native::native_runtime;
    ///
    /// // Create an interface with quantum operations
    /// let mut interface = OperationCollector::new();
    /// let q0 = interface.allocate_qubit();
    /// let q1 = interface.allocate_qubit();
    /// interface.operations.push(QuantumOp::H(q0).into());
    /// interface.operations.push(QuantumOp::CX(q0, q1).into());
    ///
    /// // Build engine with the program - program() will panic on invalid data
    /// let builder = qis_control_engine()
    ///     .runtime(native_runtime())
    ///     .program(interface);
    ///
    /// // Build the engine and verify it's configured
    /// let engine = builder.build().unwrap();
    /// assert_eq!(engine.num_qubits(), 2);
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
    /// ```no_run
    /// use pecos_qis_core::qis_control_engine;
    /// use pecos_programs::QisProgram;
    /// use pecos_engines::ClassicalControlEngineBuilder;
    /// use pecos_qis_native::native_runtime;
    ///
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
    /// // When using .interface(), you specify which backend to use
    /// // This example requires a QisInterfaceBuilder implementation
    /// # let interface_builder = Box::new(pecos_qis_core::interface_impl::SimpleQisInterface::new(pecos_qis_ffi::OperationCollector::new())) as Box<dyn pecos_qis_core::QisInterface>;
    /// let engine = qis_control_engine()
    ///     .runtime(native_runtime())
    ///     .program(program)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn interface(mut self, builder: impl crate::program::QisInterfaceBuilder + 'static) -> Self {
        self.interface_builder = Some(Box::new(builder));
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
    /// use pecos_qis_core::qis_control_engine;
    /// use pecos_qis_ffi::{OperationCollector, QuantumOp};
    /// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
    /// use pecos_qis_native::native_runtime;
    ///
    /// // Create an interface with quantum operations
    /// let mut interface = OperationCollector::new();
    /// let q0 = interface.allocate_qubit();
    /// interface.operations.push(QuantumOp::H(q0).into());
    ///
    /// let engine = qis_control_engine()
    ///     .runtime(native_runtime())
    ///     .try_program(interface)?
    ///     .build()?;
    /// assert_eq!(engine.num_qubits(), 1);
    ///
    /// // QIS programs with proper quantum functions compile successfully
    /// println!("QIS program compilation successful");
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_program<P: IntoQisInterface + 'static>(mut self, program: P) -> Result<Self, PecosError> {
        // Check if the program is already an OperationCollector
        let any_program = &program as &dyn std::any::Any;

        if let Some(interface) = any_program.downcast_ref::<OperationCollector>() {
            // If an OperationCollector is directly provided and an interface builder was specified, error
            if self.interface_builder.is_some() {
                return Err(PecosError::with_context(
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid configuration"),
                    "Cannot use .interface() when providing a pre-built OperationCollector to .program()"
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
                    pecos_programs::QisContent::Bitcode(_bitcode) => {
                        // For bitcode, we'll need to convert or handle differently
                        // For now, skip storing source for bitcode programs
                        log::warn!("Bitcode programs not yet supported for interface loading");
                    }
                }
            }

            let interface = if let Some(builder) = &self.interface_builder {
                // Use the explicitly specified interface builder
                log::debug!("Using interface builder: {}", builder.name());
                if let Some(qis_prog) = any_program.downcast_ref::<pecos_programs::QisProgram>() {
                    log::debug!("Building interface from QIS program");
                    builder.build_from_qis_program(qis_prog.clone())?
                } else if let Some(hugr_prog) = any_program.downcast_ref::<pecos_programs::HugrProgram>() {
                    log::debug!("Building interface from HUGR program");
                    builder.build_from_hugr_program(hugr_prog.clone())?
                } else {
                    // Unknown type, use default conversion with the default backend (Helios)
                    log::debug!("Unknown program type, using into_qis_interface");
                    program.into_qis_interface()?
                }
            } else {
                // No interface builder specified, default to Helios
                log::debug!("No interface builder specified, using into_qis_interface");
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
    /// ```rust
    /// use pecos_qis_core::qis_control_engine;
    /// use pecos_qis_native::native_runtime;
    /// use pecos_engines::ClassicalControlEngineBuilder;
    /// use pecos_qis_ffi::OperationCollector;
    ///
    /// let builder = qis_control_engine()
    ///     .runtime(native_runtime());
    ///
    /// // Need to add a program before building
    /// let interface = OperationCollector::new();
    /// let engine = builder.program(interface).build().unwrap();
    /// ```
    /// Set the runtime to use for classical control flow
    ///
    /// The runtime must implement the `QisRuntime` trait. Common runtimes:
    /// - `pecos_qis_native::NativeRuntime` - Pure Rust implementation
    /// - `pecos_qis_selene::SeleneRuntime` - Selene-based implementation
    pub fn runtime(mut self, runtime: impl crate::runtime::QisRuntime + 'static) -> Self {
        self.runtime = Some(Box::new(runtime));
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
        // Check that a runtime was provided
        let runtime = self.runtime.ok_or_else(|| {
            PecosError::Processing(
                "No runtime specified. Please provide a runtime using .runtime().\n\
                Common runtimes:\n\
                - pecos_qis_native::native_runtime() - Pure Rust implementation\n\
                - pecos_qis_selene::SeleneRuntime - Selene-based implementation\n\
                Example: qis_control_engine().runtime(native_runtime()).build()".to_string()
            )
        })?;

        // Create the interface from builder or use default
        let interface: Option<crate::qis_interface::BoxedInterface> = if let Some(qis_interface) = &self.interface {
            // Pre-built QisInterface provided (from .try_program()) - use it directly without recreating
            log::debug!("Pre-built QisInterface provided with {} allocated qubits and {} operations",
                       qis_interface.allocated_qubits.len(), qis_interface.operations.len());

            // When we have a pre-built interface, we should NOT create a new interface implementation
            // Instead, the QisControlEngine will use this interface directly via initialize_from_interface()
            None
        } else if let Some(_builder) = &self.interface_builder {
            // Interface builder is set but no program was provided - return error
            log::debug!("Interface builder specified but no program was provided");
            return Err(PecosError::Processing(
                "Interface builder specified but no program provided.\n\
                Please provide a program using .program() or .try_program()".to_string()
            ));
        } else {
            // No interface specified, return error - user must provide implementation
            log::debug!("No interface specified - will return error if no interface is provided");
            None
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
                    crate::qis_interface::ProgramFormat::LlvmIrText
                )?;
            }

            Ok(engine)
        } else {
            // Case 3: Nothing specified - error, user must provide an interface implementation
            Err(PecosError::Processing(
                "No interface implementation provided. Please specify an interface using:\n\
                - .program() to load from a program (uses default Selene Helios interface)\n\
                - .try_program() for explicit interface selection\n\
                - Or import pecos-qis-jit or pecos-qis-selene and create an interface directly".to_string()
            ))
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
/// use pecos_qis_core::qis_control_engine;
/// use pecos_qis_ffi::{OperationCollector, QuantumOp};
/// use pecos_engines::{ClassicalControlEngineBuilder, ClassicalEngine};
/// use pecos_qis_native::native_runtime;
///
/// // Create a builder (you must specify a runtime)
/// let builder = qis_control_engine();
///
/// // Create an interface with quantum operations
/// let mut interface = OperationCollector::new();
/// let q0 = interface.allocate_qubit();
/// interface.operations.push(QuantumOp::H(q0).into());
///
/// let engine = builder
///     .runtime(native_runtime())
///     .program(interface)
///     .build()
///     .unwrap();
///
/// // Engine is ready for quantum simulation
/// assert_eq!(engine.num_qubits(), 1);
/// ```
pub fn qis_control_engine() -> QisEngineBuilder {
    QisEngineBuilder::new()
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

    #[test]
    fn test_builder_creation() {
        // Basic builder creation - doesn't require a runtime
        let _builder = qis_control_engine();
    }

    // Note: Full builder tests with runtime and interface are in integration tests
    // in pecos-qis-native and pecos-qis-selene crates, since those have the actual
    // runtime implementations available.
}