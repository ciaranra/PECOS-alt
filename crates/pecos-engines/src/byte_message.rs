//! Byte message protocol for quantum operations
//!
//! This module provides a binary messaging protocol for efficient communication
//! between classical and quantum components.

pub use self::builder::ByteMessageBuilder;
pub use self::debug::dump_batch;
pub use self::gate_type::{GateType, QuantumGate};
pub use self::message::ByteMessage;
pub use self::message_data::MessageData;
pub use self::quantum_cmd::QuantumCmd;
pub use self::quantum_command::{CommandType, QuantumCommand};

// Re-export QubitId from pecos-core
pub use pecos_core::QubitId;

pub mod builder;
pub mod debug;
pub mod gate_type;
pub mod message;
pub mod message_data;
pub mod protocol;
pub mod quantum_cmd;
pub mod quantum_command;
