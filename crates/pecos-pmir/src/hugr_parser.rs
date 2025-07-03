/*!
HUGR Parser using Pest

This module parses HUGR JSON format into PAST (PECOS AST) structures.
*/

use pecos_core::errors::PecosError;
use pest_derive::Parser;
use serde_json::Value;
use std::collections::HashMap;

use super::ast::{
    EdgeType, PastEdge, PastFunction, PastGraph, PastModule, PastNode, PastOp, PastType, PastValue,
};

#[derive(Parser)]
#[grammar = "hugr.pest"]
pub struct HugrParser;

/// Parse HUGR JSON into PAST representation
///
/// # Errors
///
/// Returns `PecosError::ParseSyntax` if the JSON is invalid or doesn't match the expected HUGR format
pub fn parse_hugr_to_past(hugr_json: &str) -> Result<PastModule, PecosError> {
    // For now, we'll use serde_json for initial parsing and convert to PAST
    // In the future, we can use the Pest grammar for more control
    let json_value: Value =
        serde_json::from_str(hugr_json).map_err(|e| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: format!("Invalid HUGR JSON: {e}"),
        })?;

    convert_json_to_past(&json_value)
}

/// Convert JSON Value to PAST module
fn convert_json_to_past(json: &Value) -> Result<PastModule, PecosError> {
    let obj = json.as_object().ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "HUGR root must be an object".to_string(),
    })?;

    // Extract modules array (new HUGR format)
    let modules = obj
        .get("modules")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing 'modules' array in HUGR".to_string(),
        })?;

    if modules.is_empty() {
        return Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "No modules found in HUGR".to_string(),
        });
    }

    // Process first module
    let first_module = modules[0]
        .as_object()
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "First module is not an object".to_string(),
        })?;

    // Parse nodes and edges from module
    let mut nodes = parse_nodes(first_module)?;
    let edges = parse_edges(first_module)?;
    
    // Get the original nodes array for function signature parsing
    let original_nodes = first_module
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing 'nodes' array in first module".to_string(),
        })?;

    // Resolve rotation angles from dataflow edges
    super::angle_resolver::resolve_rotation_angles(&mut nodes, &edges)?;

    // Extract metadata
    let name = first_module
        .get("metadata")
        .and_then(|m| m.as_object())
        .and_then(|m| m.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed_module")
        .to_string();

    let version = first_module
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("live")
        .to_string();

    // Build functions from the graph - pass original JSON nodes for signature parsing
    let functions = build_functions_from_graph(&nodes, &edges, original_nodes)?;

    Ok(PastModule {
        name,
        version,
        entry_point: find_entry_point(&functions),
        functions,
        types: HashMap::new(),
    })
}

/// Parse nodes from HUGR JSON
fn parse_nodes(obj: &serde_json::Map<String, Value>) -> Result<Vec<PastNode>, PecosError> {
    let nodes_array =
        obj.get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Missing 'nodes' array".to_string(),
            })?;

    let mut past_nodes = Vec::new();

    for (idx, node_value) in nodes_array.iter().enumerate() {
        let node_obj = node_value
            .as_object()
            .ok_or_else(|| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: format!("Node {idx} is not an object"),
            })?;

        let op = parse_operation(node_obj)?;

        // Count inputs/outputs based on operation type
        let (inputs, outputs) = count_ports(&op);

        past_nodes.push(PastNode {
            id: idx,
            op,
            inputs,
            outputs,
        });
    }

    Ok(past_nodes)
}

/// Extract operation type from node
fn extract_op_type(node: &serde_json::Map<String, Value>) -> Result<(&str, bool), PecosError> {
    let op_value = node.get("op").ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "Missing 'op' in node".to_string(),
    })?;

    // Handle both string and object forms of op
    if let Some(op_str) = op_value.as_str() {
        // Check if it's an Extension operation by looking at node fields
        if op_str == "Extension" && node.contains_key("name") {
            // Get the extension operation name
            let ext_name = node.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Missing name in Extension operation".to_string(),
                }
            })?;
            Ok((ext_name, true))
        } else {
            Ok((op_str, false))
        }
    } else if let Some(op_obj) = op_value.as_object() {
        // For ExtensionOp, get the operation name
        if op_obj.get("op").and_then(|v| v.as_str()) == Some("ExtensionOp") {
            let op_name = op_obj
                .get("op_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Missing op_name in ExtensionOp".to_string(),
                })?;
            Ok((op_name, true))
        } else {
            let op_type = op_obj.get("op").and_then(|v| v.as_str()).ok_or_else(|| {
                PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Invalid op object structure".to_string(),
                }
            })?;
            Ok((op_type, false))
        }
    } else {
        Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Op must be a string or object".to_string(),
        })
    }
}

