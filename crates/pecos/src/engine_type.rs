//! Engine type enumeration and dynamic engine builder support
//!
//! This module provides tools for working with PECOS engines in both compile-time
//! and runtime contexts. It includes:
//!
//! - `EngineType`: An enumeration of all available engine types
//! - `DynamicEngineBuilder`: A type-erased wrapper for runtime engine selection
//!
//! # Overview
//!
//! PECOS provides multiple classical control engines (QASM, LLVM, Selene) for
//! executing quantum programs. Normally, you work with these engines directly:
//!
//! ```ignore
//! use pecos_qasm::qasm_engine;
//! use pecos_engines::sim;
//!
//! // Compile-time engine selection - best performance
//! let results = sim(qasm_engine().qasm("H q[0];"))
//!     .seed(42)
//!     .run(1000)?;
//! ```
//!
//! However, sometimes you need to select an engine at runtime based on user input,
//! configuration files, or other dynamic conditions. This module provides the tools
//! to do that.
//!
//! # Dynamic Engine Selection
//!
//! The `DynamicEngineBuilder` type uses trait objects to enable runtime engine
//! selection while maintaining the same API:
//!
//! ```ignore
//! use pecos::{EngineType, DynamicEngineBuilder, sim_dynamic};
//!
//! // Runtime engine selection based on user input
//! let engine_type = match user_input {
//!     "qasm" => EngineType::Qasm,
//!     "llvm" => EngineType::Llvm,
//!     "selene" => EngineType::Selene,
//!     _ => panic!("Unknown engine type"),
//! };
//!
//! // Create builder dynamically
//! let builder = match engine_type {
//!     EngineType::Qasm => DynamicEngineBuilder::new(qasm_engine().qasm("...")),
//!     EngineType::Llvm => DynamicEngineBuilder::new(llvm_engine().llvm_ir("...")),
//!     EngineType::Selene => DynamicEngineBuilder::new(selene_engine().hugr(...)),
//! };
//!
//! // Use the same API regardless of engine type
//! let results = sim_dynamic(builder).seed(42).run(1000)?;
//! ```
//!
//! # Performance Considerations
//!
//! Dynamic engine selection has a small runtime overhead due to trait object
//! indirection. For performance-critical code where the engine type is known
//! at compile time, prefer using the concrete engine builders directly.
//!
//! # Feature Flags
//!
//! The availability of engines depends on which features are enabled:
//! - `qasm`: Enables QASM engine support
//! - `llvm`: Enables LLVM engine support
//! - `selene`: Enables Selene engine support

use pecos_engines::{ClassicalControlEngineBuilder, ClassicalControlEngine, sim, SimBuilder};
use pecos_core::errors::PecosError;
use std::fmt;

/// Available engine types in PECOS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    /// QASM engine for OpenQASM 2.0 programs
    Qasm,
    /// LLVM engine for LLVM IR/bitcode programs
    Llvm,
    /// Selene engine for optimized quantum programs
    Selene,
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineType::Qasm => write!(f, "QASM"),
            EngineType::Llvm => write!(f, "LLVM"),
            EngineType::Selene => write!(f, "Selene"),
        }
    }
}

/// Dynamic engine builder that can hold any engine builder type
///
/// This type uses boxed trait objects to enable runtime engine selection.
/// It's useful when you need to dynamically choose between different engines
/// based on runtime conditions.
///
/// # Why Use DynamicEngineBuilder?
///
/// In Rust, each engine builder has its own concrete type (QasmEngineBuilder,
/// LlvmEngineBuilder, etc.). This is great for performance and type safety,
/// but it means you can't easily store different builders in the same variable
/// or collection. DynamicEngineBuilder solves this by wrapping any engine
/// builder in a type-erased container.
///
/// # Examples
///
/// ## Runtime engine selection from user input
/// ```ignore
/// use pecos::{EngineType, DynamicEngineBuilder, sim_dynamic};
/// 
/// fn create_engine_from_config(config: &Config) -> DynamicEngineBuilder {
///     match config.engine_type {
///         "qasm" => DynamicEngineBuilder::new(
///             qasm_engine().qasm(&config.source_code)
///         ),
///         "llvm" => DynamicEngineBuilder::new(
///             llvm_engine().llvm_ir(&config.source_code)
///         ),
///         _ => panic!("Unknown engine type"),
///     }
/// }
///
/// let engine = create_engine_from_config(&config);
/// let results = sim_dynamic(engine).seed(42).run(1000)?;
/// ```
///
/// ## Storing multiple engines in a collection
/// ```ignore
/// use std::collections::HashMap;
/// 
/// let mut engines = HashMap::new();
/// engines.insert("qasm", DynamicEngineBuilder::new(qasm_engine()));
/// engines.insert("llvm", DynamicEngineBuilder::new(llvm_engine()));
/// engines.insert("selene", DynamicEngineBuilder::new(selene_engine()));
/// 
/// // Select engine at runtime
/// let selected = engines.get(user_choice).unwrap();
/// ```
pub struct DynamicEngineBuilder {
    builder: Box<dyn DynamicEngineBuilderTrait>,
}

