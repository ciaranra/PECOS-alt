/*!
Example of optimization passes using the boxing approach

This shows how the boxing approach enables sophisticated optimization
without complicating the core IR.
*/

use pecos_pmir::attributes::{AttributeBuilder, helpers};
use pecos_pmir::builtin_ops::{BuiltinOp, FuncOp};
use pecos_pmir::ops::Operation;
use pecos_pmir::pmir::{AttributeValue, Module, Region};
use pecos_pmir::region_kinds::RegionKind;
use pecos_pmir::types::{FunctionType, Type};
use std::collections::HashMap;

// Example-specific constants
mod tags {
    pub const QFT: &str = "qft";
    pub const SYNDROME_EXTRACTION: &str = "syndrome_extraction";
}

mod keys {
    pub const ERROR_MODEL: &str = "error_model";
}

/// Example optimization pass that recognizes and optimizes QFT patterns
struct QFTOptimizationPass;

impl QFTOptimizationPass {
    pub fn run(&self, module: &mut Module) {
        // Iterate through functions in the module's body
        if let Some(block) = module.body.blocks.first_mut() {
            for inst in &mut block.operations {
                if let Operation::Builtin(BuiltinOp::Func(function)) = &mut inst.operation {
                    for region in &mut function.body {
                        if helpers::has_tag(&region.attributes, tags::QFT) {
                            self.optimize_qft(region);
                        }
                    }
                }
            }
        }
    }

    fn optimize_qft(&self, region: &mut Region) {
        println!("Found QFT region to optimize!");

        // Check if we can use approximate QFT
        if let Some(AttributeValue::Int(n)) = region.attributes.get("num_qubits") {
            if *n > 10 {
                // For large QFTs, we can drop small angle rotations
                region.attributes.insert(
                    "optimization_applied".to_string(),
                    AttributeValue::String("approximate_qft".to_string()),
                );
                region.attributes.insert(
                    "approximation_threshold".to_string(),
                    AttributeValue::Float(0.01),
                );
            }
        }

        // Note: Actual rewriting would modify the operations
    }
}

/// Example pass for QEC-specific optimizations
struct QECOptimizationPass;

impl QECOptimizationPass {
    pub fn run(&self, module: &mut Module) {
        // First pass: identify syndrome extraction patterns
        let mut syndrome_regions = Vec::new();

        if let Some(block) = module.body.blocks.first() {
            for (f_idx, inst) in block.operations.iter().enumerate() {
                if let Operation::Builtin(BuiltinOp::Func(function)) = &inst.operation {
                    for (r_idx, region) in function.body.iter().enumerate() {
                        if helpers::has_tag(&region.attributes, tags::SYNDROME_EXTRACTION) {
                            syndrome_regions.push((f_idx, r_idx));
                        }
                    }
                }
            }
        }

        // Second pass: optimize based on patterns
        if let Some(block) = module.body.blocks.first_mut() {
            for (f_idx, r_idx) in syndrome_regions {
                if let Some(inst) = block.operations.get_mut(f_idx) {
                    if let Operation::Builtin(BuiltinOp::Func(function)) = &mut inst.operation {
                        if let Some(region) = function.body.get_mut(r_idx) {
                            self.optimize_syndrome_extraction(region);
                        }
                    }
                }
            }
        }
    }

    fn optimize_syndrome_extraction(&self, region: &mut Region) {
        println!("Optimizing syndrome extraction!");

        // Check error model to decide on optimization strategy
        if let Some(AttributeValue::Dict(error_model)) = region.attributes.get(keys::ERROR_MODEL) {
            if let Some(AttributeValue::Float(rate)) = error_model.get("measurement_error_rate") {
                if *rate < 0.001 {
                    // Low error rate - can use faster syndrome extraction
                    region.attributes.insert(
                        "optimization_applied".to_string(),
                        AttributeValue::String("fast_syndrome".to_string()),
                    );
                }
            }
        }
    }
}

/// Example analysis pass that doesn't modify, just collects information
struct ResourceEstimationPass;

