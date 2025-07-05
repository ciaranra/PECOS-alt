//! Example showing how MLIR's native structure supports the boxing/protocol approach
//!
//! This demonstrates that we don't need special constructs - MLIR's hierarchy
//! naturally provides the boxing we need:
//!
//! - Modules = Top-level containers/libraries
//! - Functions = Reusable protocols/macros
//! - Regions = Isolated scopes with clear interfaces
//! - Blocks = Basic protocol steps that can be composed
//! - Operations = Atomic actions

use pecos_pmir::{
    attributes::AttributeBuilder,
    ops::{ControlFlowOp, FunctionCall, Operation},
    pmir::{Block, BlockRef, Function, Instruction, Module, Region, Terminator},
    region_kinds::RegionKind,
    types::{FunctionType, Type},
};

fn main() {
    println!("=== MLIR Native Boxing Example ===\n");

    // Build a QEC protocol library using MLIR's natural structure
    let qec_library = build_qec_protocol_library();

    // Show the MLIR text representation
    println!("MLIR Representation of QEC Protocol Library:");
    println!("{}", qec_library.to_mlir_text());

    // Example: Compose protocols by calling functions
    let surface_code_cycle = build_surface_code_cycle();
    println!("\n=== Surface Code Cycle (Composed from Protocols) ===");
    println!("{}", surface_code_cycle.to_mlir_text());

    // Demonstrate region-based isolation
    demonstrate_region_isolation();
}

/// Build a library of QEC protocols as MLIR functions
fn build_qec_protocol_library() -> Module {
    let mut module = Module::new("qec_protocols");

    // Protocol 1: X-type syndrome extraction (as a function)
    module.add_function(create_x_syndrome_protocol());

    // Protocol 2: Z-type syndrome extraction (as a function)
    module.add_function(create_z_syndrome_protocol());

    // Protocol 3: Decoder protocol
    module.add_function(create_decoder_protocol());

    // Protocol 4: Correction application
    module.add_function(create_correction_protocol());

    module
}

/// X-type syndrome extraction as a reusable function/protocol
fn create_x_syndrome_protocol() -> Function {
    let signature = FunctionType {
        inputs: vec![
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic), // data qubits
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic), // ancilla qubits
        ],
        outputs: vec![
            Type::Array(Box::new(Type::Bit), pecos_pmir::types::ArraySize::Dynamic), // syndrome bits
        ],
        variadic: false,
    };

    let mut func = Function::new_with_visibility(
        "x_syndrome_extraction",
        signature,
        pecos_pmir::pmir::Visibility::Public,
    );

    // Tag the function as a protocol
    func.attributes = AttributeBuilder::new()
        .with_tag("qec_protocol")
        .with_attr(
            "syndrome_type",
            pecos_pmir::pmir::AttributeValue::String("X".to_string()),
        )
        .with_attr(
            "protocol_type",
            pecos_pmir::pmir::AttributeValue::String("syndrome_extraction".to_string()),
        )
        .build();

    // Create the protocol implementation
    let mut region = Region::new(RegionKind::SSACFG);

    // Block 1: Initialize ancillas
    let mut init_block = Block::new(Some("init_ancillas".to_string()));
    init_block.attributes.insert(
        "protocol_step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("ancilla_preparation".to_string()),
    );
    // In real implementation, would have reset operations here

    // Block 2: Entangling gates
    let mut entangle_block = Block::new(Some("entangle".to_string()));
    entangle_block.attributes.insert(
        "protocol_step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("stabilizer_entangling".to_string()),
    );
    entangle_block.attributes.insert(
        "can_parallelize".to_string(),
        pecos_pmir::pmir::AttributeValue::Bool(true),
    );

    // Block 3: Measure ancillas
    let mut measure_block = Block::new(Some("measure".to_string()));
    measure_block.attributes.insert(
        "protocol_step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("ancilla_measurement".to_string()),
    );

    // Set up control flow
    init_block.set_terminator(Terminator::Branch {
        target: BlockRef::by_label("entangle"),
        args: vec![],
    });
    entangle_block.set_terminator(Terminator::Branch {
        target: BlockRef::by_label("measure"),
        args: vec![],
    });
    measure_block.set_terminator(Terminator::Return { values: vec![] });

    region.add_block(init_block);
    region.add_block(entangle_block);
    region.add_block(measure_block);

    func.body.push(region);
    func
}

