//! Protocol definitions for the byte-level messaging system
//!
//! This module defines the message formats, headers, and constants
//! used in the byte protocol.

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

// Magic bytes to identify PECOS message batches - "PEQS"
pub const BATCH_MAGIC: u32 = 0x5045_5153;
// Current protocol version
pub const PROTOCOL_VERSION: u8 = 1;

bitflags! {
    /// Flags that can be set on individual messages
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MessageFlags: u8 {
        const NONE          = 0b0000_0000;
        const LAST_MESSAGE  = 0b0000_0001; // Indicates last message in a sequence
        const ERROR         = 0b0000_0010; // Indicates error condition
        const RESERVED_1    = 0b0000_0100;
        const RESERVED_2    = 0b0000_1000;
        const RESERVED_3    = 0b0001_0000;
        const RESERVED_4    = 0b0010_0000;
        const RESERVED_5    = 0b0100_0000;
        const RESERVED_6    = 0b1000_0000;
    }
}

/// Message types used in the protocol
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MessageType {
    // Control messages
    BeginBatch = 1, // Start of command batch
    EndBatch = 2,   // End of command batch
    Flush = 3,      // Flush all pending operations
    Reset = 4,      // Reset state

    // Operation messages
    QuantumGate = 10, // Quantum gate operation
    Measurement = 11, // Measurement operation

    // Result messages
    MeasurementResult = 20, // Measurement result

    // Error messages
    Error = 100, // Error condition
}

/// Message batch header for framing multiple messages
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct BatchHeader {
    pub magic: u32,      // Magic number 'PEQS'
    pub version: u8,     // Protocol version
    pub flags: u8,       // Batch flags
    pub reserved: u16,   // Reserved for future use (padding for alignment)
    pub msg_count: u32,  // Number of messages in batch
    pub total_size: u32, // Total size in bytes including this header
}

impl BatchHeader {
    /// Create a new batch header
    #[must_use]
    pub fn new(msg_count: u32, total_size: u32) -> Self {
        Self {
            magic: BATCH_MAGIC,
            version: PROTOCOL_VERSION,
            flags: 0,
            reserved: 0,
            msg_count,
            total_size,
        }
    }

    /// Check if the header has a valid magic number and version
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.magic == BATCH_MAGIC && self.version == PROTOCOL_VERSION
    }
}

/// Individual message header
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MessageHeader {
    pub msg_type: u8,      // Message type
    pub flags: u8,         // Message flags
    pub reserved: u16,     // Reserved for future use (padding for alignment)
    pub payload_size: u32, // Size of payload following this header
}

impl MessageHeader {
    /// Create a new message header
    #[must_use]
    pub fn new(msg_type: MessageType, payload_size: u32, flags: MessageFlags) -> Self {
        Self {
            msg_type: msg_type as u8,
            flags: flags.bits(),
            reserved: 0,
            payload_size,
        }
    }

    /// Get the message type from a raw header
    pub fn get_type(&self) -> Result<MessageType, &'static str> {
        match self.msg_type {
            1 => Ok(MessageType::BeginBatch),
            2 => Ok(MessageType::EndBatch),
            3 => Ok(MessageType::Flush),
            4 => Ok(MessageType::Reset),
            10 => Ok(MessageType::QuantumGate),
            11 => Ok(MessageType::Measurement),
            20 => Ok(MessageType::MeasurementResult),
            100 => Ok(MessageType::Error),
            _ => Err("Unknown message type"),
        }
    }

    /// Get the message flags from a raw header
    #[must_use]
    pub fn get_flags(&self) -> MessageFlags {
        MessageFlags::from_bits_truncate(self.flags)
    }
}

/// Quantum gate message payload header
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct QuantumGateHeader {
    pub gate_type: u8,  // Gate type (using GateType enum values)
    pub num_qubits: u8, // Number of qubits
    pub has_params: u8, // Whether gate has parameters (1=yes, 0=no)
    pub reserved: u8,   // Reserved for future use (padding for alignment)
                        // Followed by:
                        // - qubit_indices: [u32; num_qubits]
                        // - parameters: depends on gate type (if has_params=1)
}

/// Measurement message payload header
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MeasurementHeader {
    pub qubit: u32, // Qubit index
    pub result_id: u32, // Result identifier
                    // No additional data
}

/// Measurement result message payload header
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MeasurementResultHeader {
    pub result_id: u32, // Result identifier
    pub outcome: u32,   // Measurement outcome (0 or 1, but u32 for alignment)
}

/// Calculate padding needed for alignment
#[must_use]
pub fn calc_padding(offset: usize, alignment: usize) -> usize {
    (alignment - (offset % alignment)) % alignment
}