/// Parse quantum gate operations
fn parse_quantum_gate(
    op_type: &str,
    node: &serde_json::Map<String, Value>,
) -> Result<PastOp, PecosError> {
    match op_type {
        "H" | "h" | "Hadamard" => Ok(PastOp::H),
        "X" | "x" | "PauliX" => Ok(PastOp::X),
        "Y" | "y" | "PauliY" => Ok(PastOp::Y),
        "Z" | "z" | "PauliZ" => Ok(PastOp::Z),
        "CX" | "cx" | "CNOT" => Ok(PastOp::CX),
        "CY" | "cy" => Ok(PastOp::CY),
        "CZ" | "cz" => Ok(PastOp::CZ),
        "CH" | "ch" => Ok(PastOp::CH),
        "Toffoli" | "toffoli" | "CCX" | "ccx" => Ok(PastOp::Toffoli),
        "S" | "s" => Ok(PastOp::S),
        "T" | "t" => Ok(PastOp::T),
        "Sdg" | "sdg" => Ok(PastOp::Sdg),
        "Tdg" | "tdg" => Ok(PastOp::Tdg),
        "Rx" | "RX" | "rx" => {
            let angle = parse_angle_from_node(node);
            Ok(PastOp::RX(angle))
        }
        "Ry" | "RY" | "ry" => {
            let angle = parse_angle_from_node(node);
            Ok(PastOp::RY(angle))
        }
        "Rz" | "RZ" | "rz" => {
            let angle = parse_angle_from_node(node);
            Ok(PastOp::RZ(angle))
        }
        "CRz" | "CRZ" | "crz" => {
            let angle = parse_angle_from_node(node);
            Ok(PastOp::CRZ(angle))
        }
        "MeasureFree" | "Measure" | "measure" | "MeasureZ" => Ok(PastOp::Measure),
        "Reset" | "reset" => Ok(PastOp::Reset),
        "QAlloc" | "AllocQubit" | "q_alloc" => Ok(PastOp::QAlloc),
        "result_bool" => {
            let name = parse_result_name_from_node(node).unwrap_or_else(|| "result".to_string());
            Ok(PastOp::ResultBool(name))
        }
        "result_int" => {
            let name = parse_result_name_from_node(node).unwrap_or_else(|| "result".to_string());
            Ok(PastOp::ResultInt(name))
        }
        "result_f64" => {
            let name = parse_result_name_from_node(node).unwrap_or_else(|| "result".to_string());
            Ok(PastOp::ResultF64(name))
        }
        _ => Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: format!("Unknown quantum operation: {op_type}"),
        }),
    }
}

/// Parse special node operations (Input, Output, Const)
fn parse_special_node(op_type: &str, op_value: &Value) -> Result<Option<PastOp>, PecosError> {
    match op_type {
        "Input" => {
            let port = if let Some(op_obj) = op_value.as_object() {
                op_obj
                    .get("port")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| usize::try_from(v).ok())
                    .unwrap_or(0)
            } else {
                0
            };
            Ok(Some(PastOp::Input(port)))
        }
        "Output" => {
            let port = if let Some(op_obj) = op_value.as_object() {
                op_obj
                    .get("port")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| usize::try_from(v).ok())
                    .unwrap_or(0)
            } else {
                0
            };
            Ok(Some(PastOp::Output(port)))
        }
        "Const" => {
            if let Some(op_obj) = op_value.as_object() {
                let value = parse_const_value(op_obj)?;
                Ok(Some(PastOp::Const(value)))
            } else {
                Err(PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Const operation requires object form".to_string(),
                })
            }
        }
        _ => Ok(None),
    }
}