impl DynamicEngineBuilder {
    /// Create a new dynamic engine builder from any concrete engine builder
    pub fn new<B>(builder: B) -> Self 
    where 
        B: ClassicalControlEngineBuilder + 'static,
        B::Engine: 'static,
    {
        Self {
            builder: Box::new(ConcreteEngineBuilder(builder)),
        }
    }

    /// Create a dynamic engine builder from an EngineType
    ///
    /// This creates a default builder for the specified engine type.
    /// You'll need to configure it further with engine-specific methods.
    #[cfg(all(feature = "qasm", feature = "llvm", feature = "selene"))]
    pub fn from_type(engine_type: EngineType) -> Self {
        match engine_type {
            EngineType::Qasm => Self::new(pecos_qasm::qasm_engine()),
            EngineType::Llvm => Self::new(pecos_llvm_sim::llvm_engine()),
            EngineType::Selene => Self::new(pecos_selene_ceng::selene_engine()),
        }
    }
}

/// Internal trait for type erasure
trait DynamicEngineBuilderTrait {
    fn build(self: Box<Self>) -> Result<Box<dyn ClassicalControlEngine>, PecosError>;
}

/// Wrapper to implement the dynamic trait for concrete builders
struct ConcreteEngineBuilder<B: ClassicalControlEngineBuilder>(B);

impl<B> DynamicEngineBuilderTrait for ConcreteEngineBuilder<B>
where
    B: ClassicalControlEngineBuilder + 'static,
    B::Engine: 'static,
{
    fn build(self: Box<Self>) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
        Ok(Box::new(self.0.build()?))
    }
}

impl ClassicalControlEngineBuilder for DynamicEngineBuilder {
    type Engine = Box<dyn ClassicalControlEngine>;

    fn build(self) -> Result<Self::Engine, PecosError> {
        self.builder.build()
    }
}

/// Create a simulation builder from a dynamic engine builder
///
/// This allows using the sim() function with dynamic engine builders.
///
/// # Example
/// ```ignore
/// use pecos::{EngineType, DynamicEngineBuilder, sim_dynamic};
/// 
/// let engine = DynamicEngineBuilder::from_type(EngineType::Qasm);
/// let results = sim_dynamic(engine).seed(42).run(1000)?;
/// ```
pub fn sim_dynamic(builder: DynamicEngineBuilder) -> SimBuilder<DynamicEngineBuilder> {
    sim(builder)
}

/// Helper macro to create engine builders based on EngineType
///
/// This macro assumes the engine crates are available as dependencies.
///
/// # Example
/// ```ignore
/// use pecos::{create_engine_builder, EngineType};
/// 
/// let builder = create_engine_builder!(EngineType::Qasm);
/// ```
#[macro_export]
macro_rules! create_engine_builder {
    ($engine_type:expr) => {
        match $engine_type {
            $crate::EngineType::Qasm => {
                #[cfg(feature = "qasm")]
                {
                    $crate::DynamicEngineBuilder::new(pecos_qasm::qasm_engine())
                }
                #[cfg(not(feature = "qasm"))]
                {
                    panic!("QASM engine not available. Enable the 'qasm' feature.")
                }
            }
            $crate::EngineType::Llvm => {
                #[cfg(feature = "llvm")]
                {
                    $crate::DynamicEngineBuilder::new(pecos_llvm_sim::llvm_engine())
                }
                #[cfg(not(feature = "llvm"))]
                {
                    panic!("LLVM engine not available. Enable the 'llvm' feature.")
                }
            }
            $crate::EngineType::Selene => {
                #[cfg(feature = "selene")]
                {
                    $crate::DynamicEngineBuilder::new(pecos_selene_ceng::selene_engine())
                }
                #[cfg(not(feature = "selene"))]
                {
                    panic!("Selene engine not available. Enable the 'selene' feature.")
                }
            }
        }
    };
}