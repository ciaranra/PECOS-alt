//! Example of using the "boxing" approach for quantum algorithms and patterns

use pecos_phir::{
    attributes::{AttributeBuilder, helpers},
    ops::{Operation, QuantumOp},
    phir::{Function, Instruction, Module, Region},
    region_kinds::RegionKind,
    types::{FunctionType, Type},
};

// Example-specific constants
mod tags {
    pub const QFT: &str = "qft";
    pub const SYNDROME_EXTRACTION: &str = "syndrome_extraction";
}

fn main() {
    // Example 1: Boxing a QFT circuit
    println!("=== Boxing Example: QFT Circuit ===\n");

    let _module = Module::new("qft_example");

    // Create a function with QFT
    let qft_signature = FunctionType {
        inputs: vec![Type::Array(
            Box::new(Type::Qubit),
            pecos_phir::types::ArraySize::Fixed(4),
        )],
        outputs: vec![Type::Array(
            Box::new(Type::Qubit),
            pecos_phir::types::ArraySize::Fixed(4),
        )],
        variadic: false,
    };
    let _qft_func = Function::new_with_visibility(
        "quantum_fourier_transform",
        qft_signature,
        pecos_phir::phir::Visibility::Public,
    );

    // Create a region for the QFT algorithm
    let mut qft_region = Region::new(RegionKind::SSACFG);

    // Box the region with QFT metadata
    qft_region.attributes = AttributeBuilder::new()
        .with_tag(tags::QFT)
        .with_algorithm("quantum_fourier_transform")
        .with_attr("num_qubits", pecos_phir::phir::AttributeValue::Int(4))
        .with_attr("circuit_depth", pecos_phir::phir::AttributeValue::Int(16))
        .parallelizable()
        .build();

    println!("QFT Region Attributes:");
    for (key, value) in &qft_region.attributes {
        println!("  {key}: {value:?}");
    }

    // Example 2: Boxing syndrome extraction for QEC experiments
    println!("\n=== Boxing Example: Syndrome Extraction ===\n");

    let mut syndrome_region = Region::new(RegionKind::SSACFG);

    // Box with syndrome extraction metadata
    syndrome_region.attributes = AttributeBuilder::new()
        .with_tag(tags::SYNDROME_EXTRACTION)
        .with_interface(
            vec![
                "data_qubits[5]".to_string(),
                "ancilla_qubits[4]".to_string(),
            ],
            vec!["syndrome_bits[4]".to_string()],
        )
        .with_attr(
            "stabilizer_type",
            pecos_phir::phir::AttributeValue::String("X".to_string()),
        )
        .with_attr(
            "measurement_order",
            pecos_phir::phir::AttributeValue::String("sequential".to_string()),
        )
        .build();

    println!("Syndrome Extraction Attributes:");
    for (key, value) in &syndrome_region.attributes {
        println!("  {key}: {value:?}");
    }

    // Example 3: Optimization pass can recognize patterns
    println!("\n=== Pattern Recognition Example ===\n");

    // Simulate an optimization pass checking regions
    let regions = vec![
        ("QFT Region", &qft_region),
        ("Syndrome Region", &syndrome_region),
    ];

    for (name, region) in regions {
        println!("Analyzing {name}");

        // Check semantic tags
        if helpers::has_tag(&region.attributes, tags::QFT) {
            println!("  ✓ Found QFT pattern - can apply QFT-specific optimizations");
            if helpers::is_parallelizable(&region.attributes) {
                println!("  ✓ Marked as parallelizable - can distribute phase rotations");
            }
        }

        if helpers::has_tag(&region.attributes, tags::SYNDROME_EXTRACTION) {
            println!("  ✓ Found syndrome extraction - can optimize for fault tolerance");
            if let Some(stab_type) = region.attributes.get("stabilizer_type") {
                println!("  ✓ Stabilizer type: {stab_type:?}");
            }
        }

        // Check interfaces
        if let Some(inputs) = region.attributes.get("input_interface") {
            println!("  → Inputs: {inputs:?}");
        }
        if let Some(outputs) = region.attributes.get("output_interface") {
            println!("  → Outputs: {outputs:?}");
        }
    }

    // Example 4: Boxing individual operations
    println!("\n=== Boxing Individual Operations ===\n");

    let mut magic_state_prep = Instruction::new(
        Operation::Quantum(QuantumOp::InitState(vec![])),
        vec![],
        vec![],
        vec![Type::Qubit],
    );

    // Tag it as magic state preparation
    magic_state_prep.attributes = AttributeBuilder::new()
        .with_tag("magic_state_preparation")
        .with_attr(
            "state_type",
            pecos_phir::phir::AttributeValue::String("T".to_string()),
        )
        .with_attr(
            "fidelity_required",
            pecos_phir::phir::AttributeValue::Float(0.999),
        )
        .build();

    println!("Magic State Preparation Attributes:");
    for (key, value) in &magic_state_prep.attributes {
        println!("  {key}: {value:?}");
    }

    // Example 5: Nested boxing - algorithms within algorithms
    println!("\n=== Nested Boxing Example ===\n");

    let mut shor_region = Region::new(RegionKind::SSACFG);
    shor_region.attributes = AttributeBuilder::new()
        .with_tag("shor_algorithm")
        .with_algorithm("integer_factorization")
        .with_attr("contains_qft", pecos_phir::phir::AttributeValue::Bool(true))
        .with_attr(
            "contains_modular_exp",
            pecos_phir::phir::AttributeValue::Bool(true),
        )
        .build();

    println!("Shor's Algorithm Region:");
    println!(
        "  - Contains QFT: {:?}",
        shor_region.attributes.get("contains_qft")
    );
    println!(
        "  - Contains Modular Exp: {:?}",
        shor_region.attributes.get("contains_modular_exp")
    );
    println!("\nOptimization passes can recognize nested patterns and optimize accordingly!");
}
