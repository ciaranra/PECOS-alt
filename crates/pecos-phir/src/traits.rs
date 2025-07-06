/*!
Operation traits and interfaces for PHIR

This module provides MLIR-style traits and interfaces that categorize and provide
common functionality for operations.
*/

use crate::ops::{ControlFlowOp, Operation, QuantumOp};
use crate::phir::{Instruction, Region};
use std::collections::HashSet;

/// Core operation traits
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpTrait {
    /// Operation has no side effects and can be eliminated if unused
    NoSideEffect,
    /// Operation is commutative (operands can be reordered)
    Commutative,
    /// Operation is associative
    Associative,
    /// Operation is a terminator (must be last in block)
    Terminator,
    /// Operation is idempotent (f(f(x)) = f(x))
    Idempotent,
    /// Operation can be speculatively executed
    Speculatable,
    /// Operation allocates resources
    AllocatesResources,
    /// Operation is a constant
    ConstantLike,
    /// Operation has recursive side effects (affects nested regions)
    RecursiveSideEffects,
    /// Operation is isolated from above (regions can't reference outside values)
    IsolatedFromAbove,
    /// Operation is pure quantum (unitary)
    PureQuantum,
    /// Operation involves measurement
    Measurement,
    /// Operation defines a symbol table scope
    SymbolTable,
}

/// Get traits for an operation
#[must_use]
pub fn get_operation_traits(op: &Operation) -> HashSet<OpTrait> {
    use crate::ops::{ClassicalOp, MemoryOp};
    use OpTrait::{
        AllocatesResources, Associative, Commutative, ConstantLike, Idempotent, IsolatedFromAbove,
        Measurement, NoSideEffect, PureQuantum, RecursiveSideEffects, Speculatable, SymbolTable,
        Terminator,
    };

    let mut traits = HashSet::new();

    match op {
        Operation::Quantum(q_op) => {
            match q_op {
                // Pure quantum gates
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
                | QuantumOp::U3(_, _, _)
                | QuantumOp::CX
                | QuantumOp::CZ
                | QuantumOp::SWAP
                | QuantumOp::CPhase(_)
                | QuantumOp::RZZ(_) => {
                    traits.insert(PureQuantum);
                    traits.insert(NoSideEffect);
                }

                // Measurement operations
                QuantumOp::Measure
                | QuantumOp::MeasurePauli(_)
                | QuantumOp::MeasureExpectation(_) => {
                    traits.insert(Measurement);
                    // Measurements have side effects
                }

                // Resource management
                QuantumOp::Alloc => {
                    traits.insert(AllocatesResources);
                }
                QuantumOp::Dealloc | QuantumOp::Reset => {
                    // Has side effects
                }

                // State preparation
                QuantumOp::InitZero
                | QuantumOp::InitOne
                | QuantumOp::InitPlus
                | QuantumOp::InitMinus
                | QuantumOp::InitState(_) => {
                    // Has side effects (modifies qubit state)
                }

                _ => {}
            }
        }

        Operation::Classical(c_op) => {
            match c_op {
                // Arithmetic operations
                ClassicalOp::Add | ClassicalOp::Mul | ClassicalOp::FAdd | ClassicalOp::FMul => {
                    traits.insert(NoSideEffect);
                    traits.insert(Commutative);
                    traits.insert(Associative);
                }

                ClassicalOp::Sub
                | ClassicalOp::Div
                | ClassicalOp::Mod
                | ClassicalOp::FSub
                | ClassicalOp::FDiv => {
                    traits.insert(NoSideEffect);
                    // Not commutative or associative
                }

                ClassicalOp::Neg
                | ClassicalOp::FNeg
                | ClassicalOp::Sqrt
                | ClassicalOp::Sin
                | ClassicalOp::Cos
                | ClassicalOp::Tan => {
                    traits.insert(NoSideEffect);
                    traits.insert(Speculatable);
                }

                // Bitwise operations
                ClassicalOp::And | ClassicalOp::Or | ClassicalOp::Xor => {
                    traits.insert(NoSideEffect);
                    traits.insert(Commutative);
                    traits.insert(Associative);
                }

                ClassicalOp::Not => {
                    traits.insert(NoSideEffect);
                    traits.insert(Idempotent);
                }

                // Constants
                ClassicalOp::ConstInt(_)
                | ClassicalOp::ConstFloat(_)
                | ClassicalOp::ConstBool(_)
                | ClassicalOp::ConstString(_) => {
                    traits.insert(NoSideEffect);
                    traits.insert(ConstantLike);
                    traits.insert(Speculatable);
                }

                _ => {
                    traits.insert(NoSideEffect);
                }
            }
        }

        Operation::ControlFlow(cf_op) => {
            match cf_op {
                ControlFlowOp::Return => {
                    traits.insert(Terminator);
                }
                ControlFlowOp::Branch(_) | ControlFlowOp::Jump(_) => {
                    traits.insert(Terminator);
                }
                ControlFlowOp::Loop(_) => {
                    traits.insert(RecursiveSideEffects);
                    traits.insert(IsolatedFromAbove);
                }
                ControlFlowOp::Call(_) => {
                    // Function calls may have side effects
                }
                ControlFlowOp::Parallel | ControlFlowOp::Barrier => {
                    // Synchronization has side effects
                }
            }
        }

        Operation::Memory(m_op) => {
            match m_op {
                MemoryOp::Alloc(_) => {
                    traits.insert(AllocatesResources);
                }
                MemoryOp::Load => {
                    traits.insert(Speculatable);
                }
                MemoryOp::Store => {
                    // Has side effects
                }
                _ => {}
            }
        }

        Operation::Custom(_) => {
            // Custom operations must specify their own traits
        }

        Operation::Builtin(_) => {
            // Builtin structural operations (module, func, etc.)
            traits.insert(NoSideEffect);
            traits.insert(SymbolTable);
        }

        Operation::Parsing(_) => {
            // Parsing operations are temporary and have no side effects
            traits.insert(NoSideEffect);
        }
    }

    traits
}

