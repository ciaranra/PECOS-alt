/*!
HUGR to QIS Conversion Pass

This module provides a conversion pass that translates HUGR dialect operations
to QIS dialect operations. This follows the same decomposition strategy used by
Selene's hugr-qis compiler.

The conversion maps high-level quantum gates to hardware-native gates:
- Hadamard (H) → RZ(-π/2), RXY(π/2, 0), RZ(-π/2)
- CNOT/CX → RXY(π/2, 0) on target, RZZ(π/2), RZ(-π/2) on control, RXY(-π/2, 0) on target
- RX(θ) → RXY(θ, 0)
- RY(θ) → RXY(θ, π/2)
*/

use crate::error::Result;
use crate::ops::{Operation, CustomOp};
use crate::phir::{Block, Instruction, Module, Region, SSAValue};
use std::collections::HashMap;
use std::f64::consts::PI;

/// Convert a HUGR module to use QIS operations
pub fn convert_hugr_to_qis(module: &mut Module) -> Result<()> {
    let mut converter = HugrToQisConverter::new();
    converter.convert_module(module)
}

struct HugrToQisConverter {
    /// Map from HUGR qubit values to QIS qubit IDs
    #[allow(dead_code)]
    qubit_map: HashMap<SSAValue, SSAValue>,
    /// Counter for generating fresh SSA values
    next_value_id: u32,
}

impl HugrToQisConverter {
    fn new() -> Self {
        Self {
            qubit_map: HashMap::new(),
            next_value_id: 1000, // Start from a high number to avoid conflicts
        }
    }

    fn fresh_value(&mut self) -> SSAValue {
        let value = SSAValue::new(self.next_value_id);
        self.next_value_id += 1;
        value
    }

    fn convert_module(&mut self, module: &mut Module) -> Result<()> {
        // Process the module's body region
        self.convert_region(&mut module.body)?;
        Ok(())
    }

    fn convert_region(&mut self, region: &mut Region) -> Result<()> {
        for block in &mut region.blocks {
            self.convert_block(block)?;
        }
        Ok(())
    }

    fn convert_block(&mut self, block: &mut Block) -> Result<()> {
        let mut new_instructions = Vec::new();

        for instruction in &block.operations {
            match &instruction.operation {
                Operation::Custom(custom_op) if custom_op.dialect() == "hugr" => {
                    // Convert HUGR operations to QIS
                    let qis_ops = self.convert_hugr_op(custom_op, &instruction.operands, &instruction.results)?;
                    new_instructions.extend(qis_ops);
                }
                _ => {
                    // Keep non-HUGR operations as-is
                    new_instructions.push(instruction.clone());
                }
            }
        }

        block.operations = new_instructions;
        Ok(())
    }

    fn convert_hugr_op(&mut self, op: &CustomOp, operands: &[SSAValue], results: &[SSAValue]) -> Result<Vec<Instruction>> {
        let mut instructions = Vec::new();

        match op.name() {
            "qalloc" => {
                // HUGR qalloc → QIS qalloc
                let qis_op = CustomOp::new("qis", "qalloc", vec![], HashMap::new());
                instructions.push(Instruction {
                    results: results.to_vec(),
                    operation: Operation::Custom(qis_op),
                    operands: vec![],
                    result_types: vec![crate::types::Type::Qubit],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "qfree" => {
                // HUGR qfree → QIS qfree
                let qis_op = CustomOp::new("qis", "qfree", vec![], HashMap::new());
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(qis_op),
                    operands: operands.to_vec(),
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "h" => {
                // Hadamard decomposition: H = RZ(-π/2) · RXY(π/2, 0) · RZ(-π/2)
                let qubit = &operands[0];

                // RZ(-π/2)
                let rz1 = CustomOp::new(
                    "qis",
                    "rz",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rz1),
                    operands: vec![qubit.clone(), self.make_float_constant(-PI / 2.0)],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });

                // RXY(π/2, 0)
                let rxy = CustomOp::new(
                    "qis",
                    "rxy",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rxy),
                    operands: vec![
                        qubit.clone(),
                        self.make_float_constant(PI / 2.0),
                        self.make_float_constant(0.0),
                    ],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });

                // RZ(-π/2)
                let rz2 = CustomOp::new(
                    "qis",
                    "rz",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rz2),
                    operands: vec![qubit.clone(), self.make_float_constant(-PI / 2.0)],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "cx" => {
                // CNOT decomposition using RXY and RZZ
                let control = &operands[0];
                let target = &operands[1];

                // RXY(π/2, 0) on target
                let rxy1 = CustomOp::new(
                    "qis",
                    "rxy",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rxy1),
                    operands: vec![
                        target.clone(),
                        self.make_float_constant(PI / 2.0),
                        self.make_float_constant(0.0),
                    ],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });

                // RZZ(π/2) on control and target
                let rzz = CustomOp::new(
                    "qis",
                    "rzz",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rzz),
                    operands: vec![
                        control.clone(),
                        target.clone(),
                        self.make_float_constant(PI / 2.0),
                    ],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });

                // RZ(-π/2) on control
                let rz = CustomOp::new(
                    "qis",
                    "rz",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rz),
                    operands: vec![control.clone(), self.make_float_constant(-PI / 2.0)],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });

                // RXY(-π/2, 0) on target
                let rxy2 = CustomOp::new(
                    "qis",
                    "rxy",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rxy2),
                    operands: vec![
                        target.clone(),
                        self.make_float_constant(-PI / 2.0),
                        self.make_float_constant(0.0),
                    ],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "rx" => {
                // RX(θ) → RXY(θ, 0)
                let qubit = &operands[0];
                let angle = &operands[1];

                let rxy = CustomOp::new(
                    "qis",
                    "rxy",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rxy),
                    operands: vec![qubit.clone(), angle.clone(), self.make_float_constant(0.0)],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "ry" => {
                // RY(θ) → RXY(θ, π/2)
                let qubit = &operands[0];
                let angle = &operands[1];

                let rxy = CustomOp::new(
                    "qis",
                    "rxy",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(rxy),
                    operands: vec![
                        qubit.clone(),
                        angle.clone(),
                        self.make_float_constant(PI / 2.0),
                    ],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "rz" => {
                // RZ(θ) → QIS RZ(θ) (direct mapping)
                let qubit = &operands[0];
                let angle = &operands[1];

                let qis_rz = CustomOp::new(
                    "qis",
                    "rz",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![],
                    operation: Operation::Custom(qis_rz),
                    operands: vec![qubit.clone(), angle.clone()],
                    result_types: vec![],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            "measure" => {
                // HUGR measure → QIS lazy_measure + read_future
                let qubit = &operands[0];

                // Create a future for the measurement
                let future = self.fresh_value();
                let lazy_measure = CustomOp::new(
                    "qis",
                    "lazy_measure",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: vec![future.clone()],
                    operation: Operation::Custom(lazy_measure),
                    operands: vec![qubit.clone()],
                    result_types: vec![crate::types::Type::Future],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });

                // Read the future to get the result
                let read_future = CustomOp::new(
                    "qis",
                    "read_future",
                    vec![],
                    HashMap::new()
                );
                instructions.push(Instruction {
                    results: results.to_vec(),
                    operation: Operation::Custom(read_future),
                    operands: vec![future],
                    result_types: vec![crate::types::Type::Bool],
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }

            _ => {
                // For other HUGR operations, keep as-is for now
                // In a complete implementation, all HUGR ops would be converted
                instructions.push(Instruction {
                    results: results.to_vec(),
                    operation: Operation::Custom(op.clone()),
                    operands: operands.to_vec(),
                    result_types: vec![], // Unknown types for unhandled ops
                    regions: vec![],
                    attributes: HashMap::new(),
                    location: None,
                });
            }
        }

        Ok(instructions)
    }

    fn make_float_constant(&mut self, _value: f64) -> SSAValue {
        // In a real implementation, this would create a proper constant
        // For now, we just create a placeholder SSA value
        let const_value = self.fresh_value();
        // This would normally emit a constant operation
        const_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phir::{Block, Module, Region};

    #[test]
    fn test_hadamard_decomposition() {
        // Create a module with a Hadamard gate
        let mut module = Module {
            name: "test".to_string(),
            attributes: HashMap::new(),
            body: Region {
                kind: crate::region_kinds::RegionKind::Graph,
                attributes: HashMap::new(),
                blocks: vec![Block {
                    label: None,
                    arguments: vec![],
                    attributes: HashMap::new(),
                    operations: vec![Instruction {
                        results: vec![],
                        operation: Operation::Custom(CustomOp::new(
                            "hugr",
                            "h",
                            vec![],
                            HashMap::new(),
                        )),
                        operands: vec![SSAValue::new(0)],  // q0
                        result_types: vec![],
                        regions: vec![],
                        attributes: HashMap::new(),
                        location: None,
                    }],
                    terminator: None,
                }],
            },
        };

        // Convert HUGR to QIS
        let mut converter = HugrToQisConverter::new();
        converter.convert_module(&mut module).unwrap();

        // Check that we have 3 QIS operations (RZ, RXY, RZ)
        assert_eq!(module.body.blocks[0].operations.len(), 3);

        // Verify the operations are correct QIS ops
        for op in &module.body.blocks[0].operations {
            if let Operation::Custom(custom_op) = &op.operation {
                assert_eq!(custom_op.dialect(), "qis");
                assert!(custom_op.name() == "rz" || custom_op.name() == "rxy");
            }
        }
    }

    #[test]
    fn test_cnot_decomposition() {
        // Create a module with a CNOT gate
        let mut module = Module {
            name: "test".to_string(),
            attributes: HashMap::new(),
            body: Region {
                kind: crate::region_kinds::RegionKind::Graph,
                attributes: HashMap::new(),
                blocks: vec![Block {
                    label: None,
                    arguments: vec![],
                    attributes: HashMap::new(),
                    operations: vec![Instruction {
                        results: vec![],
                        operation: Operation::Custom(CustomOp::new(
                            "hugr",
                            "cx",
                            vec![],
                            HashMap::new(),
                        )),
                        operands: vec![SSAValue::new(0), SSAValue::new(1)],  // q0, q1
                        result_types: vec![],
                        regions: vec![],
                        attributes: HashMap::new(),
                        location: None,
                    }],
                    terminator: None,
                }],
            },
        };

        // Convert HUGR to QIS
        let mut converter = HugrToQisConverter::new();
        converter.convert_module(&mut module).unwrap();

        // Check that we have 4 QIS operations (RXY, RZZ, RZ, RXY)
        assert_eq!(module.body.blocks[0].operations.len(), 4);

        // Verify the sequence of operations
        let ops: Vec<_> = module.body.blocks[0].operations.iter()
            .filter_map(|instr| {
                if let Operation::Custom(custom_op) = &instr.operation {
                    Some(custom_op.name())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(ops, vec!["rxy", "rzz", "rz", "rxy"]);
    }
}