/// Parse operation from node object
fn parse_operation(node: &serde_json::Map<String, Value>) -> Result<PastOp, PecosError> {
    let (op_type, _is_extension) = extract_op_type(node)?;

    let op_value = node.get("op").ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "Missing 'op' in node".to_string(),
    })?;

    // Try special nodes first
    if let Some(op) = parse_special_node(op_type, op_value)? {
        return Ok(op);
    }

    match op_type {
        // Try quantum gates first
        "H" | "h" | "Hadamard" | "X" | "x" | "PauliX" | "Y" | "y" | "PauliY" | "Z" | "z"
        | "PauliZ" | "CX" | "cx" | "CNOT" | "CY" | "cy" | "CZ" | "cz" | "CH" | "ch" | "Toffoli"
        | "toffoli" | "CCX" | "ccx" | "S" | "s" | "T" | "t" | "Sdg" | "sdg" | "Tdg" | "tdg"
        | "Rx" | "RX" | "rx" | "Ry" | "RY" | "ry" | "Rz" | "RZ" | "rz" | "CRz" | "CRZ" | "crz"
        | "MeasureFree" | "Measure" | "measure" | "MeasureZ" | "Reset" | "reset" | "QAlloc"
        | "AllocQubit" | "q_alloc" => parse_quantum_gate(op_type, node),

        // Classical operations
        "Add" => Ok(PastOp::Add),
        "Sub" => Ok(PastOp::Sub),
        "Mul" => Ok(PastOp::Mul),
        "Div" => Ok(PastOp::Div),

        // Function operations
        "Call" => {
            let func_name = if let Some(op_obj) = op_value.as_object() {
                op_obj
                    .get("function")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                "unknown".to_string()
            };
            Ok(PastOp::Call(func_name))
        }

        // Handle other common HUGR operations
        "Module" | "FuncDefn" | "FuncDecl" | "CFG" | "DataflowBlock" | "ExitBlock"
        | "BasicBlock" | "Conditional" | "TailLoop" | "Tag" | "Lift" | "MakeTuple"
        | "UnpackTuple" | "Case" | "LoadConstant" | "LoadFunction" => {
            // These are structural nodes, map to special operations
            Ok(PastOp::Input(0)) // Placeholder for now
        }

        _ => Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: format!("Unknown operation type: {op_type}"),
        }),
    }
}

/// Parse angle parameter for rotation gates from node
fn parse_angle_from_node(_node: &serde_json::Map<String, Value>) -> f64 {
    // In the new HUGR format, angles are passed as inputs to the operation
    // through the dataflow graph, not as direct attributes.
    // The actual angle resolution happens in a post-processing step
    // using angle_resolver::resolve_rotation_angles
    0.0 // Placeholder value, will be replaced by angle resolver
}

/// Parse constant value
fn parse_const_value(op_obj: &serde_json::Map<String, Value>) -> Result<PastValue, PecosError> {
    let value = op_obj.get("value").ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "Missing value in Const".to_string(),
    })?;

    if let Some(b) = value.as_bool() {
        Ok(PastValue::Bool(b))
    } else if let Some(i) = value.as_i64() {
        Ok(PastValue::Int(i))
    } else if let Some(f) = value.as_f64() {
        Ok(PastValue::Float(f))
    } else if let Some(s) = value.as_str() {
        Ok(PastValue::String(s.to_string()))
    } else {
        Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid constant value type".to_string(),
        })
    }
}

/// Count input/output ports for an operation
fn count_ports(op: &PastOp) -> (usize, usize) {
    match op {
        // Operations with 1 in, 1 out (single qubit gates and misc operations)
        PastOp::H
        | PastOp::X
        | PastOp::Y
        | PastOp::Z
        | PastOp::S
        | PastOp::T
        | PastOp::Sdg
        | PastOp::Tdg
        | PastOp::RX(_)
        | PastOp::RY(_)
        | PastOp::RZ(_)
        | PastOp::Measure
        | PastOp::Reset
        | PastOp::Compare(_)
        | PastOp::Branch
        | PastOp::Call(_)
        | PastOp::Return
        | PastOp::Loop
        | PastOp::Load
        | PastOp::Store => (1, 1),

        // Two qubit gates: 2 in, 2 out
        PastOp::CX | PastOp::CY | PastOp::CZ | PastOp::CH | PastOp::CRZ(_) => (2, 2),

        // Three qubit gates: 3 in, 3 out
        PastOp::Toffoli => (3, 3),

        // Operations with 0 in, 1 out
        PastOp::QAlloc
        | PastOp::AllocQubit
        | PastOp::AllocBit(_)
        | PastOp::Const(_)
        | PastOp::Input(_) => (0, 1),

        // Binary operations: 2 in, 1 out
        PastOp::Add | PastOp::Sub | PastOp::Mul | PastOp::Div => (2, 1),

        // Output node: 1 in, 0 out
        PastOp::Output(_) => (1, 0),
        
        // Result operations: 1 in, 0 out (they consume measurement results)
        PastOp::ResultBool(_) | PastOp::ResultInt(_) | PastOp::ResultF64(_) => (1, 0),
    }
}

