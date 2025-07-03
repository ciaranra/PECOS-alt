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
    /// Global string constants for result names
    pub global_strings: Vec<GlobalString>,
}

/// Global string constant declaration
pub struct GlobalString {
    pub name: String,
    pub value: String,
    pub length: usize,
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
        // Write global string constants first (MLIR format)
        for global_str in &self.global_strings {
            writeln!(f, "llvm.mlir.global internal constant @{}(\"{}\\00\") : !llvm.array<{} x i8>", 
                     global_str.name, global_str.value, global_str.length)?;
        }
        
        if !self.global_strings.is_empty() {
            writeln!(f)?; // Add blank line after globals
        }
        
        // Write external function declarations
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

        // Special handling for call, return, and LLVM dialect operations
        if self.op_name == "call" && !self.args.is_empty() {
            // Extract function name and arguments
            let first_arg = &self.args[0];
            write!(f, "call {first_arg}")?;
        } else if self.op_name == "return" {
            write!(f, "return")?;
            if !self.args.is_empty() {
                write!(f, " {}", self.args.join(", "))?;
            }
        } else if self.op_name.starts_with("llvm.") {
            // Special handling for LLVM dialect operations
            write!(f, "{}", self.op_name)?;
            if !self.args.is_empty() {
                write!(f, " {}", self.args.join(", "))?;
            }
        } else if self.op_name.starts_with("arith.") {
            // Special handling for arith dialect operations - no parentheses
            write!(f, "{}", self.op_name)?;
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
                write!(f, " : i32")?; // HUGR convention: measurements return i32
            } else {
                // Multiple return values - each is i32 for measurements
                let types = vec!["i32"; self.args.len()].join(", ");
                write!(f, " : {types}")?;
            }
        }

        Ok(())
    }
}

/// Lower PAST to PMIR (PECOS Middle-level IR) expressed as MLIR
///
/// # Errors
///
/// Returns `PecosError` if:
/// - No entry point function is found
/// - Function lowering fails
pub fn lower_past_to_pmir(
    past: &PastModule,
    config: &PmirConfig,
) -> Result<MlirModule, PecosError> {
    let mut mlir_functions = Vec::new();
    let external_funcs = collect_external_functions();

    // Extract result names from the PAST (parsed from HUGR)
    let mut result_names = std::collections::HashSet::new();
    for func in &past.functions {
        extract_result_names_from_function(func, &mut result_names);
    }

    for func in &past.functions {
        let mlir_func = lower_function(func, config)?;
        mlir_functions.push(mlir_func);
    }

    // Create global string constants for result recording based on extracted names
    let mut global_strings = Vec::new();
    for result_name in &result_names {
        global_strings.push(GlobalString {
            name: format!("str_{}", result_name),
            value: result_name.clone(),
            length: result_name.len() + 1, // +1 for null terminator
        });
    }

    // Add default result strings based on the number of outputs
    if global_strings.is_empty() {
        // Check if we have multiple outputs in any function
        let max_outputs = past.functions.iter()
            .map(|f| f.outputs.len())
            .max()
            .unwrap_or(0);
            
        // Always create indexed names like HUGR-LLVM does for consistency
        // Using "c", "c1", "c2" pattern to match HUGR-LLVM
        global_strings.push(GlobalString {
            name: "str_c".to_string(),
            value: "c".to_string(),
            length: 2,
        });
        
        // Create additional strings if needed for multiple outputs
        for i in 1..max_outputs.max(1) {
            let name = format!("c{}", i);
            global_strings.push(GlobalString {
                name: format!("str_{}", name),
                value: name.clone(),
                length: name.len() + 1,
            });
        }
    }

    Ok(MlirModule {
        name: past.name.clone(),
        functions: mlir_functions,
        external_funcs,
        global_strings,
    })
}

/// Extract result names from function nodes
fn extract_result_names_from_function(func: &PastFunction, result_names: &mut std::collections::HashSet<String>) {
    for node in &func.body.nodes {
        match &node.op {
            PastOp::ResultBool(name) | PastOp::ResultInt(name) | PastOp::ResultF64(name) => {
                result_names.insert(name.clone());
            }
            _ => {}
        }
    }
}

/// Helper to create external function declarations
fn create_external_functions(specs: &[(&str, Option<&str>, &[&str])]) -> Vec<ExternalFunc> {
    specs
        .iter()
        .map(|(name, ret_type, arg_types)| ExternalFunc {
            name: (*name).to_string(),
            return_type: ret_type.map(String::from),
            arg_types: arg_types.iter().map(|&s| s.to_string()).collect(),
        })
        .collect()
}