/// Z-type syndrome extraction protocol
fn create_z_syndrome_protocol() -> Function {
    let signature = FunctionType {
        inputs: vec![
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic),
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic),
        ],
        outputs: vec![Type::Array(
            Box::new(Type::Bit),
            pecos_pmir::types::ArraySize::Dynamic,
        )],
        variadic: false,
    };

    let mut func = Function::new_with_visibility(
        "z_syndrome_extraction",
        signature,
        pecos_pmir::pmir::Visibility::Public,
    );

    func.attributes = AttributeBuilder::new()
        .with_tag("qec_protocol")
        .with_attr(
            "syndrome_type",
            pecos_pmir::pmir::AttributeValue::String("Z".to_string()),
        )
        .with_attr(
            "protocol_type",
            pecos_pmir::pmir::AttributeValue::String("syndrome_extraction".to_string()),
        )
        .build();

    // Similar structure but different gate patterns
    let mut region = Region::new(RegionKind::SSACFG);
    region.add_block(Block::new(Some("z_protocol".to_string())));
    func.body.push(region);
    func
}

/// Decoder protocol - takes syndrome and returns correction
fn create_decoder_protocol() -> Function {
    let signature = FunctionType {
        inputs: vec![
            Type::Array(Box::new(Type::Bit), pecos_pmir::types::ArraySize::Dynamic), // X syndrome
            Type::Array(Box::new(Type::Bit), pecos_pmir::types::ArraySize::Dynamic), // Z syndrome
        ],
        outputs: vec![
            Type::Array(Box::new(Type::Bit), pecos_pmir::types::ArraySize::Dynamic), // corrections
        ],
        variadic: false,
    };

    let mut func = Function::new_with_visibility(
        "decode_syndrome",
        signature,
        pecos_pmir::pmir::Visibility::Public,
    );

    func.attributes = AttributeBuilder::new()
        .with_tag("qec_protocol")
        .with_attr(
            "protocol_type",
            pecos_pmir::pmir::AttributeValue::String("decoder".to_string()),
        )
        .with_attr(
            "decoder_type",
            pecos_pmir::pmir::AttributeValue::String("MWPM".to_string()),
        )
        .build();

    let region = Region::new(RegionKind::SSACFG);
    func.body.push(region);
    func
}

/// Correction application protocol
fn create_correction_protocol() -> Function {
    let signature = FunctionType {
        inputs: vec![
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic), // data qubits
            Type::Array(Box::new(Type::Bit), pecos_pmir::types::ArraySize::Dynamic), // corrections
        ],
        outputs: vec![],
        variadic: false,
    };

    let mut func = Function::new_with_visibility(
        "apply_corrections",
        signature,
        pecos_pmir::pmir::Visibility::Public,
    );

    func.attributes = AttributeBuilder::new()
        .with_tag("qec_protocol")
        .with_attr(
            "protocol_type",
            pecos_pmir::pmir::AttributeValue::String("correction".to_string()),
        )
        .build();

    let region = Region::new(RegionKind::SSACFG);
    func.body.push(region);
    func
}

