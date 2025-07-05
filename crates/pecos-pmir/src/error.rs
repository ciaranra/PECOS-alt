/*!
Error handling for PECOS PMIR

This module provides comprehensive error handling for all PMIR operations including:
- Parse errors from various input formats
- Type checking and validation errors
- Runtime execution errors
- Compilation and optimization errors
- QEC-specific errors

Follows Rust error handling best practices with detailed error information
and user-friendly error messages.
*/

use std::fmt;

/// Main error type for PMIR operations
#[derive(Debug, Clone)]
pub enum PMIRError {
    /// Parsing errors from input formats
    Parse(ParseError),
    /// Type system errors
    Type(TypeError),
    /// Validation errors (semantic analysis)
    Validation(ValidationError),
    /// Runtime execution errors
    Runtime(RuntimeError),
    /// Compilation/optimization errors
    Compilation(CompilationError),
    /// I/O errors
    IO(String),
    /// Internal errors (bugs)
    Internal(String),
}

/// Parsing errors from various input formats
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Syntax error in input
    Syntax {
        message: String,
        location: SourceLocation,
        expected: Option<String>,
        found: Option<String>,
    },
    /// Unsupported feature in input format
    Unsupported {
        feature: String,
        format: String,
        location: SourceLocation,
    },
    /// Invalid structure (e.g., malformed HUGR)
    InvalidStructure {
        message: String,
        location: SourceLocation,
    },
    /// JSON/serialization errors
    Serialization { message: String, format: String },
    /// File I/O errors during parsing
    FileIO { path: String, message: String },
}

/// Type system errors
#[derive(Debug, Clone)]
pub enum TypeError {
    /// Type mismatch
    Mismatch {
        expected: crate::types::Type,
        found: crate::types::Type,
        location: SourceLocation,
    },
    /// Undefined type
    Undefined {
        type_name: String,
        location: SourceLocation,
    },
    /// Incompatible types in operation
    Incompatible {
        op_name: String,
        types: Vec<crate::types::Type>,
        location: SourceLocation,
    },
    /// Type inference failure
    InferenceFailed {
        message: String,
        location: SourceLocation,
    },
    /// Quantum no-cloning violation
    NoCloning {
        variable: String,
        location: SourceLocation,
    },
    /// Invalid type parameters
    InvalidParameters {
        type_name: String,
        message: String,
        location: SourceLocation,
    },
}

/// Semantic validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Undefined variable or function
    Undefined {
        name: String,
        kind: DefinitionKind,
        location: SourceLocation,
    },
    /// Variable used before definition
    UseBeforeDefine {
        variable: String,
        use_location: SourceLocation,
        define_location: Option<SourceLocation>,
    },
    /// Multiple definitions of same name
    Redefinition {
        name: String,
        kind: DefinitionKind,
        first_location: SourceLocation,
        second_location: SourceLocation,
    },
    /// Invalid control flow
    ControlFlow {
        message: String,
        location: SourceLocation,
    },
    /// Quantum circuit violations
    QuantumViolation {
        rule: String,
        message: String,
        location: SourceLocation,
    },
    /// Function signature mismatch
    SignatureMismatch {
        function: String,
        expected_args: usize,
        found_args: usize,
        location: SourceLocation,
    },
    /// Unknown dialect
    UnknownDialect(String),
    /// Unknown operation in dialect
    UnknownOperation(String),
}

/// Runtime execution errors
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Division by zero
    DivisionByZero { location: SourceLocation },
    /// Array index out of bounds
    IndexOutOfBounds {
        index: i64,
        size: usize,
        location: SourceLocation,
    },
    /// Null pointer dereference
    NullDereference { location: SourceLocation },
    /// Quantum measurement error
    MeasurementError {
        message: String,
        location: SourceLocation,
    },
    /// Insufficient quantum resources
    InsufficientResources {
        requested: usize,
        available: usize,
        resource_type: String,
        location: SourceLocation,
    },
    /// Stack overflow
    StackOverflow { location: SourceLocation },
    /// External function call failed
    ExternalCall {
        function: String,
        message: String,
        location: SourceLocation,
    },
}

