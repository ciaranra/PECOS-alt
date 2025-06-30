/// Structured logging helpers for PECOS components
use log::{debug, trace, warn};

/// Component-specific logger
pub struct ComponentLogger {
    component: &'static str,
}

impl ComponentLogger {
    /// Create a new logger for a component
    #[must_use]
    pub const fn new(component: &'static str) -> Self {
        Self { component }
    }

    /// Log a debug message with component prefix
    pub fn debug(&self, message: impl AsRef<str>) {
        debug!("{}: {}", self.component, message.as_ref());
    }

    /// Log a debug message with formatted arguments
    pub fn debugf<F>(&self, f: F)
    where
        F: FnOnce() -> String,
    {
        debug!("{}: {}", self.component, f());
    }

    /// Log a trace message with component prefix
    pub fn trace(&self, message: impl AsRef<str>) {
        trace!("{}: {}", self.component, message.as_ref());
    }

    /// Log a warning with component prefix
    pub fn warn(&self, message: impl AsRef<str>) {
        warn!("{}: {}", self.component, message.as_ref());
    }
}

/// Create component loggers for common components
pub const LLVM_LOG: ComponentLogger = ComponentLogger::new("LLVM");
pub const HUGR_LOG: ComponentLogger = ComponentLogger::new("HUGR");
pub const RUNTIME_LOG: ComponentLogger = ComponentLogger::new("Runtime");