/// Parse edges from HUGR JSON
fn parse_edges(obj: &serde_json::Map<String, Value>) -> Result<Vec<PastEdge>, PecosError> {
    let edges_array =
        obj.get("edges")
            .and_then(|v| v.as_array())
            .ok_or_else(|| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Missing 'edges' array".to_string(),
            })?;

    let mut past_edges = Vec::new();

    for edge_value in edges_array {
        // New format: edges are arrays of two arrays [[src_node, src_port], [dst_node, dst_port]]
        if let Some(edge_array) = edge_value.as_array() {
            if edge_array.len() == 2 {
                let (src, src_port) = parse_node_port(Some(&edge_array[0]))?;
                let (dst, dst_port) = parse_node_port(Some(&edge_array[1]))?;

                let edge_type = EdgeType::Data(PastType::Qubit); // Default for now

                past_edges.push(PastEdge {
                    src,
                    src_port,
                    dst,
                    dst_port,
                    edge_type,
                });
                continue;
            }
        }

        // Old format fallback: edges as objects with src/dst fields
        if let Some(edge_obj) = edge_value.as_object() {
            let (src, src_port) = parse_node_port(edge_obj.get("src"))?;
            let (dst, dst_port) = parse_node_port(edge_obj.get("dst"))?;

            let edge_type = EdgeType::Data(PastType::Qubit); // Default for now

            past_edges.push(PastEdge {
                src,
                src_port,
                dst,
                dst_port,
                edge_type,
            });
        } else {
            return Err(PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Invalid edge format".to_string(),
            });
        }
    }

    Ok(past_edges)
}

/// Parse node and port from array like [`node_id`, `port_id`]
fn parse_node_port(value: Option<&Value>) -> Result<(usize, usize), PecosError> {
    let array = value
        .and_then(|v| v.as_array())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid node port format".to_string(),
        })?;

    if array.len() != 2 {
        return Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Node port must have 2 elements".to_string(),
        });
    }

    let node = array[0]
        .as_u64()
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid node ID".to_string(),
        })
        .and_then(|v| {
            usize::try_from(v).map_err(|_| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Node ID too large for platform".to_string(),
            })
        })?;

    let port = array[1]
        .as_u64()
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid port ID".to_string(),
        })
        .and_then(|v| {
            usize::try_from(v).map_err(|_| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Port ID too large for platform".to_string(),
            })
        })?;

    Ok((node, port))
}

/// Parse result name from tket2.result operation (similar to hugr-llvm)
fn parse_result_name_from_node(node: &serde_json::Map<String, Value>) -> Option<String> {
    // tket2.result operations have the result name as the first string parameter in args
    if let Some(op) = node.get("op").and_then(|v| v.as_object()) {
        if let Some(args) = op.get("args").and_then(|v| v.as_array()) {
            // Look for string argument in args array
            for arg in args {
                if let Some(string_arg) = arg.get("String").and_then(|v| v.as_str()) {
                    return Some(string_arg.to_string());
                }
                // Also handle direct string format
                if let Some(string_val) = arg.as_str() {
                    return Some(string_val.to_string());
                }
            }
        }
    }
    None
}

/// Build functions from nodes and edges
fn build_functions_from_graph(nodes: &[PastNode], edges: &[PastEdge], original_nodes: &[Value]) -> Result<Vec<PastFunction>, PecosError> {
    // For now, create a single main function containing all nodes
    // In the future, we'll properly identify function boundaries

    // Find Output nodes as exit nodes
    let exit_nodes: Vec<usize> = nodes
        .iter()
        .filter_map(|node| match &node.op {
            PastOp::Output(_) => Some(node.id),
            _ => None,
        })
        .collect();

    // Find Input nodes as entry points
    let entry_node = nodes
        .iter()
        .find_map(|node| match &node.op {
            PastOp::Input(_) => Some(node.id),
            _ => None,
        })
        .unwrap_or(0);

    // Extract output types from the HUGR function signature
    let output_types = extract_function_output_types(original_nodes)?;

    let main_func = PastFunction {
        name: "main".to_string(),
        inputs: vec![], // TODO: Identify input parameters
        outputs: output_types,
        body: PastGraph {
            nodes: nodes.to_vec(),
            edges: edges.to_vec(),
            entry: entry_node,
            exits: exit_nodes,
        },
    };

    Ok(vec![main_func])
}

