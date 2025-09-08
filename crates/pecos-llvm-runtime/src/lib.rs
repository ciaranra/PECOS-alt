pub mod engine;
pub mod interactive;
pub(crate) mod library;

/// LLVM linker module for compiling LLVM IR programs with the runtime library.
///
/// This module is primarily used internally by `LlvmEngine`, but is exposed
/// publicly to support advanced use cases such as:
/// - Pre-compiling LLVM IR programs
/// - Custom caching strategies
/// - Testing and benchmarking compilation performance
///
/// Most users should use `LlvmEngine` instead of interacting with the linker directly.
pub mod linker;

pub(crate) mod llvm_utils; // LLVM utilities for entry point detection
pub(crate) mod platform;
pub mod prelude; // Convenient re-exports for common usage
/// LLVM runtime implementation with submodules
///
/// This module is exposed to support static library generation for linking
/// with LLVM IR programs. Most users should use `LlvmEngine` instead of
/// interacting with the runtime directly.
#[doc(hidden)]
pub mod runtime;
pub(crate) mod utils; // Common utilities for error handling, logging, etc.

pub use engine::{LlvmEngine, LlvmEngineConfig};
