//! Byte message protocol for quantum operations
//!
//! This module provides a binary messaging protocol for efficient communication
//! between classical and quantum components.

pub mod builder;
pub mod debug;
pub mod gate_type;
pub mod message;
pub mod message_data;
pub mod protocol;
pub mod quantum_cmd;
pub mod quantum_command;

pub use builder::ByteMessageBuilder;
pub use debug::dump_batch;
pub use gate_type::GateType;
pub use gate_type::QuantumGate;
pub use message::ByteMessage;
pub use message_data::MessageData;
pub use quantum_cmd::QuantumCmd;
pub use quantum_command::{CommandType, QuantumCommand};

// Re-export QubitId from pecos-core
pub use pecos_core::QubitId;