/// Build a complete surface code cycle by composing protocols
fn build_surface_code_cycle() -> Module {
    let mut module = Module::new("surface_code_cycle");

    let signature = FunctionType {
        inputs: vec![
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic), // data
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic), // X ancillas
            Type::Array(Box::new(Type::Qubit), pecos_pmir::types::ArraySize::Dynamic), // Z ancillas
        ],
        outputs: vec![],
        variadic: false,
    };

    let mut cycle_func = Function::new_with_visibility(
        "surface_code_cycle",
        signature,
        pecos_pmir::pmir::Visibility::Public,
    );

    // Tag as a composite protocol
    cycle_func.attributes = AttributeBuilder::new()
        .with_tag("composite_protocol")
        .with_attr(
            "error_correction_code",
            pecos_pmir::pmir::AttributeValue::String("surface_code".to_string()),
        )
        .build();

    let mut region = Region::new(RegionKind::SSACFG);
    let mut main_block = Block::new(None);

    // Compose the cycle from protocol calls
    // This is like assembly macros - each call expands to the full protocol

    // Step 1: Extract X syndrome
    let mut x_syndrome_call = Instruction::new(
        Operation::ControlFlow(ControlFlowOp::Call(FunctionCall {
            name: "x_syndrome_extraction".to_string(),
            args: vec![], // Would have actual SSA values
        })),
        vec![],
        vec![],
        vec![Type::Array(
            Box::new(Type::Bit),
            pecos_pmir::types::ArraySize::Dynamic,
        )],
    );
    x_syndrome_call.attributes.insert(
        "step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("extract_x_syndrome".to_string()),
    );

    // Step 2: Extract Z syndrome
    let mut z_syndrome_call = Instruction::new(
        Operation::ControlFlow(ControlFlowOp::Call(FunctionCall {
            name: "z_syndrome_extraction".to_string(),
            args: vec![],
        })),
        vec![],
        vec![],
        vec![Type::Array(
            Box::new(Type::Bit),
            pecos_pmir::types::ArraySize::Dynamic,
        )],
    );
    z_syndrome_call.attributes.insert(
        "step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("extract_z_syndrome".to_string()),
    );

    // Step 3: Decode
    let mut decode_call = Instruction::new(
        Operation::ControlFlow(ControlFlowOp::Call(FunctionCall {
            name: "decode_syndrome".to_string(),
            args: vec![],
        })),
        vec![],
        vec![],
        vec![Type::Array(
            Box::new(Type::Bit),
            pecos_pmir::types::ArraySize::Dynamic,
        )],
    );
    decode_call.attributes.insert(
        "step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("decode_syndrome".to_string()),
    );

    // Step 4: Apply corrections
    let mut correct_call = Instruction::new(
        Operation::ControlFlow(ControlFlowOp::Call(FunctionCall {
            name: "apply_corrections".to_string(),
            args: vec![],
        })),
        vec![],
        vec![],
        vec![],
    );
    correct_call.attributes.insert(
        "step".to_string(),
        pecos_pmir::pmir::AttributeValue::String("apply_corrections".to_string()),
    );

    main_block.add_instruction(x_syndrome_call);
    main_block.add_instruction(z_syndrome_call);
    main_block.add_instruction(decode_call);
    main_block.add_instruction(correct_call);
    main_block.set_terminator(Terminator::Return { values: vec![] });

    region.add_block(main_block);
    cycle_func.body.push(region);
    module.add_function(cycle_func);

    module
}

/// Example showing how regions provide natural isolation
fn demonstrate_region_isolation() {
    println!("\n=== Region-Based Protocol Isolation ===\n");

    let mut func = Function::new_with_visibility(
        "multi_protocol_function",
        FunctionType {
            inputs: vec![],
            outputs: vec![],
            variadic: false,
        },
        pecos_pmir::pmir::Visibility::Public,
    );

    // Region 1: State preparation protocol
    let mut prep_region = Region::new(RegionKind::SSACFG);
    prep_region.attributes = AttributeBuilder::new()
        .with_tag("state_preparation")
        .with_attr(
            "target_state",
            pecos_pmir::pmir::AttributeValue::String("GHZ".to_string()),
        )
        .build();

    // Region 2: Measurement protocol
    let mut measure_region = Region::new(RegionKind::SSACFG);
    measure_region.attributes = AttributeBuilder::new()
        .with_tag("measurement_protocol")
        .with_attr(
            "basis",
            pecos_pmir::pmir::AttributeValue::String("Bell".to_string()),
        )
        .build();

    func.body.push(prep_region);
    func.body.push(measure_region);

    println!("Regions provide natural protocol isolation:");
    println!("- Each region has its own scope");
    println!("- Clear interfaces through SSA values");
    println!("- Can be optimized independently");
    println!("- Tagged with protocol metadata");
}
