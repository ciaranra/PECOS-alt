//! Channel interfaces for communication between engine components
//!
//! This module provides the core communication interfaces and implementations
//! for the PECOS simulator.

pub mod byte;
pub mod byte_message;

// Re-export the byte message for easy access
pub use byte::ByteMessageBuilder;
pub use byte_message::ByteMessage;