/// Compilation and optimization errors
#[derive(Debug, Clone)]
pub enum CompilationError {
    /// Optimization pass failed
    OptimizationFailed {
        pass_name: String,
        message: String,
        location: Option<SourceLocation>,
    },
    /// Code generation failed
    CodegenFailed {
        target: String,
        message: String,
        location: Option<SourceLocation>,
    },
    /// Unsupported target
    UnsupportedTarget { target: String, feature: String },
    /// Resource estimation failed
    ResourceEstimation { message: String },
    /// Circuit routing failed
    RoutingFailed { topology: String, message: String },
}

/// Kind of definition for validation errors
#[derive(Debug, Clone)]
pub enum DefinitionKind {
    Variable,
    Function,
    Type,
    Module,
    Block,
}

/// Source location for error reporting
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceLocation {
    /// Source file path
    pub file: String,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)  
    pub column: usize,
    /// Character span in source
    pub span: Span,
}

/// Character span in source text
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Span {
    /// Start position (0-based)
    pub start: usize,
    /// End position (0-based, exclusive)
    pub end: usize,
}

/// Result type alias for PMIR operations
pub type Result<T> = std::result::Result<T, PMIRError>;

impl PMIRError {
    /// Create a parse error
    pub fn parse_error(message: impl Into<String>, location: SourceLocation) -> Self {
        PMIRError::Parse(ParseError::Syntax {
            message: message.into(),
            location,
            expected: None,
            found: None,
        })
    }

    /// Create a type error
    #[must_use]
    pub fn type_error(
        expected: crate::types::Type,
        found: crate::types::Type,
        location: SourceLocation,
    ) -> Self {
        PMIRError::Type(TypeError::Mismatch {
            expected,
            found,
            location,
        })
    }

    /// Create a validation error
    pub fn undefined_variable(name: impl Into<String>, location: SourceLocation) -> Self {
        PMIRError::Validation(ValidationError::Undefined {
            name: name.into(),
            kind: DefinitionKind::Variable,
            location,
        })
    }

    /// Create a runtime error
    pub fn runtime_error(message: impl Into<String>, location: SourceLocation) -> Self {
        PMIRError::Runtime(RuntimeError::ExternalCall {
            function: "unknown".to_string(),
            message: message.into(),
            location,
        })
    }

    /// Create an internal error (for bugs)
    pub fn internal(message: impl Into<String>) -> Self {
        PMIRError::Internal(message.into())
    }

    /// Get the source location associated with this error (if any)
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            PMIRError::Parse(e) => e.location(),
            PMIRError::Type(e) => e.location(),
            PMIRError::Validation(e) => e.location(),
            PMIRError::Runtime(e) => e.location(),
            PMIRError::Compilation(e) => e.location(),
            PMIRError::IO(_) | PMIRError::Internal(_) => None,
        }
    }

    /// Check if this is a recoverable error
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        match self {
            PMIRError::Parse(_) | PMIRError::Type(_) | PMIRError::Validation(_) => false,
            PMIRError::Runtime(_) => true,
            PMIRError::Compilation(_) => true,
            PMIRError::IO(_) => true,
            PMIRError::Internal(_) => false,
        }
    }
}

impl ParseError {
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            ParseError::Syntax { location, .. }
            | ParseError::Unsupported { location, .. }
            | ParseError::InvalidStructure { location, .. } => Some(location),
            ParseError::Serialization { .. } | ParseError::FileIO { .. } => None,
        }
    }
}

impl TypeError {
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            TypeError::Mismatch { location, .. }
            | TypeError::Undefined { location, .. }
            | TypeError::Incompatible { location, .. }
            | TypeError::InferenceFailed { location, .. }
            | TypeError::NoCloning { location, .. }
            | TypeError::InvalidParameters { location, .. } => Some(location),
        }
    }
}

impl ValidationError {
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            ValidationError::Undefined { location, .. }
            | ValidationError::UseBeforeDefine {
                use_location: location,
                ..
            }
            | ValidationError::Redefinition {
                second_location: location,
                ..
            }
            | ValidationError::ControlFlow { location, .. }
            | ValidationError::QuantumViolation { location, .. }
            | ValidationError::SignatureMismatch { location, .. } => Some(location),
            ValidationError::UnknownDialect(_) | ValidationError::UnknownOperation(_) => None,
        }
    }
}

