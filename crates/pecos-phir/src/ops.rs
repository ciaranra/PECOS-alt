/*!
Core operation definitions for PHIR

This module defines the complete operation set for PHIR, including:
- Builtin operations (Module, Function, etc.)
- Quantum operations (gates, measurements, state preparation)
- Classical operations (arithmetic, logic, comparisons)
- Control flow operations (branches, loops, calls)
- Memory operations (allocation, load/store)
- Parsing operations (for direct parsing to PHIR)
- Custom/dialect operations

All operations follow MLIR's design where operations can contain nested regions.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core operation enum for PHIR
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    /// Builtin structural operations (module, func, etc.)
    Builtin(crate::builtin_ops::BuiltinOp),
    /// Quantum operations (gates, measurements, state preparation)
    Quantum(QuantumOp),
    /// Classical arithmetic and logic operations  
    Classical(ClassicalOp),
    /// Control flow operations (branches, loops, function calls)
    ControlFlow(ControlFlowOp),
    /// Memory operations (allocation, load, store)
    Memory(MemoryOp),
    /// Custom/extension operations from dialects
    Custom(CustomOp),
    /// Parsing-specific operations (unresolved refs, type inference, etc.)
    Parsing(crate::parsing_ops::ParsingOp),
}

/// Quantum operations
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum QuantumOp {
    // Single-qubit gates
    /// Hadamard gate
    H,
    /// Pauli-X gate
    X,
    /// Pauli-Y gate  
    Y,
    /// Pauli-Z gate
    Z,
    /// S gate (phase)
    S,
    /// S† gate
    Sdg,
    /// T gate
    T,
    /// T† gate
    Tdg,

    // Parameterized single-qubit rotations
    /// X-axis rotation
    RX(f64),
    /// Y-axis rotation
    RY(f64),
    /// Z-axis rotation
    RZ(f64),
    /// Arbitrary single-qubit rotation
    U3(f64, f64, f64), // theta, phi, lambda

    // Two-qubit gates
    /// CNOT/CX gate
    CX,
    /// CZ gate
    CZ,
    /// SWAP gate
    SWAP,
    /// Controlled phase
    CPhase(f64),
    /// ZZ rotation
    RZZ(f64),

    // Multi-qubit gates
    /// Multi-controlled NOT
    MCX(usize), // number of controls
    /// Multi-controlled Z
    MCZ(usize),
    /// Toffoli (CCX)
    Toffoli,
    /// Fredkin (CSWAP)
    Fredkin,

    // Measurements
    /// Computational basis measurement
    Measure,
    /// Pauli basis measurement
    MeasurePauli(PauliBasis),
    /// Expectation value measurement
    MeasureExpectation(String), // observable name

    // State preparation
    /// Initialize qubit to |0⟩
    InitZero,
    /// Initialize qubit to |1⟩  
    InitOne,
    /// Initialize qubit to |+⟩
    InitPlus,
    /// Initialize qubit to |-⟩
    InitMinus,
    /// Initialize to arbitrary state
    InitState(Vec<Complex>),

    // Resource management
    /// Allocate fresh qubit
    Alloc,
    /// Deallocate qubit (must be in |0⟩)
    Dealloc,
    /// Reset qubit to |0⟩
    Reset,
}

/// Classical arithmetic and logic operations
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClassicalOp {
    // Arithmetic
    /// Integer addition
    Add,
    /// Integer subtraction
    Sub,
    /// Integer multiplication
    Mul,
    /// Integer division
    Div,
    /// Modulo operation
    Mod,
    /// Negation
    Neg,

    // Floating point
    /// Float addition
    FAdd,
    /// Float subtraction
    FSub,
    /// Float multiplication
    FMul,
    /// Float division
    FDiv,
    /// Float negation
    FNeg,
    /// Float square root
    Sqrt,
    /// Float power
    Pow,
    /// Trigonometric functions
    Sin,
    Cos,
    Tan,

    // Bitwise operations
    /// Bitwise AND
    And,
    /// Bitwise OR
    Or,
    /// Bitwise XOR
    Xor,
    /// Bitwise NOT
    Not,
    /// Left shift
    Shl(u32),
    /// Right shift
    Shr(u32),

    // Comparisons
    /// Equality
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,

    // Type conversions
    /// Integer to float
    IntToFloat,
    /// Float to integer
    FloatToInt,
    /// Bitcast
    Bitcast,

    // Constants
    /// Integer constant
    ConstInt(i64),
    /// Float constant
    ConstFloat(f64),
    /// Boolean constant
    ConstBool(bool),
    /// String constant
    ConstString(String),
}

/// Control flow operations
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlFlowOp {
    /// Function call
    Call(FunctionCall),
    /// Function return
    Return,
    /// Conditional branch
    Branch(BranchType),
    /// Unconditional jump
    Jump(String), // block name
    /// Loop constructs
    Loop(LoopType),
    /// Parallel execution
    Parallel,
    /// Synchronization barrier
    Barrier,
}

/// Memory management operations
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MemoryOp {
    /// Allocate memory
    Alloc(AllocType),
    /// Load from memory
    Load,
    /// Store to memory
    Store,
    /// Copy memory
    Copy,
    /// Get array element
    ArrayGet,
    /// Set array element
    ArraySet,
    /// Get array length
    ArrayLen,
    /// Create array from elements
    ArrayCreate,
}

/// Custom operations from dialect extensions
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomOp {
    /// Dialect namespace (e.g., "qec", "pulse", "chem")
    pub dialect: String,
    /// Operation name within dialect
    pub name: String,
    /// Operation-specific attributes
    pub attributes: HashMap<String, crate::phir::AttributeValue>,
}

// Supporting types

/// Pauli measurement basis
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PauliBasis {
    X,
    Y,
    Z,
}

/// Complex number representation
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

/// Function call details
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<ValueRef>,
}

/// Branch type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BranchType {
    /// if-then
    Conditional {
        condition: ValueRef,
        then_block: String,
        else_block: Option<String>,
    },
    /// switch statement
    Switch {
        value: ValueRef,
        cases: Vec<(i64, String)>, // (case_value, block_name)
        default: Option<String>,
    },
}

/// Loop constructs
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LoopType {
    /// while loop
    While {
        condition: ValueRef,
        body_block: String,
    },
    /// for loop
    For {
        init: ValueRef,
        condition: ValueRef,
        step: ValueRef,
        body_block: String,
    },
    /// Fixed iteration count
    Repeat { count: ValueRef, body_block: String },
}

/// Memory allocation types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AllocType {
    /// Single value
    Scalar(crate::types::Type),
    /// Array allocation
    Array(crate::types::Type, ValueRef), // type, size
    /// Stack allocation
    Stack(usize), // size in bytes
}

/// Value reference (operand in operations)
#[derive(Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
pub enum ValueRef {
    /// SSA value reference (for PHIR)
    SSA(SSAValue),
    /// Variable name reference (for parsing operations)
    Variable(String),
    /// Immediate constant
    Constant(ConstantValue),
    /// Block argument
    BlockArg(usize),
}

/// SSA value identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SSAValue {
    pub id: u32,
    pub version: u32, // For phi nodes and versioning
}

/// Constant values
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConstantValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<ConstantValue>),
}

impl std::hash::Hash for ConstantValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ConstantValue::Int(i) => i.hash(state),
            ConstantValue::Float(f) => f.to_bits().hash(state), // Hash bit representation
            ConstantValue::Bool(b) => b.hash(state),
            ConstantValue::String(s) => s.hash(state),
            ConstantValue::Array(arr) => arr.hash(state),
        }
    }
}

/// Operation attributes (compile-time metadata)
#[derive(Clone, Debug, PartialEq)]
pub enum Attribute {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Attribute>),
    Dict(HashMap<String, Attribute>),
}

impl Operation {
    /// Get the dialect namespace for this operation
    #[must_use]
    pub fn dialect(&self) -> String {
        match self {
            Operation::Builtin(_) => "builtin".to_string(),
            Operation::Quantum(_) => "quantum".to_string(),
            Operation::Classical(_) => "arith".to_string(),
            Operation::ControlFlow(_) => "control".to_string(),
            Operation::Memory(_) => "memory".to_string(),
            Operation::Custom(op) => op.dialect.clone(),
            Operation::Parsing(_) => "parse".to_string(),
        }
    }

    /// Get the operation name within its dialect
    #[must_use]
    pub fn name(&self) -> String {
        use crate::builtin_ops::BuiltinOp;
        use crate::parsing_ops::ParsingOp;
        match self {
            Operation::Builtin(op) => match op {
                BuiltinOp::Module(_) => "module".to_string(),
                BuiltinOp::Func(_) => "func.func".to_string(),
                BuiltinOp::Return(_) => "return".to_string(),
            },
            Operation::Quantum(op) => format!("quantum.{}", op.name()),
            Operation::Classical(op) => format!("arith.{}", op.name()),
            Operation::ControlFlow(op) => format!("control.{}", op.name()),
            Operation::Memory(op) => format!("memory.{}", op.name()),
            Operation::Custom(op) => format!("{}.{}", op.dialect, op.name),
            Operation::Parsing(op) => match op {
                ParsingOp::UnresolvedCall(_) => "parse.unresolved_call".to_string(),
                ParsingOp::UnresolvedRef(_) => "parse.unresolved_ref".to_string(),
                ParsingOp::ForwardDecl(_) => "parse.forward_decl".to_string(),
                ParsingOp::ImplicitCast(_) => "parse.implicit_cast".to_string(),
                ParsingOp::ForLoop(_) => "parse.for_loop".to_string(),
                ParsingOp::IfElse(_) => "parse.if_else".to_string(),
                ParsingOp::InferType(_) => "parse.infer_type".to_string(),
            },
        }
    }

    /// Check if operation has side effects
    #[must_use]
    pub fn has_side_effects(&self) -> bool {
        match self {
            Operation::Builtin(_) => false, // Structural ops have no side effects
            Operation::Quantum(op) => match op {
                QuantumOp::Measure
                | QuantumOp::MeasurePauli(_)
                | QuantumOp::MeasureExpectation(_)
                | QuantumOp::Alloc
                | QuantumOp::Dealloc
                | QuantumOp::Reset => true,
                _ => false, // Most quantum operations are unitary
            },
            Operation::Memory(_) => true,
            Operation::ControlFlow(_) => true,
            Operation::Classical(_) => false,
            Operation::Custom(_) => true,   // Conservative assumption
            Operation::Parsing(_) => false, // Parsing ops have no runtime side effects
        }
    }

    /// Get expected number of operands
    #[must_use]
    pub fn operand_count(&self) -> Option<usize> {
        use crate::builtin_ops::BuiltinOp;
        match self {
            Operation::Builtin(op) => match op {
                BuiltinOp::Module(_) => Some(0),
                BuiltinOp::Func(_) => Some(0),
                BuiltinOp::Return(ret) => Some(ret.operands.len()),
            },
            Operation::Quantum(op) => op.operand_count(),
            Operation::Classical(op) => op.operand_count(),
            Operation::ControlFlow(op) => op.operand_count(),
            Operation::Memory(op) => op.operand_count(),
            Operation::Custom(_) => None,  // Variable
            Operation::Parsing(_) => None, // Variable for parsing ops
        }
    }
}

impl QuantumOp {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            QuantumOp::H => "h",
            QuantumOp::X => "x",
            QuantumOp::Y => "y",
            QuantumOp::Z => "z",
            QuantumOp::S => "s",
            QuantumOp::Sdg => "sdg",
            QuantumOp::T => "t",
            QuantumOp::Tdg => "tdg",
            QuantumOp::RX(_) => "rx",
            QuantumOp::RY(_) => "ry",
            QuantumOp::RZ(_) => "rz",
            QuantumOp::U3(_, _, _) => "u3",
            QuantumOp::CX => "cx",
            QuantumOp::CZ => "cz",
            QuantumOp::SWAP => "swap",
            QuantumOp::CPhase(_) => "cp",
            QuantumOp::RZZ(_) => "rzz",
            QuantumOp::MCX(_) => "mcx",
            QuantumOp::MCZ(_) => "mcz",
            QuantumOp::Toffoli => "ccx",
            QuantumOp::Fredkin => "cswap",
            QuantumOp::Measure => "measure",
            QuantumOp::MeasurePauli(_) => "measure_pauli",
            QuantumOp::MeasureExpectation(_) => "measure_expectation",
            QuantumOp::InitZero => "init_zero",
            QuantumOp::InitOne => "init_one",
            QuantumOp::InitPlus => "init_plus",
            QuantumOp::InitMinus => "init_minus",
            QuantumOp::InitState(_) => "init_state",
            QuantumOp::Alloc => "alloc",
            QuantumOp::Dealloc => "dealloc",
            QuantumOp::Reset => "reset",
        }
    }

    #[must_use]
    pub fn operand_count(&self) -> Option<usize> {
        match self {
            // Single-qubit gates
            QuantumOp::H
            | QuantumOp::X
            | QuantumOp::Y
            | QuantumOp::Z
            | QuantumOp::S
            | QuantumOp::Sdg
            | QuantumOp::T
            | QuantumOp::Tdg
            | QuantumOp::RX(_)
            | QuantumOp::RY(_)
            | QuantumOp::RZ(_)
            | QuantumOp::Measure
            | QuantumOp::MeasurePauli(_)
            | QuantumOp::Reset
            | QuantumOp::Dealloc => Some(1),

            // Two-qubit gates
            QuantumOp::CX
            | QuantumOp::CZ
            | QuantumOp::SWAP
            | QuantumOp::CPhase(_)
            | QuantumOp::RZZ(_) => Some(2),

            // Three-qubit gates
            QuantumOp::U3(_, _, _) => Some(1),
            QuantumOp::Toffoli | QuantumOp::Fredkin => Some(3),

            // Multi-qubit gates (variable)
            QuantumOp::MCX(n) | QuantumOp::MCZ(n) => Some(*n + 1),

            // No operands
            QuantumOp::Alloc
            | QuantumOp::InitZero
            | QuantumOp::InitOne
            | QuantumOp::InitPlus
            | QuantumOp::InitMinus => Some(0),

            // Variable operands
            QuantumOp::InitState(_) | QuantumOp::MeasureExpectation(_) => None,
        }
    }

    /// Check if operation is unitary (reversible)
    #[must_use]
    pub fn is_unitary(&self) -> bool {
        match self {
            QuantumOp::Measure
            | QuantumOp::MeasurePauli(_)
            | QuantumOp::MeasureExpectation(_)
            | QuantumOp::Reset
            | QuantumOp::Alloc
            | QuantumOp::Dealloc
            | QuantumOp::InitZero
            | QuantumOp::InitOne
            | QuantumOp::InitPlus
            | QuantumOp::InitMinus
            | QuantumOp::InitState(_) => false,
            _ => true,
        }
    }
}

impl ClassicalOp {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            ClassicalOp::Add => "add",
            ClassicalOp::Sub => "sub",
            ClassicalOp::Mul => "mul",
            ClassicalOp::Div => "div",
            ClassicalOp::Mod => "mod",
            ClassicalOp::Neg => "neg",
            ClassicalOp::FAdd => "fadd",
            ClassicalOp::FSub => "fsub",
            ClassicalOp::FMul => "fmul",
            ClassicalOp::FDiv => "fdiv",
            ClassicalOp::FNeg => "fneg",
            ClassicalOp::Sqrt => "sqrt",
            ClassicalOp::Pow => "pow",
            ClassicalOp::Sin => "sin",
            ClassicalOp::Cos => "cos",
            ClassicalOp::Tan => "tan",
            ClassicalOp::And => "and",
            ClassicalOp::Or => "or",
            ClassicalOp::Xor => "xor",
            ClassicalOp::Not => "not",
            ClassicalOp::Shl(_) => "shl",
            ClassicalOp::Shr(_) => "shr",
            ClassicalOp::Eq => "eq",
            ClassicalOp::Ne => "ne",
            ClassicalOp::Lt => "lt",
            ClassicalOp::Le => "le",
            ClassicalOp::Gt => "gt",
            ClassicalOp::Ge => "ge",
            ClassicalOp::IntToFloat => "int_to_float",
            ClassicalOp::FloatToInt => "float_to_int",
            ClassicalOp::Bitcast => "bitcast",
            ClassicalOp::ConstInt(_) => "const_int",
            ClassicalOp::ConstFloat(_) => "const_float",
            ClassicalOp::ConstBool(_) => "const_bool",
            ClassicalOp::ConstString(_) => "const_string",
        }
    }

    #[must_use]
    pub fn operand_count(&self) -> Option<usize> {
        match self {
            // Binary operations
            ClassicalOp::Add
            | ClassicalOp::Sub
            | ClassicalOp::Mul
            | ClassicalOp::Div
            | ClassicalOp::Mod
            | ClassicalOp::FAdd
            | ClassicalOp::FSub
            | ClassicalOp::FMul
            | ClassicalOp::FDiv
            | ClassicalOp::Pow
            | ClassicalOp::And
            | ClassicalOp::Or
            | ClassicalOp::Xor
            | ClassicalOp::Eq
            | ClassicalOp::Ne
            | ClassicalOp::Lt
            | ClassicalOp::Le
            | ClassicalOp::Gt
            | ClassicalOp::Ge => Some(2),

            // Unary operations
            ClassicalOp::Neg
            | ClassicalOp::FNeg
            | ClassicalOp::Not
            | ClassicalOp::Sqrt
            | ClassicalOp::Sin
            | ClassicalOp::Cos
            | ClassicalOp::Tan
            | ClassicalOp::IntToFloat
            | ClassicalOp::FloatToInt
            | ClassicalOp::Bitcast => Some(1),

            // Shift operations
            ClassicalOp::Shl(_) | ClassicalOp::Shr(_) => Some(1),

            // Constants (no operands)
            ClassicalOp::ConstInt(_)
            | ClassicalOp::ConstFloat(_)
            | ClassicalOp::ConstBool(_)
            | ClassicalOp::ConstString(_) => Some(0),
        }
    }
}

impl ControlFlowOp {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            ControlFlowOp::Call(_) => "call",
            ControlFlowOp::Return => "return",
            ControlFlowOp::Branch(_) => "branch",
            ControlFlowOp::Jump(_) => "jump",
            ControlFlowOp::Loop(_) => "loop",
            ControlFlowOp::Parallel => "parallel",
            ControlFlowOp::Barrier => "barrier",
        }
    }

    #[must_use]
    pub fn operand_count(&self) -> Option<usize> {
        match self {
            ControlFlowOp::Call(call) => Some(call.args.len()),
            ControlFlowOp::Return => None,       // Variable
            ControlFlowOp::Branch(_) => Some(1), // Condition
            ControlFlowOp::Jump(_) => Some(0),
            ControlFlowOp::Loop(_) => None, // Variable
            ControlFlowOp::Parallel => Some(0),
            ControlFlowOp::Barrier => Some(0),
        }
    }
}

impl MemoryOp {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            MemoryOp::Alloc(_) => "alloc",
            MemoryOp::Load => "load",
            MemoryOp::Store => "store",
            MemoryOp::Copy => "copy",
            MemoryOp::ArrayGet => "array_get",
            MemoryOp::ArraySet => "array_set",
            MemoryOp::ArrayLen => "array_len",
            MemoryOp::ArrayCreate => "array_create",
        }
    }

    #[must_use]
    pub fn operand_count(&self) -> Option<usize> {
        match self {
            MemoryOp::Alloc(_) => Some(0),
            MemoryOp::Load => Some(1),     // address
            MemoryOp::Store => Some(2),    // address, value
            MemoryOp::Copy => Some(3),     // src, dst, size
            MemoryOp::ArrayGet => Some(2), // array, index
            MemoryOp::ArraySet => Some(3), // array, index, value
            MemoryOp::ArrayLen => Some(1), // array
            MemoryOp::ArrayCreate => None, // Variable number of elements
        }
    }
}

impl SSAValue {
    #[must_use]
    pub fn new(id: u32) -> Self {
        Self { id, version: 0 }
    }

    #[must_use]
    pub fn with_version(id: u32, version: u32) -> Self {
        Self { id, version }
    }
}

impl std::fmt::Display for SSAValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.version == 0 {
            write!(f, "%{}", self.id)
        } else {
            write!(f, "%{}.{}", self.id, self.version)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_names() {
        assert_eq!(Operation::Quantum(QuantumOp::H).name(), "quantum.h");
        assert_eq!(Operation::Classical(ClassicalOp::Add).name(), "arith.add");
        assert_eq!(
            Operation::ControlFlow(ControlFlowOp::Return).name(),
            "control.return"
        );
    }

    #[test]
    fn test_quantum_op_properties() {
        assert!(QuantumOp::H.is_unitary());
        assert!(!QuantumOp::Measure.is_unitary());

        assert_eq!(QuantumOp::CX.operand_count(), Some(2));
        assert_eq!(QuantumOp::Toffoli.operand_count(), Some(3));
    }

    #[test]
    fn test_ssa_value_display() {
        let val1 = SSAValue::new(42);
        assert_eq!(val1.to_string(), "%42");

        let val2 = SSAValue::with_version(42, 3);
        assert_eq!(val2.to_string(), "%42.3");
    }
}
