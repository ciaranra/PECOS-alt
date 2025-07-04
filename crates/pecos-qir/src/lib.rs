pub mod engine;
pub mod library;
pub mod linker; // Links LLVM IR programs with runtime library
pub mod llvm_utils; // LLVM utilities for entry point detection
pub mod platform;
pub mod prelude; // Convenient re-exports for common usage
pub mod runtime; // LLVM runtime implementation with submodules
pub mod utils; // Common utilities for error handling, logging, etc.

pub use engine::LlvmEngine;
