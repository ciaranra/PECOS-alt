/*!
PAST (PECOS AST) - Abstract Syntax Tree for HUGR

This module defines the AST structures that represent parsed HUGR,
designed to be serializable to RON (Rust Object Notation) for debugging
and intermediate representation.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root structure for a HUGR package/module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastModule {
    /// Module name/identifier
    pub name: String,
    /// Module version
    pub version: String,
    /// Entry point function (if specified)
    pub entry_point: Option<String>,
    /// Functions defined in the module
    pub functions: Vec<PastFunction>,
    /// Type definitions
    pub types: HashMap<String, PastType>,
}

/// Function definition in PAST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastFunction {
    /// Function name
    pub name: String,
    /// Input parameters
    pub inputs: Vec<PastParameter>,
    /// Output types
    pub outputs: Vec<PastType>,
    /// Function body as a graph of operations
    pub body: PastGraph,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastParameter {
    pub name: String,
    pub ty: PastType,
}

/// Type definitions in PAST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PastType {
    /// Quantum bit
    Qubit,
    /// Classical bit
    Bit,
    /// Integer with bit width
    Int(u8),
    /// Floating point
    Float(u8),
    /// Array type
    Array(Box<PastType>, usize),
    /// Tuple type
    Tuple(Vec<PastType>),
    /// Custom/opaque type
    Custom(String),
}

/// Graph structure representing computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastGraph {
    /// Nodes in the graph
    pub nodes: Vec<PastNode>,
    /// Edges connecting nodes
    pub edges: Vec<PastEdge>,
    /// Entry node index
    pub entry: usize,
    /// Exit node indices
    pub exits: Vec<usize>,
}

/// Node in the computation graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastNode {
    /// Unique node ID
    pub id: usize,
    /// Operation performed by this node
    pub op: PastOp,
    /// Input port count
    pub inputs: usize,
    /// Output port count  
    pub outputs: usize,
}

/// Edge connecting nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastEdge {
    /// Source node ID
    pub src: usize,
    /// Source port index
    pub src_port: usize,
    /// Destination node ID
    pub dst: usize,
    /// Destination port index
    pub dst_port: usize,
    /// Edge type (data flow, control flow, etc.)
    pub edge_type: EdgeType,
}

/// Types of edges in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    /// Data flow edge
    Data(PastType),
    /// Control flow edge
    Control,
    /// Quantum entanglement edge
    Quantum,
}

/// Operations that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PastOp {
    // Quantum operations
    /// Hadamard gate
    H,
    /// Pauli X gate
    X,
    /// Pauli Y gate  
    Y,
    /// Pauli Z gate
    Z,
    /// S gate (phase gate)
    S,
    /// T gate (pi/8 gate)
    T,
    /// S-dagger gate
    Sdg,
    /// T-dagger gate
    Tdg,
    /// Controlled-X gate
    CX,
    /// Controlled-Y gate
    CY,
    /// Controlled-Z gate
    CZ,
    /// Controlled-H gate
    CH,
    /// Controlled rotation around Z axis
    CRZ(f64),
    /// Toffoli gate (CCX)
    Toffoli,
    /// Rotation around X axis
    RX(f64),
    /// Rotation around Y axis
    RY(f64),
    /// Rotation around Z axis
    RZ(f64),
    /// Measurement
    Measure,
    /// Reset qubit to |0⟩
    Reset,
    /// Allocate qubit
    QAlloc,
    
    // Classical operations
    /// Integer addition
    Add,
    /// Integer subtraction
    Sub,
    /// Integer multiplication
    Mul,
    /// Integer division
    Div,
    /// Comparison
    Compare(CompareOp),
    /// Conditional branch
    Branch,
    
    // Memory operations
    /// Allocate qubit
    AllocQubit,
    /// Allocate classical register
    AllocBit(usize),
    /// Load from memory
    Load,
    /// Store to memory
    Store,
    
    // Control flow
    /// Function call
    Call(String),
    /// Return from function
    Return,
    /// Loop
    Loop,
    
    // Special nodes
    /// Input node
    Input(usize),
    /// Output node
    Output(usize),
    /// Constant value
    Const(PastValue),
}

/// Comparison operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Constant values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PastValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl PastModule {
    /// Convert to RON string representation
    pub fn to_ron_string(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }
    
    /// Load from RON string
    pub fn from_ron_string(s: &str) -> Result<Self, ron::Error> {
        ron::de::from_str(s).map_err(|e| match e {
            ron::de::SpannedError { code, position } => ron::Error::Message(format!("{:?} at {:?}", code, position)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ron_serialization() {
        let module = PastModule {
            name: "test_module".to_string(),
            version: "0.1.0".to_string(),
            entry_point: Some("main".to_string()),
            functions: vec![],
            types: HashMap::new(),
        };
        
        let ron_str = module.to_ron_string().unwrap();
        assert!(ron_str.contains("test_module"));
        
        let parsed = PastModule::from_ron_string(&ron_str).unwrap();
        assert_eq!(parsed.name, module.name);
    }
}