/// Operation interface for common functionality
pub trait OperationInterface {
    /// Check if operation has a specific trait
    fn has_trait(&self, trait_: OpTrait) -> bool;

    /// Check if operation has side effects
    fn has_side_effects(&self) -> bool;

    /// Check if operation is a terminator
    fn is_terminator(&self) -> bool;

    /// Check if operation can be eliminated if results are unused
    fn is_dead_if_unused(&self) -> bool;

    /// Get the regions this operation contains
    fn regions(&self) -> &[Region];

    /// Verify operation invariants
    fn verify(&self) -> Result<(), String>;
}

impl OperationInterface for Instruction {
    fn has_trait(&self, trait_: OpTrait) -> bool {
        get_operation_traits(&self.operation).contains(&trait_)
    }

    fn has_side_effects(&self) -> bool {
        !self.has_trait(OpTrait::NoSideEffect)
    }

    fn is_terminator(&self) -> bool {
        self.has_trait(OpTrait::Terminator)
    }

    fn is_dead_if_unused(&self) -> bool {
        self.has_trait(OpTrait::NoSideEffect) && !self.has_trait(OpTrait::AllocatesResources)
    }

    fn regions(&self) -> &[Region] {
        &self.regions
    }

    fn verify(&self) -> Result<(), String> {
        // Basic verification

        // Terminators should not have regions
        if self.is_terminator() && !self.regions.is_empty() {
            return Err("Terminator operations cannot have regions".to_string());
        }

        // Check result types match number of results
        if self.results.len() != self.result_types.len() {
            return Err(format!(
                "Mismatch between number of results ({}) and result types ({})",
                self.results.len(),
                self.result_types.len()
            ));
        }

        // Additional verification based on operation type
        match &self.operation {
            Operation::Quantum(QuantumOp::Measure) => {
                if self.operands.is_empty() {
                    return Err("Measure operation requires at least one qubit operand".to_string());
                }
                if self.results.is_empty() {
                    return Err("Measure operation must produce at least one result".to_string());
                }
            }
            Operation::ControlFlow(ControlFlowOp::Loop(_)) => {
                if self.regions.is_empty() {
                    return Err("Loop operation must have at least one region".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{ClassicalOp, Operation, QuantumOp, SSAValue};
    use crate::types::Type;

    #[test]
    fn test_quantum_traits() {
        let h_op = Operation::Quantum(QuantumOp::H);
        let traits = get_operation_traits(&h_op);
        assert!(traits.contains(&OpTrait::PureQuantum));
        assert!(traits.contains(&OpTrait::NoSideEffect));

        let measure_op = Operation::Quantum(QuantumOp::Measure);
        let traits = get_operation_traits(&measure_op);
        assert!(traits.contains(&OpTrait::Measurement));
        assert!(!traits.contains(&OpTrait::NoSideEffect));
    }

    #[test]
    fn test_classical_traits() {
        let add_op = Operation::Classical(ClassicalOp::Add);
        let traits = get_operation_traits(&add_op);
        assert!(traits.contains(&OpTrait::NoSideEffect));
        assert!(traits.contains(&OpTrait::Commutative));
        assert!(traits.contains(&OpTrait::Associative));

        let const_op = Operation::Classical(ClassicalOp::ConstInt(42));
        let traits = get_operation_traits(&const_op);
        assert!(traits.contains(&OpTrait::ConstantLike));
        assert!(traits.contains(&OpTrait::Speculatable));
    }

    #[test]
    fn test_operation_interface() {
        let inst = Instruction::new(
            Operation::Quantum(QuantumOp::H),
            vec![SSAValue::new(1)],
            vec![SSAValue::new(2)],
            vec![Type::Qubit],
        );

        assert!(!inst.has_side_effects());
        assert!(!inst.is_terminator());
        assert!(inst.is_dead_if_unused());
        assert!(inst.verify().is_ok());
    }
}
