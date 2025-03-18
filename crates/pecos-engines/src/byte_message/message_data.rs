use std::fmt;

/// Message data for quantum operations
///
/// This enum represents various types of messages that can be sent
/// between classical and quantum components.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageData {
    /// Informational message
    Info(String),

    /// Warning message
    Warning(String),

    /// Error message
    Error(String),

    /// Debug message
    Debug(String),

    /// Raw message for backward compatibility
    Raw(String),
}

impl MessageData {
    /// Create a new informational message
    ///
    /// # Arguments
    ///
    /// * `msg` - The message content
    ///
    /// # Returns
    ///
    /// A new `MessageData::Info` variant
    #[must_use]
    pub fn info(msg: String) -> Self {
        MessageData::Info(msg)
    }

    /// Create a new warning message
    ///
    /// # Arguments
    ///
    /// * `msg` - The message content
    ///
    /// # Returns
    ///
    /// A new `MessageData::Warning` variant
    #[must_use]
    pub fn warning(msg: String) -> Self {
        MessageData::Warning(msg)
    }

    /// Create a new error message
    ///
    /// # Arguments
    ///
    /// * `msg` - The message content
    ///
    /// # Returns
    ///
    /// A new `MessageData::Error` variant
    #[must_use]
    pub fn error(msg: String) -> Self {
        MessageData::Error(msg)
    }

    /// Create a new debug message
    ///
    /// # Arguments
    ///
    /// * `msg` - The message content
    ///
    /// # Returns
    ///
    /// A new `MessageData::Debug` variant
    #[must_use]
    pub fn debug(msg: String) -> Self {
        MessageData::Debug(msg)
    }
}

impl fmt::Display for MessageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageData::Info(msg) => write!(f, "MESSAGE Info: {msg}"),
            MessageData::Warning(msg) => write!(f, "MESSAGE Warning: {msg}"),
            MessageData::Error(msg) => write!(f, "MESSAGE Error: {msg}"),
            MessageData::Debug(msg) => write!(f, "MESSAGE Debug: {msg}"),
            MessageData::Raw(msg) => write!(f, "{msg}"),
        }
    }
}
