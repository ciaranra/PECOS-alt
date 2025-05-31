//! Byte message protocol for quantum operations
//!
//! This module provides a binary messaging protocol for efficient communication
//! between classical and quantum components.

pub mod builder;
pub mod debug;
pub mod message;
pub mod protocol;

pub use builder::ByteMessageBuilder;
pub use debug::dump_batch;
pub use message::ByteMessage;
pub use pecos_core::gate_type::GateType;
pub use pecos_core::gates::Gate;

// Re-export QubitId from pecos-core
pub use pecos_core::QubitId;
