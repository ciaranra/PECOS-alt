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
    Prep = 10,
    RZZ = 11,
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
            10 => GateTypeId::Prep,
            11 => GateTypeId::RZZ,
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
            GateTypeId::Prep => write!(f, "Prep"),
            GateTypeId::RZZ => write!(f, "RZZ"),
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

    /// Create a new RZZ gate
    #[must_use]
    pub fn rzz(theta: f64, qubit1: usize, qubit2: usize) -> Self {
        Self::new(GateTypeId::RZZ, vec![qubit1, qubit2], vec![theta], None)
    }

    /// Create a new RZ gate
    #[must_use]
    pub fn rz(theta: f64, qubit: usize) -> Self {
        Self::new(GateTypeId::RZ, vec![qubit], vec![theta], None)
    }

    /// Create a new R1XY gate
    #[must_use]
    pub fn r1xy(theta: f64, phi: f64, qubit: usize) -> Self {
        Self::new(GateTypeId::R1XY, vec![qubit], vec![theta, phi], None)
    }

    /// Create a new Measure gate
    #[must_use]
    pub fn measure(qubit: usize, result_id: usize) -> Self {
        Self::new(GateTypeId::Measure, vec![qubit], vec![], Some(result_id))
    }

    #[must_use]
    pub fn prep(qubit: usize) -> Self {
        Self::new(GateTypeId::Prep, vec![qubit], vec![], None)
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
}