/// Extract function output types from HUGR nodes
/// This finds the FuncDefn node and parses its signature to determine actual output types
fn extract_function_output_types(nodes: &[Value]) -> Result<Vec<PastType>, PecosError> {
    // Find the FuncDefn node (should be node 1 in modern HUGR format)
    let func_defn_node = nodes
        .iter()
        .enumerate()
        .find(|(_, node)| {
            node.as_object()
                .and_then(|obj| obj.get("op"))
                .and_then(|op| op.as_str()) == Some("FuncDefn")
        })
        .map(|(_, node)| node)
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "No FuncDefn node found".to_string(),
        })?;

    // Extract the signature from the FuncDefn node (if it exists)
    let signature = func_defn_node
        .as_object()
        .and_then(|obj| obj.get("signature"));

    if let Some(sig) = signature {
        // New HUGR format with explicit signature
        let outputs = sig.as_object()
            .and_then(|sig| sig.get("body"))
            .and_then(|body| body.as_object())
            .and_then(|body| body.get("output"))
            .and_then(|output| output.as_array())
            .ok_or_else(|| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Invalid function signature output format".to_string(),
            })?;

        // Convert HUGR output types to PAST types
        let mut output_types = Vec::new();
        for output_type in outputs {
            // For now, all measurement outputs are treated as Bit (i32 in MLIR)
            if let Some(obj) = output_type.as_object() {
                if let Some(t) = obj.get("t").and_then(|v| v.as_str()) {
                    match t {
                        "Opaque" => {
                            // Check if it's a bool type (measurement result)
                            if obj.get("extension").and_then(|v| v.as_str()) == Some("tket2.bool") {
                                output_types.push(PastType::Bit);
                            } else {
                                // Other opaque types, default to bit for now
                                output_types.push(PastType::Bit);
                            }
                        }
                        "Sum" => {
                            // Sum types (like unit sums for booleans) are also measurement results
                            output_types.push(PastType::Bit);
                        }
                        _ => {
                            // Other types, default to bit
                            output_types.push(PastType::Bit);
                        }
                    }
                } else {
                    // Fallback for unknown type format
                    output_types.push(PastType::Bit);
                }
            }
        }
        Ok(output_types)
    } else {
        // Old HUGR format without signature - fallback to counting measurement operations
        let measurement_count = count_measurement_operations(nodes);
        let output_types = (0..measurement_count).map(|_| PastType::Bit).collect();
        Ok(output_types)
    }
}

/// Count measurement operations in HUGR nodes (fallback for old format)
fn count_measurement_operations(nodes: &[Value]) -> usize {
    nodes
        .iter()
        .filter(|node| {
            if let Some(obj) = node.as_object() {
                // Check for MeasureFree operation
                if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                    return name == "MeasureFree" || name == "Measure";
                }
                // Check for Extension operations with measurement names
                if obj.get("op").and_then(|v| v.as_str()) == Some("Extension") {
                    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                        return name == "MeasureFree" || name == "Measure";
                    }
                }
            }
            false
        })
        .count()
}

/// Find the entry point function
fn find_entry_point(functions: &[PastFunction]) -> Option<String> {
    // Look for "main" or the first function
    functions
        .iter()
        .find(|f| f.name == "main")
        .or_else(|| functions.first())
        .map(|f| f.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_hugr() {
        let hugr_json = r#"{
            "modules": [{
                "version": "live",
                "metadata": {"name": "test"},
                "nodes": [
                    {"parent": 0, "op": "Module"},
                    {"parent": 0, "op": "FuncDefn", "name": "main"},
                    {"parent": 1, "op": "Input"},
                    {"parent": 1, "op": "Output"},
                    {"parent": 1, "op": "Extension", "name": "H"},
                    {"parent": 1, "op": "Extension", "name": "MeasureFree"}
                ],
                "edges": [
                    [[2, 0], [4, 0]],
                    [[4, 0], [5, 0]],
                    [[5, 0], [3, 0]]
                ]
            }],
            "extensions": []
        }"#;

        let past = parse_hugr_to_past(hugr_json).unwrap();
        assert_eq!(past.name, "test");
        assert!(!past.functions.is_empty());
    }
}
