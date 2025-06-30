/*!
PMIR Generation from PAST

This module converts PAST (PECOS AST) to PMIR (PECOS Middle-level IR) expressed as
MLIR text format using standard MLIR dialects. The generated MLIR can then be
processed by MLIR tools (mlir-opt, mlir-translate) to produce LLVM IR.
*/

use pecos_core::errors::PecosError;
use std::fmt;

use crate::PmirConfig;
use crate::ast::{PastFunction, PastGraph, PastModule, PastNode, PastOp, PastType, PastValue};

/// MLIR Module representation for text generation
pub struct MlirModule {
    /// Module name
    pub name: String,
    /// Functions in the module
    pub functions: Vec<MlirFunction>,
    /// External function declarations
    pub external_funcs: Vec<ExternalFunc>,
}

/// External function declaration
pub struct ExternalFunc {
    pub name: String,
    pub return_type: Option<String>,
    pub arg_types: Vec<String>,
}

/// MLIR Function
pub struct MlirFunction {
    /// Function name
    pub name: String,
    /// Function signature
    pub signature: String,
    /// Basic blocks
    pub blocks: Vec<MlirBlock>,
}

/// MLIR Basic Block
pub struct MlirBlock {
    /// Block label
    pub label: String,
    /// Operations in the block
    pub operations: Vec<MlirOperation>,
    /// Terminator operation
    pub terminator: MlirOperation,
}

/// MLIR Operation
pub struct MlirOperation {
    /// Result values (if any)
    pub results: Vec<String>,
    /// Operation name
    pub op_name: String,
    /// Operation arguments
    pub args: Vec<String>,
    /// Attributes
    pub attrs: Vec<(String, String)>,
}

impl fmt::Display for MlirModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write external function declarations first
        for ext_func in &self.external_funcs {
            write!(f, "func private @{}(", ext_func.name)?;
            write!(f, "{}", ext_func.arg_types.join(", "))?;
            write!(f, ")")?;
            if let Some(ret_ty) = &ext_func.return_type {
                write!(f, " -> {ret_ty}")?;
            }
            writeln!(f)?;
        }

        if !self.external_funcs.is_empty() {
            writeln!(f)?;
        }

        // Write module functions
        for func in &self.functions {
            write!(f, "{func}")?;
        }
        Ok(())
    }
}

impl fmt::Display for MlirFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "func {} {{", self.signature)?;
        for block in &self.blocks {
            write!(f, "{block}")?;
        }
        writeln!(f, "}}")
    }
}

impl fmt::Display for MlirBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.label.is_empty() {
            writeln!(f, "^{}:", self.label)?;
        }
        for op in &self.operations {
            if !op.op_name.starts_with("//") {
                // Skip comments
                writeln!(f, "  {op}")?;
            }
        }
        writeln!(f, "  {}", self.terminator)
    }
}

impl fmt::Display for MlirOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.results.is_empty() {
            write!(f, "{} = ", self.results.join(", "))?;
        }

        // Special handling for call and return operations
        if self.op_name == "call" && !self.args.is_empty() {
            // Extract function name and arguments
            let first_arg = &self.args[0];
            write!(f, "call {first_arg}")?;
        } else if self.op_name == "return" {
            write!(f, "return")?;
            if !self.args.is_empty() {
                write!(f, " {}", self.args.join(", "))?;
            }
        } else {
            write!(f, "{}", self.op_name)?;

            if !self.args.is_empty() {
                write!(f, "({})", self.args.join(", "))?;
            }
        }

        // Add type annotation if present (skip for return operations)
        if self.op_name != "return" {
            if let Some((_, ty)) = self.attrs.iter().find(|(k, _)| k == "type") {
                write!(f, " : {ty}")?;
            }
        } else if !self.args.is_empty() {
            // For return, we need to add the type based on the returned values
            if self.args.len() == 1 {
                write!(f, " : i1")?;
            } else {
                // Multiple return values - each is i1 for now
                let types = vec!["i1"; self.args.len()].join(", ");
                write!(f, " : {types}")?;
            }
        }

        Ok(())
    }
}

