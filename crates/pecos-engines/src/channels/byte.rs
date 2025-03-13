//! Main exports for the byte-level message protocol
//!
//! This module provides a binary messaging protocol for efficient communication
//! between classical and quantum components.

// Re-export public components from submodules
pub use self::builder::ByteMessageBuilder;
pub use self::debug::dump_batch;
pub use self::gate_type::{GateTypeId, QuantumGate};

// Submodules
pub mod builder;
pub mod debug;
pub mod gate_type;
pub mod protocol;