impl RuntimeError {
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            RuntimeError::DivisionByZero { location }
            | RuntimeError::IndexOutOfBounds { location, .. }
            | RuntimeError::NullDereference { location }
            | RuntimeError::MeasurementError { location, .. }
            | RuntimeError::InsufficientResources { location, .. }
            | RuntimeError::StackOverflow { location }
            | RuntimeError::ExternalCall { location, .. } => Some(location),
        }
    }
}

impl CompilationError {
    #[must_use]
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            CompilationError::OptimizationFailed { location, .. }
            | CompilationError::CodegenFailed { location, .. } => location.as_ref(),
            CompilationError::UnsupportedTarget { .. }
            | CompilationError::ResourceEstimation { .. }
            | CompilationError::RoutingFailed { .. } => None,
        }
    }
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: impl Into<String>, line: usize, column: usize, span: Span) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            span,
        }
    }

    /// Create an unknown/dummy location
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            file: "<unknown>".to_string(),
            line: 1,
            column: 1,
            span: Span { start: 0, end: 0 },
        }
    }
}

impl Span {
    /// Create a new span
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of this span
    #[must_use]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if span is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

// Display implementations for user-friendly error messages

impl fmt::Display for PMIRError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PMIRError::Parse(e) => write!(f, "Parse error: {e}"),
            PMIRError::Type(e) => write!(f, "Type error: {e}"),
            PMIRError::Validation(e) => write!(f, "Validation error: {e}"),
            PMIRError::Runtime(e) => write!(f, "Runtime error: {e}"),
            PMIRError::Compilation(e) => write!(f, "Compilation error: {e}"),
            PMIRError::IO(msg) => write!(f, "I/O error: {msg}"),
            PMIRError::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Syntax {
                message,
                expected,
                found,
                ..
            } => {
                write!(f, "{message}")?;
                if let (Some(exp), Some(fnd)) = (expected, found) {
                    write!(f, " (expected {exp}, found {fnd})")?;
                }
                Ok(())
            }
            ParseError::Unsupported {
                feature, format, ..
            } => {
                write!(f, "Unsupported feature '{feature}' in format '{format}'")
            }
            ParseError::InvalidStructure { message, .. } => {
                write!(f, "Invalid structure: {message}")
            }
            ParseError::Serialization { message, format } => {
                write!(f, "Serialization error in {format}: {message}")
            }
            ParseError::FileIO { path, message } => {
                write!(f, "File I/O error for '{path}': {message}")
            }
        }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Mismatch {
                expected, found, ..
            } => {
                write!(f, "Type mismatch: expected {expected}, found {found}")
            }
            TypeError::Undefined { type_name, .. } => {
                write!(f, "Undefined type '{type_name}'")
            }
            TypeError::Incompatible { op_name, types, .. } => {
                write!(f, "Incompatible types for operation '{op_name}': {types:?}")
            }
            TypeError::InferenceFailed { message, .. } => {
                write!(f, "Type inference failed: {message}")
            }
            TypeError::NoCloning { variable, .. } => {
                write!(
                    f,
                    "Quantum no-cloning violation: variable '{variable}' used multiple times"
                )
            }
            TypeError::InvalidParameters {
                type_name, message, ..
            } => {
                write!(f, "Invalid type parameters for '{type_name}': {message}")
            }
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Undefined { name, kind, .. } => {
                write!(f, "Undefined {kind}: '{name}'")
            }
            ValidationError::UseBeforeDefine { variable, .. } => {
                write!(f, "Variable '{variable}' used before definition")
            }
            ValidationError::Redefinition { name, kind, .. } => {
                write!(f, "Redefinition of {kind} '{name}'")
            }
            ValidationError::ControlFlow { message, .. } => {
                write!(f, "Control flow error: {message}")
            }
            ValidationError::QuantumViolation { rule, message, .. } => {
                write!(f, "Quantum rule '{rule}' violated: {message}")
            }
            ValidationError::SignatureMismatch {
                function,
                expected_args,
                found_args,
                ..
            } => {
                write!(
                    f,
                    "Function '{function}' expects {expected_args} arguments, found {found_args}"
                )
            }
            ValidationError::UnknownDialect(dialect) => {
                write!(f, "Unknown dialect: '{dialect}'")
            }
            ValidationError::UnknownOperation(op) => {
                write!(f, "Unknown operation: '{op}'")
            }
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::DivisionByZero { .. } => {
                write!(f, "Division by zero")
            }
            RuntimeError::IndexOutOfBounds { index, size, .. } => {
                write!(f, "Index {index} out of bounds for array of size {size}")
            }
            RuntimeError::NullDereference { .. } => {
                write!(f, "Null pointer dereference")
            }
            RuntimeError::MeasurementError { message, .. } => {
                write!(f, "Measurement error: {message}")
            }
            RuntimeError::InsufficientResources {
                requested,
                available,
                resource_type,
                ..
            } => {
                write!(
                    f,
                    "Insufficient {resource_type}: requested {requested}, available {available}"
                )
            }
            RuntimeError::StackOverflow { .. } => {
                write!(f, "Stack overflow")
            }
            RuntimeError::ExternalCall {
                function, message, ..
            } => {
                write!(f, "External function '{function}' failed: {message}")
            }
        }
    }
}