/// Lower PAST to PMIR (PECOS Middle-level IR) expressed as MLIR
pub fn lower_past_to_pmir(
    past: &PastModule,
    config: &PmirConfig,
) -> Result<MlirModule, PecosError> {
    let mut mlir_functions = Vec::new();
    let external_funcs = collect_external_functions();

    for func in &past.functions {
        let mlir_func = lower_function(func, config)?;
        mlir_functions.push(mlir_func);
    }

    Ok(MlirModule {
        name: past.name.clone(),
        functions: mlir_functions,
        external_funcs,
    })
}

/// Collect all QIR external function declarations
fn collect_external_functions() -> Vec<ExternalFunc> {
    vec![
        // Qubit allocation/deallocation
        ExternalFunc {
            name: "__quantum__rt__qubit_allocate".to_string(),
            return_type: Some("!llvm.ptr<i8>".to_string()),
            arg_types: vec![],
        },
        ExternalFunc {
            name: "__quantum__rt__qubit_release".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        // Single qubit gates
        ExternalFunc {
            name: "__quantum__qis__h__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__x__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__y__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__z__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__s__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__t__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__sadj__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__tadj__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
        // Two qubit gates
        ExternalFunc {
            name: "__quantum__qis__cnot__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__cz__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__cy__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__ch__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        // Three qubit gates
        ExternalFunc {
            name: "__quantum__qis__ccx__body".to_string(),
            return_type: None,
            arg_types: vec![
                "!llvm.ptr<i8>".to_string(),
                "!llvm.ptr<i8>".to_string(),
                "!llvm.ptr<i8>".to_string(),
            ],
        },
        // Rotation gates
        ExternalFunc {
            name: "__quantum__qis__rx__body".to_string(),
            return_type: None,
            arg_types: vec!["f64".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__ry__body".to_string(),
            return_type: None,
            arg_types: vec!["f64".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__rz__body".to_string(),
            return_type: None,
            arg_types: vec!["f64".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__crz__body".to_string(),
            return_type: None,
            arg_types: vec![
                "f64".to_string(),
                "!llvm.ptr<i8>".to_string(),
                "!llvm.ptr<i8>".to_string(),
            ],
        },
        // Measurement
        ExternalFunc {
            name: "__quantum__rt__result_get_zero".to_string(),
            return_type: Some("!llvm.ptr<i8>".to_string()),
            arg_types: vec![],
        },
        ExternalFunc {
            name: "__quantum__qis__mz__body".to_string(),
            return_type: None,
            arg_types: vec!["!llvm.ptr<i8>".to_string(), "!llvm.ptr<i8>".to_string()],
        },
        ExternalFunc {
            name: "__quantum__qis__read_result__body".to_string(),
            return_type: Some("i1".to_string()),
            arg_types: vec!["!llvm.ptr<i8>".to_string()],
        },
    ]
}

/// Convert PAST type to MLIR type string
fn type_to_mlir(ty: &PastType) -> String {
    match ty {
        PastType::Qubit => "!llvm.ptr<i8>".to_string(), // Opaque pointer for Qubit*
        PastType::Bit => "i1".to_string(),
        PastType::Int(width) => format!("i{width}"),
        PastType::Float(width) => format!("f{width}"),
        PastType::Array(elem, size) => format!("!llvm.array<{} x {}>", size, type_to_mlir(elem)),
        PastType::Tuple(types) => {
            let inner = types
                .iter()
                .map(type_to_mlir)
                .collect::<Vec<_>>()
                .join(", ");
            format!("!llvm.struct<({inner})>")
        }
        PastType::Custom(_) => "!llvm.ptr<i8>".to_string(), // Default to opaque pointer
    }
}

/// Lower a single function
fn lower_function(func: &PastFunction, _config: &PmirConfig) -> Result<MlirFunction, PecosError> {
    // Build function signature
    let input_types = func
        .inputs
        .iter()
        .map(|p| type_to_mlir(&p.ty))
        .collect::<Vec<_>>()
        .join(", ");

    let output_types = func
        .outputs
        .iter()
        .map(type_to_mlir)
        .collect::<Vec<_>>()
        .join(", ");

    let signature = if func.outputs.is_empty() {
        format!("@{}({})", func.name, input_types)
    } else if func.outputs.len() == 1 {
        format!("@{}({}) -> {}", func.name, input_types, output_types)
    } else {
        // For multiple outputs, we need to return them as separate values, not a tuple
        // MLIR's func dialect doesn't use parentheses for multiple returns
        format!("@{}({}) -> ({})", func.name, input_types, output_types)
    };

    // Lower the function body
    let blocks = lower_graph(&func.body)?;

    Ok(MlirFunction {
        name: func.name.clone(),
        signature,
        blocks,
    })
}

/// Lower a computation graph to basic blocks
fn lower_graph(graph: &PastGraph) -> Result<Vec<MlirBlock>, PecosError> {
    let mut blocks = Vec::new();
    let mut operations = Vec::new();
    let mut value_map = std::collections::HashMap::new();
    let mut allocated_qubits = Vec::new();

    // Build edge connectivity map: (dst_node, dst_port) -> (src_node, src_port)
    let mut edge_map = std::collections::HashMap::new();
    for edge in &graph.edges {
        edge_map.insert((edge.dst, edge.dst_port), (edge.src, edge.src_port));
    }

    // Process nodes in topological order (simplified for now)
    for node in &graph.nodes {
        let mlir_ops =
            lower_node_to_operations(node, &value_map, &edge_map, &mut allocated_qubits)?;

        // For quantum gates that operate in-place, we need to track the qubit flow
        match &node.op {
            PastOp::H
            | PastOp::X
            | PastOp::Y
            | PastOp::Z
            | PastOp::S
            | PastOp::T
            | PastOp::Sdg
            | PastOp::Tdg => {
                // These gates operate in-place, so output qubit is same as input
                if let Some(&(src_node, src_port)) = edge_map.get(&(node.id, 0)) {
                    if let Some(val) = value_map.get(&(src_node, src_port)) {
                        value_map.insert((node.id, 0), val.clone());
                    }
                }
            }
            PastOp::CX | PastOp::CZ | PastOp::CY | PastOp::CH => {
                // Two-qubit gates pass through both qubits
                for i in 0..2 {
                    if let Some(&(src_node, src_port)) = edge_map.get(&(node.id, i)) {
                        if let Some(val) = value_map.get(&(src_node, src_port)) {
                            value_map.insert((node.id, i), val.clone());
                        }
                    }
                }
            }
            PastOp::Toffoli => {
                // Three-qubit gate passes through all qubits
                for i in 0..3 {
                    if let Some(&(src_node, src_port)) = edge_map.get(&(node.id, i)) {
                        if let Some(val) = value_map.get(&(src_node, src_port)) {
                            value_map.insert((node.id, i), val.clone());
                        }
                    }
                }
            }
            _ => {
                // Track SSA values produced by these operations
                for mlir_op in &mlir_ops {
                    for (i, result) in mlir_op.results.iter().enumerate() {
                        value_map.insert((node.id, i), result.clone());
                    }
                }
            }
        }

        operations.extend(mlir_ops);
    }

    // Add cleanup operations to release qubits
    for qubit in &allocated_qubits {
        operations.push(MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__rt__qubit_release({})", qubit)],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        });
    }

    // Find the final output value for return
    let mut return_args = vec![];
    for exit_node in &graph.exits {
        // Find edges that connect to output nodes
        for edge in &graph.edges {
            if edge.dst == *exit_node {
                if let Some(src_val) = value_map.get(&(edge.src, edge.src_port)) {
                    return_args.push(src_val.clone());
                }
            }
        }
    }

    // Add return terminator
    let terminator = MlirOperation {
        results: vec![],
        op_name: "return".to_string(),
        args: return_args,
        attrs: vec![],
    };

    blocks.push(MlirBlock {
        label: String::new(), // Entry block has no label
        operations,
        terminator,
    });

    Ok(blocks)
}

/// Lower a single node to MLIR operations (may generate multiple ops)
fn lower_node_to_operations(
    node: &PastNode,
    value_map: &std::collections::HashMap<(usize, usize), String>,
    edge_map: &std::collections::HashMap<(usize, usize), (usize, usize)>,
    allocated_qubits: &mut Vec<String>,
) -> Result<Vec<MlirOperation>, PecosError> {
    // Helper to get input argument names
    let get_input_arg = |port: usize| -> String {
        if let Some(&(src_node, src_port)) = edge_map.get(&(node.id, port)) {
            if let Some(val) = value_map.get(&(src_node, src_port)) {
                return val.clone();
            }
        }
        format!("%input_{}_{}", node.id, port)
    };

    match &node.op {
        // Quantum operations using func.call
        PastOp::H => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__h__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::X => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__x__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::Y => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__y__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::Z => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__z__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::S => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__s__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::T => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__t__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::Sdg => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__sadj__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::Tdg => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__tadj__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::CX => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__cnot__body({}, {})",
                get_input_arg(0),
                get_input_arg(1)
            )],
            attrs: vec![(
                "type".to_string(),
                "(!llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
            )],
        }]),

        PastOp::CZ => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__cz__body({}, {})",
                get_input_arg(0),
                get_input_arg(1)
            )],
            attrs: vec![(
                "type".to_string(),
                "(!llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
            )],
        }]),

        PastOp::CY => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__cy__body({}, {})",
                get_input_arg(0),
                get_input_arg(1)
            )],
            attrs: vec![(
                "type".to_string(),
                "(!llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
            )],
        }]),

        PastOp::CH => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__ch__body({}, {})",
                get_input_arg(0),
                get_input_arg(1)
            )],
            attrs: vec![(
                "type".to_string(),
                "(!llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
            )],
        }]),

        PastOp::Toffoli => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__ccx__body({}, {}, {})",
                get_input_arg(0),
                get_input_arg(1),
                get_input_arg(2)
            )],
            attrs: vec![(
                "type".to_string(),
                "(!llvm.ptr<i8>, !llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
            )],
        }]),

        // Rotation gates
        PastOp::RX(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__rx__body({}, {})",
                angle,
                get_input_arg(0)
            )],
            attrs: vec![("type".to_string(), "(f64, !llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::RY(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__ry__body({}, {})",
                angle,
                get_input_arg(0)
            )],
            attrs: vec![("type".to_string(), "(f64, !llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::RZ(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__rz__body({}, {})",
                angle,
                get_input_arg(0)
            )],
            attrs: vec![("type".to_string(), "(f64, !llvm.ptr<i8>) -> ()".to_string())],
        }]),

        PastOp::CRZ(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__crz__body({}, {}, {})",
                angle,
                get_input_arg(0),
                get_input_arg(1)
            )],
            attrs: vec![(
                "type".to_string(),
                "(f64, !llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
            )],
        }]),

        PastOp::Measure => {
            // Need to allocate result, call measure, then read result
            let result_ptr = format!("%result_{}", node.id);
            let bit_result = format!("%{}", node.id);
            let qubit_input = get_input_arg(0);

            // Create a block of operations
            let alloc_result = MlirOperation {
                results: vec![result_ptr.clone()],
                op_name: "call".to_string(),
                args: vec!["@__quantum__rt__result_get_zero()".to_string()],
                attrs: vec![("type".to_string(), "() -> !llvm.ptr<i8>".to_string())],
            };

            let measure = MlirOperation {
                results: vec![],
                op_name: "call".to_string(),
                args: vec![format!(
                    "@__quantum__qis__mz__body({}, {})",
                    qubit_input, result_ptr
                )],
                attrs: vec![(
                    "type".to_string(),
                    "(!llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
                )],
            };

            let read_result = MlirOperation {
                results: vec![bit_result],
                op_name: "call".to_string(),
                args: vec![format!(
                    "@__quantum__qis__read_result__body({})",
                    result_ptr
                )],
                attrs: vec![("type".to_string(), "(!llvm.ptr<i8>) -> i1".to_string())],
            };

            // Return all three operations for measurement
            Ok(vec![alloc_result, measure, read_result])
        }

        PastOp::AllocQubit | PastOp::QAlloc => {
            let qubit_var = format!("%{}", node.id);
            allocated_qubits.push(qubit_var.clone());

            Ok(vec![MlirOperation {
                results: vec![qubit_var],
                op_name: "call".to_string(),
                args: vec!["@__quantum__rt__qubit_allocate()".to_string()],
                attrs: vec![("type".to_string(), "() -> !llvm.ptr<i8>".to_string())],
            }])
        }

        // Classical operations using arith dialect
        PastOp::Add => Ok(vec![MlirOperation {
            results: vec![format!("%{}", node.id)],
            op_name: "arith.addi".to_string(),
            args: vec![get_input_arg(0), get_input_arg(1)],
            attrs: vec![("type".to_string(), "i64, i64".to_string())],
        }]),

        PastOp::Const(value) => {
            let (val_str, ty_str) = match value {
                PastValue::Bool(b) => (if *b { "1" } else { "0" }.to_string(), "i1".to_string()),
                PastValue::Int(i) => (i.to_string(), "i64".to_string()),
                PastValue::Float(f) => (f.to_string(), "f64".to_string()),
                PastValue::String(s) => (format!("\"{s}\""), "!llvm.ptr<i8>".to_string()),
            };
            Ok(vec![MlirOperation {
                results: vec![format!("%{}", node.id)],
                op_name: "arith.constant".to_string(),
                args: vec![val_str],
                attrs: vec![("type".to_string(), ty_str)],
            }])
        }

        // Input/Output nodes
        PastOp::Input(idx) => Ok(vec![MlirOperation {
            results: vec![format!("%arg{}", idx)],
            op_name: "// input".to_string(),
            args: vec![],
            attrs: vec![],
        }]),

        PastOp::Output(idx) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: format!("// output {idx}"),
            args: vec![],
            attrs: vec![],
        }]),

        _ => Err(PecosError::CompileInvalidOperation {
            operation: format!("{:?}", node.op),
            reason: "Unsupported operation for MLIR lowering".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlir_display() {
        let module = MlirModule {
            name: "test".to_string(),
            external_funcs: vec![],
            functions: vec![MlirFunction {
                name: "main".to_string(),
                signature: "@main() -> i1".to_string(),
                blocks: vec![MlirBlock {
                    label: String::new(),
                    operations: vec![MlirOperation {
                        results: vec!["%0".to_string()],
                        op_name: "call".to_string(),
                        args: vec!["@__quantum__rt__qubit_allocate()".to_string()],
                        attrs: vec![],
                    }],
                    terminator: MlirOperation {
                        results: vec![],
                        op_name: "return".to_string(),
                        args: vec!["%0".to_string()],
                        attrs: vec![],
                    },
                }],
            }],
        };

        let mlir_str = module.to_string();
        // The Display implementation generates "call" not "func.call"
        assert!(mlir_str.contains("call @__quantum__rt__qubit_allocate"));
        // The Display implementation generates "return" not "func.return"
        assert!(mlir_str.contains("return %0"));
    }
}
