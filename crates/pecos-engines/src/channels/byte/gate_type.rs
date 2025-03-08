use pecos_core::types::{GateType as CoreGateType, QuantumCommand};
use std::fmt;

/// FFI-friendly representation of quantum gate types
///
/// This enum is designed to be FFI-friendly with a C-compatible memory layout.
/// It represents the same gate types as the core `GateType` enum but with a more
/// predictable memory layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateTypeId {
    X = 1,
    Y = 2,
    Z = 3,
    H = 4,
    CX = 5,
    SZZ = 6,
    RZ = 7,
    R1XY = 8,
    Measure = 9,
}

impl From<&CoreGateType> for GateTypeId {
    fn from(gate: &CoreGateType) -> Self {
        match gate {
            CoreGateType::X => GateTypeId::X,
            CoreGateType::Y => GateTypeId::Y,
            CoreGateType::Z => GateTypeId::Z,
            CoreGateType::H => GateTypeId::H,
            CoreGateType::CX => GateTypeId::CX,
            CoreGateType::SZZ => GateTypeId::SZZ,
            CoreGateType::RZ { .. } => GateTypeId::RZ,
            CoreGateType::R1XY { .. } => GateTypeId::R1XY,
            CoreGateType::Measure { .. } => GateTypeId::Measure,
        }
    }
}

impl From<u8> for GateTypeId {
    fn from(value: u8) -> Self {
        match value {
            1 => GateTypeId::X,
            2 => GateTypeId::Y,
            3 => GateTypeId::Z,
            4 => GateTypeId::H,
            5 => GateTypeId::CX,
            6 => GateTypeId::SZZ,
            7 => GateTypeId::RZ,
            8 => GateTypeId::R1XY,
            9 => GateTypeId::Measure,
            _ => panic!("Invalid gate type ID: {value}"),
        }
    }
}

impl From<GateTypeId> for u8 {
    fn from(gate_type: GateTypeId) -> Self {
        gate_type as u8
    }
}

impl fmt::Display for GateTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateTypeId::X => write!(f, "X"),
            GateTypeId::Y => write!(f, "Y"),
            GateTypeId::Z => write!(f, "Z"),
            GateTypeId::H => write!(f, "H"),
            GateTypeId::CX => write!(f, "CX"),
            GateTypeId::SZZ => write!(f, "SZZ"),
            GateTypeId::RZ => write!(f, "RZ"),
            GateTypeId::R1XY => write!(f, "R1XY"),
            GateTypeId::Measure => write!(f, "Measure"),
        }
    }
}

/// Represents a quantum gate with its type, parameters, and target qubits
///
/// This struct is designed to replace `QuantumCommand` with a more FFI-friendly
/// representation. It contains all the information needed to represent a quantum
/// gate operation.
#[derive(Debug, Clone)]
pub struct QuantumGate {
    /// The type of the gate
    pub gate_type: GateTypeId,
    /// The qubits the gate acts on
    pub qubits: Vec<usize>,
    /// Optional parameters for parameterized gates
    pub params: Vec<f64>,
    /// Optional result ID for measurement gates
    pub result_id: Option<usize>,
}

impl QuantumGate {
    /// Create a new quantum gate
    #[must_use]
    pub fn new(
        gate_type: GateTypeId,
        qubits: Vec<usize>,
        params: Vec<f64>,
        result_id: Option<usize>,
    ) -> Self {
        Self {
            gate_type,
            qubits,
            params,
            result_id,
        }
    }

    /// Create a new X gate
    #[must_use]
    pub fn x(qubit: usize) -> Self {
        Self::new(GateTypeId::X, vec![qubit], vec![], None)
    }

    /// Create a new Y gate
    #[must_use]
    pub fn y(qubit: usize) -> Self {
        Self::new(GateTypeId::Y, vec![qubit], vec![], None)
    }

    /// Create a new Z gate
    #[must_use]
    pub fn z(qubit: usize) -> Self {
        Self::new(GateTypeId::Z, vec![qubit], vec![], None)
    }

    /// Create a new H gate
    #[must_use]
    pub fn h(qubit: usize) -> Self {
        Self::new(GateTypeId::H, vec![qubit], vec![], None)
    }

    /// Create a new CX gate
    #[must_use]
    pub fn cx(control: usize, target: usize) -> Self {
        Self::new(GateTypeId::CX, vec![control, target], vec![], None)
    }

    /// Create a new SZZ gate
    #[must_use]
    pub fn szz(qubit1: usize, qubit2: usize) -> Self {
        Self::new(GateTypeId::SZZ, vec![qubit1, qubit2], vec![], None)
    }

    /// Create a new RZ gate
    #[must_use]
    pub fn rz(theta: f64, qubit: usize) -> Self {
        Self::new(GateTypeId::RZ, vec![qubit], vec![theta], None)
    }

