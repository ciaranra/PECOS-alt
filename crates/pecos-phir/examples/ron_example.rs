use pecos_phir::{
    builtin_ops::{FuncOp, ModuleOp, BuiltinOp},
    ops::{Operation, QuantumOp, SSAValue},
    phir::{Block, Instruction, Region, Terminator},
    region_kinds::RegionKind,
    types::{qubit_type, bit_type, FunctionType},
    ModuleRonExt,
};

fn main() {
    // Create a quantum module
    let mut module = ModuleOp::new("bell_state_module");
    
    // Create a function
    let signature = FunctionType {
        inputs: vec![],
        outputs: vec![bit_type(), bit_type()],
        variadic: false,
    };
    
    let mut func = FuncOp::new("create_bell_state", signature);
    
    // Add a region with a block
    let mut region = Region::new(RegionKind::Graph);
    let mut block = Block::new(None);
    
    // Allocate two qubits
    let alloc1 = Instruction::new(
        Operation::Quantum(QuantumOp::Alloc),
        vec![],
        vec![SSAValue::new(0)],
        vec![qubit_type()],
    );
    block.add_instruction(alloc1);
    
    let alloc2 = Instruction::new(
        Operation::Quantum(QuantumOp::Alloc),
        vec![],
        vec![SSAValue::new(1)],
        vec![qubit_type()],
    );
    block.add_instruction(alloc2);
    
    // Apply H gate to first qubit
    let h_gate = Instruction::new(
        Operation::Quantum(QuantumOp::H),
        vec![SSAValue::new(0)],
        vec![SSAValue::new(2)],
        vec![qubit_type()],
    );
    block.add_instruction(h_gate);
    
    // Apply CNOT gate
    let cnot = Instruction::new(
        Operation::Quantum(QuantumOp::CX),
        vec![SSAValue::new(2), SSAValue::new(1)],
        vec![SSAValue::new(3), SSAValue::new(4)],
        vec![qubit_type(), qubit_type()],
    );
    block.add_instruction(cnot);
    
    // Measure both qubits
    let measure1 = Instruction::new(
        Operation::Quantum(QuantumOp::Measure),
        vec![SSAValue::new(3)],
        vec![SSAValue::new(5)],
        vec![bit_type()],
    );
    block.add_instruction(measure1);
    
    let measure2 = Instruction::new(
        Operation::Quantum(QuantumOp::Measure),
        vec![SSAValue::new(4)],
        vec![SSAValue::new(6)],
        vec![bit_type()],
    );
    block.add_instruction(measure2);
    
    // Add return terminator
    block.set_terminator(Terminator::Return {
        values: vec![SSAValue::new(5), SSAValue::new(6)],
    });
    
    region.add_block(block);
    func.body.push(region);
    
    // Add function to module
    let func_inst = Instruction::new(
        Operation::Builtin(BuiltinOp::Func(func)),
        vec![],
        vec![],
        vec![],
    );
    module.add_operation(func_inst);
    
    // Convert to RON and print
    match module.to_ron() {
        Ok(ron_string) => {
            println!("PHIR Module in RON format:\n");
            println!("{}", ron_string);
        }
        Err(e) => eprintln!("Failed to serialize to RON: {}", e),
    }
}