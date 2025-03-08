//! Main exports for the byte-level message protocol
//!
//! This module provides a binary messaging protocol for efficient communication
//! between classical and quantum components.

use std::io::{self, Read, Write};

// Re-export public components from submodules
pub use self::channel::ByteChannel;
pub use self::debug::dump_batch;
pub use self::gate_type::{GateTypeId, QuantumGate};

// Submodules
pub mod builder;
pub mod channel;
pub mod debug;
pub mod gate_type;
pub mod protocol;

/// Factory functions for creating byte channel instances
pub mod factory {
    use super::{ByteChannel, Read, Write, io};

    /// Create a byte channel using standard input and output
    pub fn from_stdio() -> io::Result<ByteChannel> {
        ByteChannel::from_stdio()
    }

    /// Create a byte channel for a single shot using pipes
    pub fn create_for_shot() -> io::Result<ByteChannel> {
        ByteChannel::create_for_shot()
    }

    /// Create a byte channel from custom readers and writers
    pub fn from_io<R, W>(reader: R, writer: W) -> ByteChannel
    where
        R: Read + Send + Sync + 'static,
        W: Write + Send + Sync + 'static,
    {
        ByteChannel::new(Box::new(reader), Box::new(writer))
    }
}