    /// Create a new R1XY gate
    #[must_use]
    pub fn r1xy(phi: f64, theta: f64, qubit: usize) -> Self {
        Self::new(GateTypeId::R1XY, vec![qubit], vec![phi, theta], None)
    }

    /// Create a new Measure gate
    #[must_use]
    pub fn measure(qubit: usize, result_id: usize) -> Self {
        Self::new(GateTypeId::Measure, vec![qubit], vec![], Some(result_id))
    }

    /// Convert from a core `GateType` and qubits
    #[must_use]
    pub fn from_core_gate(gate: &CoreGateType, qubits: &[usize]) -> Self {
        match gate {
            CoreGateType::X => Self::x(qubits[0]),
            CoreGateType::Y => Self::y(qubits[0]),
            CoreGateType::Z => Self::z(qubits[0]),
            CoreGateType::H => Self::h(qubits[0]),
            CoreGateType::CX => Self::cx(qubits[0], qubits[1]),
            CoreGateType::SZZ => Self::szz(qubits[0], qubits[1]),
            CoreGateType::RZ { theta } => Self::rz(*theta, qubits[0]),
            CoreGateType::R1XY { phi, theta } => Self::r1xy(*phi, *theta, qubits[0]),
            CoreGateType::Measure { result_id } => Self::measure(qubits[0], *result_id),
        }
    }

    /// Convert to a core `GateType`
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - Called on a Measure gate without a `result_id`
    /// - Called on a parameterized gate (RZ, R1XY) without the required parameters
    #[must_use]
    pub fn to_core_gate(&self) -> CoreGateType {
        match self.gate_type {
            GateTypeId::X => CoreGateType::X,
            GateTypeId::Y => CoreGateType::Y,
            GateTypeId::Z => CoreGateType::Z,
            GateTypeId::H => CoreGateType::H,
            GateTypeId::CX => CoreGateType::CX,
            GateTypeId::SZZ => CoreGateType::SZZ,
            GateTypeId::RZ => CoreGateType::RZ {
                theta: self.params[0],
            },
            GateTypeId::R1XY => CoreGateType::R1XY {
                phi: self.params[0],
                theta: self.params[1],
            },
            GateTypeId::Measure => CoreGateType::Measure {
                result_id: self.result_id.unwrap(),
            },
        }
    }

    /// Parse a quantum gate from a command string
    #[allow(clippy::too_many_lines)]
    pub fn parse_from_str(cmd_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        match parts.first() {
            Some(&"RZ") => {
                if parts.len() != 3 {
                    return Err("Invalid RZ format".into());
                }
                let theta = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid theta: {e}"))?;
                let qubit = parts[2]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                Ok(Self::rz(theta, qubit))
            }
            Some(&"R1XY") => {
                if parts.len() != 4 {
                    return Err("Invalid R1XY format".into());
                }
                let phi = parts[1].parse().map_err(|e| format!("Invalid phi: {e}"))?;
                let theta = parts[2]
                    .parse()
                    .map_err(|e| format!("Invalid theta: {e}"))?;
                let qubit = parts[3]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                Ok(Self::r1xy(phi, theta, qubit))
            }
            Some(&"SZZ") => {
                if parts.len() != 3 {
                    return Err("Invalid SZZ format".into());
                }
                let qubit1 = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid qubit1: {e}"))?;
                let qubit2 = parts[2]
                    .parse()
                    .map_err(|e| format!("Invalid qubit2: {e}"))?;
                Ok(Self::szz(qubit1, qubit2))
            }
            Some(&"X") => {
                if parts.len() != 2 {
                    return Err("Invalid X format".into());
                }
                let qubit = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                Ok(Self::x(qubit))
            }
            Some(&"Y") => {
                if parts.len() != 2 {
                    return Err("Invalid Y format".into());
                }
                let qubit = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                Ok(Self::y(qubit))
            }
            Some(&"Z") => {
                if parts.len() != 2 {
                    return Err("Invalid Z format".into());
                }
                let qubit = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                Ok(Self::z(qubit))
            }
            Some(&"H") => {
                if parts.len() != 2 {
                    return Err("Invalid H format".into());
                }
                let qubit = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                Ok(Self::h(qubit))
            }
            Some(&"CX") => {
                if parts.len() != 3 {
                    return Err("Invalid CX format".into());
                }
                let control = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid control qubit: {e}"))?;
                let target = parts[2]
                    .parse()
                    .map_err(|e| format!("Invalid target qubit: {e}"))?;
                Ok(Self::cx(control, target))
            }
            Some(&"M") => {
                if parts.len() != 3 {
                    return Err("Invalid M format".into());
                }
                let qubit = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid qubit: {e}"))?;
                let result_id = parts[2]
                    .parse()
                    .map_err(|e| format!("Invalid result_id: {e}"))?;
                Ok(Self::measure(qubit, result_id))
            }
            _ => Err(format!(
                "Unknown command type: {}",
                parts.first().unwrap_or(&"<empty>")
            )),
        }
    }