/// Create external function declarations for qubit management (HUGR convention)
fn qubit_management_functions() -> Vec<ExternalFunc> {
    create_external_functions(&[
        ("__quantum__rt__qubit_allocate", Some("i64"), &[]), // HUGR returns i64
        // Note: HUGR runtime doesn't provide __quantum__rt__qubit_release
    ])
}

/// Create external function declarations for single qubit gates (HUGR convention)
fn single_qubit_gate_functions() -> Vec<ExternalFunc> {
    let gates = ["h", "x", "y", "z", "s", "sdg", "t", "tdg"];
    gates
        .iter()
        .map(|&gate| ExternalFunc {
            name: format!("__quantum__qis__{gate}__body"),
            return_type: None,
            arg_types: vec!["i64".to_string()], // HUGR convention uses i64 for qubits
        })
        .collect()
}

/// Create external function declarations for rotation gates (HUGR convention)
fn rotation_gate_functions() -> Vec<ExternalFunc> {
    let gates = ["rx", "ry", "rz"];
    gates
        .iter()
        .map(|&gate| ExternalFunc {
            name: format!("__quantum__qis__{gate}__body"),
            return_type: None,
            arg_types: vec!["f64".to_string(), "i64".to_string()], // HUGR convention
        })
        .collect()
}

/// Create external function declarations for two qubit gates (HUGR convention)
fn two_qubit_gate_functions() -> Vec<ExternalFunc> {
    let gates = ["cx", "cy", "cz", "ch"];
    gates
        .iter()
        .map(|&gate| ExternalFunc {
            name: format!("__quantum__qis__{gate}__body"),
            return_type: None,
            arg_types: vec!["i64".to_string(), "i64".to_string()], // HUGR convention
        })
        .collect()
}

/// Create external function declarations for measurement and results (HUGR convention)
fn measurement_functions() -> Vec<ExternalFunc> {
    create_external_functions(&[
        // HUGR convention: m__body returns i32 and takes result_id
        ("__quantum__qis__m__body", Some("i32"), &["i64", "i64"]),
        ("__quantum__rt__result_allocate", Some("i64"), &[]),
        (
            "__quantum__rt__result_record_output",
            None,
            &["!llvm.ptr<i8>", "!llvm.ptr<i8>"],
        ),
    ])
}

/// Create external function declarations for controlled rotation gates (HUGR convention)
fn controlled_rotation_functions() -> Vec<ExternalFunc> {
    create_external_functions(&[(
        "__quantum__qis__crz__body",
        None,
        &["f64", "i64", "i64"], // HUGR convention
    )])
}

/// Create external function declarations for three-qubit gates (HUGR convention)
fn three_qubit_gate_functions() -> Vec<ExternalFunc> {
    create_external_functions(&[(
        "__quantum__qis__ccx__body",
        None,
        &["i64", "i64", "i64"], // HUGR convention
    )])
}

/// Create external function declarations for special operations
fn special_operation_functions() -> Vec<ExternalFunc> {
    // No special operations needed - cx__body is already in two_qubit_gate_functions
    Vec::new()
}

/// Collect all QIR external function declarations
fn collect_external_functions() -> Vec<ExternalFunc> {
    let mut funcs = Vec::new();
    funcs.extend(qubit_management_functions());
    funcs.extend(single_qubit_gate_functions());
    funcs.extend(rotation_gate_functions());
    funcs.extend(two_qubit_gate_functions());
    funcs.extend(measurement_functions());
    funcs.extend(controlled_rotation_functions());
    funcs.extend(three_qubit_gate_functions());
    funcs.extend(special_operation_functions());
    funcs
}