impl ResourceEstimationPass {
    pub fn run(&self, module: &Module) -> ResourceEstimate {
        let mut estimate = ResourceEstimate::default();

        estimate.attributes.insert(
            keys::ERROR_MODEL.to_string(),
            AttributeValue::Dict(HashMap::new()),
        );

        if let Some(block) = module.body.blocks.first() {
            for inst in &block.operations {
                if let Operation::Builtin(BuiltinOp::Func(function)) = &inst.operation {
                    self.analyze_function(function, &mut estimate);
                }
            }
        }

        estimate
    }

    fn analyze_function(&self, function: &FuncOp, estimate: &mut ResourceEstimate) {
        for region in &function.body {
            if let Some(AttributeValue::Int(depth)) = region.attributes.get("circuit_depth") {
                estimate.total_depth += *depth as usize;
            }

            if helpers::has_tag(&region.attributes, tags::SYNDROME_EXTRACTION) {
                estimate.syndrome_rounds += 1;

                if let Some(AttributeValue::Int(n)) = region.attributes.get("num_stabilizers") {
                    // Estimate: each stabilizer needs n two-qubit gates
                    estimate.two_qubit_gates += (*n as usize) * (*n as usize - 1) / 2;
                }
            }
        }
    }
}

#[derive(Default)]
struct ResourceEstimate {
    total_depth: usize,
    two_qubit_gates: usize,
    syndrome_rounds: usize,
    attributes: HashMap<String, AttributeValue>,
}

fn main() {
    println!("=== PMIR Optimization Passes Example ===\n");

    // Create a module with boxed regions
    let mut module = create_example_module();

    println!("Original module:");
    println!("{}\n", module.to_mlir_text());

    // Run optimization passes
    let qft_pass = QFTOptimizationPass;
    qft_pass.run(&mut module);

    let qec_pass = QECOptimizationPass;
    qec_pass.run(&mut module);

    println!("\nOptimized module:");
    println!("{}\n", module.to_mlir_text());

    // Run analysis pass
    let resource_pass = ResourceEstimationPass;
    let estimate = resource_pass.run(&module);

    println!("\nResource estimates:");
    println!("  Total depth: {}", estimate.total_depth);
    println!("  Two-qubit gates: {}", estimate.two_qubit_gates);
    println!("  Syndrome rounds: {}", estimate.syndrome_rounds);
}

fn create_example_module() -> Module {
    let mut module = Module::new("qec_example");

    // Create a function with QFT region
    let mut qft_func = FuncOp::new(
        "quantum_fourier_transform",
        FunctionType {
            inputs: vec![Type::Array(
                Box::new(Type::Qubit),
                pecos_pmir::types::ArraySize::Fixed(16),
            )],
            outputs: vec![Type::Array(
                Box::new(Type::Qubit),
                pecos_pmir::types::ArraySize::Fixed(16),
            )],
            variadic: false,
        },
    );

    let mut qft_region = Region::new(RegionKind::SSACFG);
    qft_region.attributes = AttributeBuilder::new()
        .with_tag(tags::QFT)
        .with_attr("num_qubits", AttributeValue::Int(16))
        .with_attr("circuit_depth", AttributeValue::Int(120))
        .build();

    qft_func.body = vec![qft_region];
    module.add_function(qft_func);

    // Create a function with syndrome extraction
    let mut syndrome_func = FuncOp::new(
        "surface_code_syndrome",
        FunctionType {
            inputs: vec![Type::Array(
                Box::new(Type::Qubit),
                pecos_pmir::types::ArraySize::Dynamic,
            )],
            outputs: vec![Type::Array(
                Box::new(Type::Bit),
                pecos_pmir::types::ArraySize::Dynamic,
            )],
            variadic: false,
        },
    );

    let mut syndrome_region = Region::new(RegionKind::SSACFG);
    let mut error_model = HashMap::new();
    error_model.insert(
        "measurement_error_rate".to_string(),
        AttributeValue::Float(0.0005),
    );

    syndrome_region.attributes = AttributeBuilder::new()
        .with_tag(tags::SYNDROME_EXTRACTION)
        .with_attr("num_stabilizers", AttributeValue::Int(8))
        .with_attr("circuit_depth", AttributeValue::Int(4))
        .with_attr(keys::ERROR_MODEL, AttributeValue::Dict(error_model))
        .build();

    syndrome_func.body = vec![syndrome_region];
    module.add_function(syndrome_func);

    module
}