    /// Convert from a `QuantumCommand`
    #[must_use]
    pub fn from_quantum_command(cmd: &QuantumCommand) -> Self {
        Self::from_core_gate(&cmd.gate, &cmd.qubits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_type_id_conversion() {
        assert_eq!(GateTypeId::X as u8, 1);
        assert_eq!(GateTypeId::Y as u8, 2);
        assert_eq!(GateTypeId::Z as u8, 3);
        assert_eq!(GateTypeId::H as u8, 4);
        assert_eq!(GateTypeId::CX as u8, 5);
        assert_eq!(GateTypeId::SZZ as u8, 6);
        assert_eq!(GateTypeId::RZ as u8, 7);
        assert_eq!(GateTypeId::R1XY as u8, 8);
        assert_eq!(GateTypeId::Measure as u8, 9);

        assert_eq!(GateTypeId::from(1u8), GateTypeId::X);
        assert_eq!(GateTypeId::from(2u8), GateTypeId::Y);
        assert_eq!(GateTypeId::from(3u8), GateTypeId::Z);
        assert_eq!(GateTypeId::from(4u8), GateTypeId::H);
        assert_eq!(GateTypeId::from(5u8), GateTypeId::CX);
        assert_eq!(GateTypeId::from(6u8), GateTypeId::SZZ);
        assert_eq!(GateTypeId::from(7u8), GateTypeId::RZ);
        assert_eq!(GateTypeId::from(8u8), GateTypeId::R1XY);
        assert_eq!(GateTypeId::from(9u8), GateTypeId::Measure);
    }

    #[test]
    fn test_quantum_gate_creation() {
        let x_gate = QuantumGate::x(0);
        assert_eq!(x_gate.gate_type, GateTypeId::X);
        assert_eq!(x_gate.qubits, vec![0]);
        assert!(x_gate.params.is_empty());
        assert_eq!(x_gate.result_id, None);

        let rz_gate = QuantumGate::rz(0.5, 1);
        assert_eq!(rz_gate.gate_type, GateTypeId::RZ);
        assert_eq!(rz_gate.qubits, vec![1]);
        assert_eq!(rz_gate.params, vec![0.5]);
        assert_eq!(rz_gate.result_id, None);

        let measure_gate = QuantumGate::measure(2, 42);
        assert_eq!(measure_gate.gate_type, GateTypeId::Measure);
        assert_eq!(measure_gate.qubits, vec![2]);
        assert!(measure_gate.params.is_empty());
        assert_eq!(measure_gate.result_id, Some(42));
    }

    #[test]
    fn test_parse_from_str() {
        let x_gate = QuantumGate::parse_from_str("X 0").unwrap();
        assert_eq!(x_gate.gate_type, GateTypeId::X);
        assert_eq!(x_gate.qubits, vec![0]);

        let rz_gate = QuantumGate::parse_from_str("RZ 0.5 1").unwrap();
        assert_eq!(rz_gate.gate_type, GateTypeId::RZ);
        assert_eq!(rz_gate.qubits, vec![1]);
        assert_eq!(rz_gate.params, vec![0.5]);

        let r1xy_gate = QuantumGate::parse_from_str("R1XY 0.1 0.2 2").unwrap();
        assert_eq!(r1xy_gate.gate_type, GateTypeId::R1XY);
        assert_eq!(r1xy_gate.qubits, vec![2]);
        assert_eq!(r1xy_gate.params, vec![0.1, 0.2]);

        let measure_gate = QuantumGate::parse_from_str("M 3 42").unwrap();
        assert_eq!(measure_gate.gate_type, GateTypeId::Measure);
        assert_eq!(measure_gate.qubits, vec![3]);
        assert_eq!(measure_gate.result_id, Some(42));
    }

    #[test]
    fn test_core_gate_conversion() {
        let core_x = CoreGateType::X;
        let gate_id = GateTypeId::from(&core_x);
        assert_eq!(gate_id, GateTypeId::X);

        let quantum_gate = QuantumGate::x(0);
        let core_gate = quantum_gate.to_core_gate();
        match core_gate {
            CoreGateType::X => {}
            _ => panic!("Expected X gate"),
        }

        let core_rz = CoreGateType::RZ { theta: 0.5 };
        let gate_id = GateTypeId::from(&core_rz);
        assert_eq!(gate_id, GateTypeId::RZ);

        let quantum_gate = QuantumGate::from_core_gate(&core_rz, &[1]);
        assert_eq!(quantum_gate.gate_type, GateTypeId::RZ);
        assert_eq!(quantum_gate.qubits, vec![1]);
        assert_eq!(quantum_gate.params, vec![0.5]);
    }
}