/// Convert PAST type to MLIR type string (HUGR convention)
fn type_to_mlir(ty: &PastType) -> String {
    match ty {
        PastType::Qubit => "i64".to_string(), // HUGR convention: qubits are i64
        PastType::Custom(_) => "!llvm.ptr<i8>".to_string(), // Keep custom types as opaque pointers
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
        .map(|ty| match ty {
            // HUGR convention: measurement outputs are i32
            PastType::Bit => "i32".to_string(),
            _ => type_to_mlir(ty),
        })
        .collect::<Vec<_>>()
        .join(", ");

    let signature = if func.outputs.is_empty() {
        format!("@{}({})", func.name, input_types)
    } else if output_types.is_empty() {
        format!("@{}({})", func.name, input_types)
    } else {
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
    let mut measurement_count = 0usize;

    // Build edge connectivity map: (dst_node, dst_port) -> (src_node, src_port)
    let mut edge_map = std::collections::HashMap::new();
    for edge in &graph.edges {
        edge_map.insert((edge.dst, edge.dst_port), (edge.src, edge.src_port));
    }

    // Process nodes in topological order (simplified for now)
    for node in &graph.nodes {
        let mlir_ops =
            lower_node_to_operations(node, &value_map, &edge_map, &mut allocated_qubits, &mut measurement_count)?;

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

    // HUGR convention: Don't release qubits explicitly
    // The HUGR runtime doesn't provide __quantum__rt__qubit_release
    // so we skip cleanup operations to match HUGR-LLVM behavior

    // Find the final output value for return
    let mut return_args = vec![];
    
    // Look for measurement results directly in the operations
    let mut measurement_results = vec![];
    for operation in &operations {
        if operation.op_name == "call" && operation.args.iter().any(|arg| arg.contains("@__quantum__qis__m__body")) {
            if !operation.results.is_empty() {
                // This is a measurement operation - get the node ID from the result name
                let result_value = &operation.results[0];
                // Extract node ID from the result value (e.g., %8 -> 8)
                if let Some(num_str) = result_value.strip_prefix('%') {
                    if let Ok(node_id) = num_str.parse::<usize>() {
                        measurement_results.push((node_id, result_value.clone()));
                    }
                }
            }
        }
    }
    
    // Sort by node ID to ensure consistent ordering
    measurement_results.sort_by_key(|(node_id, _)| *node_id);
    
    // Use measurement results as return values
    if !measurement_results.is_empty() {
        let measurement_values: Vec<String> = measurement_results.into_iter()
            .map(|(_, value)| value)
            .collect();
        
        // Return all measurements individually as the program specifies
        return_args = measurement_values;
    }
    
    // If no measurement results found using the proper method, fall back to pattern matching
    if return_args.is_empty() {
        // Look through all operations to find measurement calls and their results
        for operation in &operations {
            if operation.op_name == "call" && operation.args.iter().any(|arg| arg.contains("@__quantum__qis__m__body")) {
                if !operation.results.is_empty() {
                    return_args.push(operation.results[0].clone());
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
#[allow(clippy::too_many_lines)]
fn lower_node_to_operations(
    node: &PastNode,
    value_map: &std::collections::HashMap<(usize, usize), String>,
    edge_map: &std::collections::HashMap<(usize, usize), (usize, usize)>,
    allocated_qubits: &mut Vec<String>,
    measurement_count: &mut usize,
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
        // Quantum operations using func.call (HUGR convention)
        PastOp::H => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__h__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::X => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__x__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::Y => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__y__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::Z => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__z__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::S => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__s__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::T => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__t__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::Sdg => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__sdg__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::Tdg => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!("@__quantum__qis__tdg__body({})", get_input_arg(0))],
            attrs: vec![("type".to_string(), "(i64) -> ()".to_string())],
        }]),

        PastOp::CX => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__cx__body({}, {})",
                get_input_arg(0),
                get_input_arg(1)
            )],
            attrs: vec![(
                "type".to_string(),
                "(i64, i64) -> ()".to_string(),
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
                "(i64, i64) -> ()".to_string(),
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
                "(i64, i64) -> ()".to_string(),
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
                "(i64, i64) -> ()".to_string(),
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
                "(i64, i64, i64) -> ()".to_string(),
            )],
        }]),

        // Rotation gates (HUGR convention)
        PastOp::RX(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__rx__body({}, {})",
                angle,
                get_input_arg(0)
            )],
            attrs: vec![("type".to_string(), "(f64, i64) -> ()".to_string())],
        }]),

        PastOp::RY(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__ry__body({}, {})",
                angle,
                get_input_arg(0)
            )],
            attrs: vec![("type".to_string(), "(f64, i64) -> ()".to_string())],
        }]),

        PastOp::RZ(angle) => Ok(vec![MlirOperation {
            results: vec![],
            op_name: "call".to_string(),
            args: vec![format!(
                "@__quantum__qis__rz__body({}, {})",
                angle,
                get_input_arg(0)
            )],
            attrs: vec![("type".to_string(), "(f64, i64) -> ()".to_string())],
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
                "(f64, i64, i64) -> ()".to_string(),
            )],
        }]),

        PastOp::Measure => {
            // HUGR convention: allocate result, call measure (returns i32), and record output
            let result_id = format!("%result_id_{}", node.id);
            let measurement_result = format!("%{}", node.id);
            let result_ptr = format!("%result_ptr_{}", node.id);
            let qubit_input = get_input_arg(0);

            // Create operations following HUGR convention
            let alloc_result = MlirOperation {
                results: vec![result_id.clone()],
                op_name: "call".to_string(),
                args: vec!["@__quantum__rt__result_allocate()".to_string()],
                attrs: vec![("type".to_string(), "() -> i64".to_string())],
            };

            let measure = MlirOperation {
                results: vec![measurement_result.clone()],
                op_name: "call".to_string(),
                args: vec![format!(
                    "@__quantum__qis__m__body({}, {})",
                    qubit_input, result_id
                )],
                attrs: vec![(
                    "type".to_string(),
                    "(i64, i64) -> i32".to_string(),
                )],
            };

            // Convert result_id to pointer for result recording (HUGR convention)
            let inttoptr = MlirOperation {
                results: vec![result_ptr.clone()],
                op_name: "llvm.inttoptr".to_string(),
                args: vec![result_id.clone()],
                attrs: vec![("type".to_string(), "i64 to !llvm.ptr<i8>".to_string())],
            };

            // Choose the appropriate result name based on measurement index
            // For now, always use indexed names for consistency with multiple outputs
            let (result_name, array_size) = if *measurement_count == 0 {
                ("@str_c", 2)
            } else {
                let name = format!("@str_c{}", measurement_count);
                let size = 2 + measurement_count.to_string().len(); // "c" + number + null terminator
                (Box::leak(name.into_boxed_str()) as &str, size)
            };
            
            // Increment measurement count for next measurement
            *measurement_count += 1;
            
            // Get pointer to global string using llvm.mlir.addressof
            let str_ptr = format!("%str_ptr_{}", node.id);
            let get_str_ptr = MlirOperation {
                results: vec![str_ptr.clone()],
                op_name: "llvm.mlir.addressof".to_string(),
                args: vec![result_name.to_string()],
                attrs: vec![("type".to_string(), format!("!llvm.ptr<!llvm.array<{} x i8>>", array_size))],
            };
            
            // Cast array pointer to i8*
            let str_i8_ptr = format!("%str_i8_ptr_{}", node.id);
            let cast_str = MlirOperation {
                results: vec![str_i8_ptr.clone()],
                op_name: "llvm.bitcast".to_string(),
                args: vec![str_ptr],
                attrs: vec![("type".to_string(), format!("!llvm.ptr<!llvm.array<{} x i8>> to !llvm.ptr<i8>", array_size))],
            };
            
            let record_output = MlirOperation {
                results: vec![],
                op_name: "call".to_string(),
                args: vec![format!(
                    "@__quantum__rt__result_record_output({}, {})",
                    result_ptr, str_i8_ptr
                )],
                attrs: vec![(
                    "type".to_string(),
                    "(!llvm.ptr<i8>, !llvm.ptr<i8>) -> ()".to_string(),
                )],
            };

            Ok(vec![alloc_result, measure, inttoptr, get_str_ptr, cast_str, record_output])
        }

        PastOp::AllocQubit | PastOp::QAlloc => {
            // Allocate a qubit (will fix indexing in LLVM IR post-processing)
            let qubit_var = format!("%{}", node.id);
            allocated_qubits.push(qubit_var.clone());

            Ok(vec![MlirOperation {
                results: vec![qubit_var],
                op_name: "call".to_string(),
                args: vec!["@__quantum__rt__qubit_allocate()".to_string()],
                attrs: vec![("type".to_string(), "() -> i64".to_string())],
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

        // Result operations - these represent the connection between measurements and output names
        PastOp::ResultBool(name) | PastOp::ResultInt(name) | PastOp::ResultF64(name) => {
            // Result operations in HUGR are used to name measurement outputs
            // In our LLVM IR generation, the actual result recording happens in the measurement operation
            // So we just generate a comment to track the result name mapping
            Ok(vec![MlirOperation {
                results: vec![],
                op_name: format!("// result operation for '{}'", name),
                args: vec![],
                attrs: vec![],
            }])
        }

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
            global_strings: vec![],
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
