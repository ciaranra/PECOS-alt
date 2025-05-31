use crate::core::record_data::RecordData;
use pecos_core::QubitId;
use std::fmt;

/// Command type for unknown commands
///
/// This enum represents the various types of unknown commands that can be
/// encountered during program execution. It helps categorize unknown
/// commands for better error reporting and debugging.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    /// Unknown gate command
    Gate(String),

    /// Unknown control command
    Control(String),

    /// Other unknown command
    Other(String),
}

impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandType::Gate(cmd) => write!(f, "Unknown Gate: {cmd}"),
            CommandType::Control(cmd) => write!(f, "Unknown Control: {cmd}"),
            CommandType::Other(cmd) => write!(f, "Unknown Command: {cmd}"),
        }
    }
}

/// Structured command type for binary representation of quantum operations
///
/// This enum represents the various quantum operations that can be executed
/// by the quantum system, including gate operations, measurements, and
/// non-gate operations like record and message commands.
///
/// # Gate Operations
///
/// Gate operations represent quantum gates that are applied to qubits:
///
/// - `H` - Hadamard gate
/// - `X`, `Y`, `Z` - Pauli gates
/// - `CX` - Controlled-NOT gate
/// - `RZ` - Rotation around Z-axis
/// - `R1XY` - Rotation in XY plane
/// - `SZZ` - SZZ gate
/// - `RZZ` - Rotation around ZZ
///
/// # Measurement Operations
///
/// - `Measure` - Measure a qubit and store the result
/// - `Prep` - Prepare a qubit in the |0⟩ state
///
/// # Non-Gate Operations
///
/// - `Record` - Record data for classical processing
/// - `Unknown` - Unknown command with command type
#[derive(Debug, Clone, PartialEq)]
pub enum QuantumCmd {
    /// Hadamard gate on qubit
    H(QubitId),

    /// X gate (NOT gate) on qubit
    X(QubitId),

    /// Y gate on qubit
    Y(QubitId),

    /// Z gate on qubit
    Z(QubitId),

    /// CNOT gate with control and target qubits
    CX(QubitId, QubitId),

    /// RZ gate with angle (in radians) and qubit
    RZ(f64, QubitId),

    /// R1XY gate with theta, phi angles (in radians) and qubit
    R1XY(f64, f64, QubitId),

    /// U gate with theta, phi, lambda angles (in radians) and qubit
    U(f64, f64, f64, QubitId),

    /// SZZ gate with two qubits
    SZZ(QubitId, QubitId),

    /// RZZ gate with angle (in radians) and two qubits
    RZZ(f64, QubitId, QubitId),

    /// Measure qubit
    Measure(QubitId),

    /// Prepare qubit in the |0⟩ state
    Prep(QubitId),

    /// Record command with structured data
    Record(RecordData),

    /// Unknown command with command type
    Unknown(CommandType),
}

impl QuantumCmd {
    /// Create a Record command from a string for backward compatibility
    #[must_use]
    pub fn record_from_string(cmd: String) -> Self {
        // Try to parse the string as structured data
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == "RECORD" {
            if let Ok(result_id) = parts[1].parse::<usize>() {
                // This is a result record
                let label = if parts.len() >= 3 {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                return QuantumCmd::Record(RecordData::result(result_id, label));
            } else if parts.len() >= 3 {
                // Try to parse as a key-value record
                if let Ok(value) = parts[2].parse::<f64>() {
                    return QuantumCmd::Record(RecordData::key_value(parts[1].to_string(), value));
                }
            }
        }
        // Fall back to raw record
        QuantumCmd::Record(RecordData::RawRecord(cmd))
    }
    /// Parse commands directly from binary data
    /// This is a more efficient alternative to string-based parsing
    #[must_use]
    pub fn parse_binary_commands<T>(commands: &[T], parse_fn: impl Fn(&T) -> Self) -> Vec<Self> {
        commands.iter().map(parse_fn).collect()
    }

    /// Check if this command is a measurement
    #[must_use]
    pub fn is_measurement(&self) -> bool {
        matches!(self, QuantumCmd::Measure(_))
    }

    /// Get the `result_id` if this is a measurement command or a result record
    #[must_use]
    pub fn result_id(&self) -> Option<usize> {
        if let QuantumCmd::Record(RecordData::ResultRecord(result_id, _)) = self {
            Some(*result_id)
        } else {
            None
        }
    }
}

impl fmt::Display for QuantumCmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantumCmd::H(qubit) => write!(f, "H {qubit}"),
            QuantumCmd::X(qubit) => write!(f, "X {qubit}"),
            QuantumCmd::Y(qubit) => write!(f, "Y {qubit}"),
            QuantumCmd::Z(qubit) => write!(f, "Z {qubit}"),
            QuantumCmd::CX(control, target) => write!(f, "CX {control} {target}"),
            QuantumCmd::RZ(angle, qubit) => write!(f, "RZ {angle} {qubit}"),
            QuantumCmd::R1XY(theta, phi, qubit) => write!(f, "R1XY {theta} {phi} {qubit}"),
            QuantumCmd::U(theta, phi, lambda, qubit) => {
                write!(f, "U {theta} {phi} {lambda} {qubit}")
            }
            QuantumCmd::SZZ(qubit1, qubit2) => write!(f, "SZZ {qubit1} {qubit2}"),
            QuantumCmd::RZZ(angle, qubit1, qubit2) => {
                write!(f, "RZZ {angle} {qubit1} {qubit2}")
            }
            QuantumCmd::Measure(qubit) => write!(f, "M {qubit}"),
            QuantumCmd::Prep(qubit) => write!(f, "PREP {qubit}"),
            QuantumCmd::Record(data) => match data {
                RecordData::ResultRecord(result_id, Some(label)) => {
                    write!(f, "RECORD {result_id} {label}")
                }
                RecordData::ResultRecord(result_id, None) => write!(f, "RECORD {result_id}"),
                RecordData::KeyValueRecord(key, value) => write!(f, "RECORD {key} {value}"),
                RecordData::RawRecord(cmd) => write!(f, "{cmd}"),
            },
            QuantumCmd::Unknown(cmd) => write!(f, "{cmd}"),
        }
    }
}
