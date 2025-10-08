//! LLVM JIT-based QIS Interface
//!
//! This crate provides a `QisInterface` implementation that uses LLVM JIT compilation
//! to execute quantum programs and collect operations.

pub mod builder;
pub mod jit_executor;
pub mod jit_ffi;
pub mod jit_interface;
pub mod measurement_manager;
pub mod prelude;

pub use builder::{JitInterfaceBuilder, jit_interface_builder};
pub use jit_executor::JitExecutor;
pub use jit_interface::QisJitInterface;
pub use measurement_manager::{
    JitMeasurementManager, reset_measurement_manager, with_measurement_manager,
    with_measurement_manager_mut,
};