impl fmt::Display for CompilationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilationError::OptimizationFailed {
                pass_name, message, ..
            } => {
                write!(f, "Optimization pass '{pass_name}' failed: {message}")
            }
            CompilationError::CodegenFailed {
                target, message, ..
            } => {
                write!(f, "Code generation for '{target}' failed: {message}")
            }
            CompilationError::UnsupportedTarget { target, feature } => {
                write!(f, "Target '{target}' does not support feature '{feature}'")
            }
            CompilationError::ResourceEstimation { message } => {
                write!(f, "Resource estimation failed: {message}")
            }
            CompilationError::RoutingFailed { topology, message } => {
                write!(
                    f,
                    "Circuit routing for '{topology}' topology failed: {message}"
                )
            }
        }
    }
}

impl fmt::Display for DefinitionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefinitionKind::Variable => write!(f, "variable"),
            DefinitionKind::Function => write!(f, "function"),
            DefinitionKind::Type => write!(f, "type"),
            DefinitionKind::Module => write!(f, "module"),
            DefinitionKind::Block => write!(f, "block"),
        }
    }
}

impl std::error::Error for PMIRError {}
impl std::error::Error for ParseError {}
impl std::error::Error for TypeError {}
impl std::error::Error for ValidationError {}
impl std::error::Error for RuntimeError {}
impl std::error::Error for CompilationError {}

// Conversion from external error types

impl From<std::io::Error> for PMIRError {
    fn from(err: std::io::Error) -> Self {
        PMIRError::IO(err.to_string())
    }
}

impl From<serde_json::Error> for PMIRError {
    fn from(err: serde_json::Error) -> Self {
        PMIRError::Parse(ParseError::Serialization {
            message: err.to_string(),
            format: "JSON".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_error_creation() {
        let loc = SourceLocation::unknown();

        let type_err = PMIRError::type_error(qubit_type(), int_type(), loc.clone());
        assert!(matches!(
            type_err,
            PMIRError::Type(TypeError::Mismatch { .. })
        ));

        let var_err = PMIRError::undefined_variable("x", loc.clone());
        assert!(matches!(
            var_err,
            PMIRError::Validation(ValidationError::Undefined { .. })
        ));
    }

    #[test]
    fn test_error_display() {
        let loc = SourceLocation::unknown();
        let err = PMIRError::type_error(qubit_type(), int_type(), loc);
        let msg = err.to_string();
        assert!(msg.contains("Type mismatch"));
        assert!(msg.contains("quantum.qubit"));
        assert!(msg.contains("int"));
    }

    #[test]
    fn test_span_operations() {
        let span = Span::new(10, 20);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());

        let empty_span = Span::new(5, 5);
        assert_eq!(empty_span.len(), 0);
        assert!(empty_span.is_empty());
    }
}
