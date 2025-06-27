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
#[grammar = "pmir/hugr.pest"]
pub struct HugrParser;

/// Parse HUGR JSON into PAST representation
pub fn parse_hugr_to_past(hugr_json: &str) -> Result<PastModule, PecosError> {
    // For now, we'll use serde_json for initial parsing and convert to PAST
    // In the future, we can use the Pest grammar for more control
    let json_value: Value =
        serde_json::from_str(hugr_json).map_err(|e| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: format!("Invalid HUGR JSON: {e}"),
        })?;

    convert_json_to_past(json_value)
}

/// Convert JSON Value to PAST module
fn convert_json_to_past(json: Value) -> Result<PastModule, PecosError> {
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
    let nodes = parse_nodes(first_module)?;
    let edges = parse_edges(first_module)?;

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

    // Build functions from the graph
    let functions = build_functions_from_graph(&nodes, &edges)?;

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

/// Parse operation from node object
fn parse_operation(node: &serde_json::Map<String, Value>) -> Result<PastOp, PecosError> {
    let op_value = node.get("op").ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "Missing 'op' in node".to_string(),
    })?;

    // Handle both string and object forms of op
    let (op_type, _is_extension) = if let Some(op_str) = op_value.as_str() {
        // Check if it's an Extension operation by looking at node fields
        if op_str == "Extension" && node.contains_key("name") {
            // Get the extension operation name
            let ext_name = node.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Missing name in Extension operation".to_string(),
                }
            })?;
            (ext_name, true)
        } else {
            (op_str, false)
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
            (op_name, true)
        } else {
            let op_type = op_obj
                .get("op_type")
                .or_else(|| op_obj.get("type"))
                .or_else(|| op_obj.get("op"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Missing operation type".to_string(),
                })?;
            (op_type, false)
        }
    } else {
        return Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid 'op' field - must be string or object".to_string(),
        });
    };

    match op_type {
        // Quantum gates
        "H" | "Hadamard" => Ok(PastOp::H),
        "X" | "PauliX" => Ok(PastOp::X),
        "Y" | "PauliY" => Ok(PastOp::Y),
        "Z" | "PauliZ" => Ok(PastOp::Z),
        "CX" | "CNOT" | "cx" => Ok(PastOp::CX),
        "CZ" | "cz" => Ok(PastOp::CZ),
        "CY" | "cy" => Ok(PastOp::CY),
        "CH" | "ch" => Ok(PastOp::CH),
        "S" | "s" => Ok(PastOp::S),
        "T" | "t" => Ok(PastOp::T),
        "Sdg" | "sdg" => Ok(PastOp::Sdg),
        "Tdg" | "tdg" => Ok(PastOp::Tdg),
        "Rx" | "RX" | "rx" => {
            let angle = parse_angle_from_node(node)?;
            Ok(PastOp::RX(angle))
        }
        "Ry" | "RY" | "ry" => {
            let angle = parse_angle_from_node(node)?;
            Ok(PastOp::RY(angle))
        }
        "Rz" | "RZ" | "rz" => {
            let angle = parse_angle_from_node(node)?;
            Ok(PastOp::RZ(angle))
        }
        "CRz" | "CRZ" | "crz" => {
            let angle = parse_angle_from_node(node)?;
            Ok(PastOp::CRZ(angle))
        }
        "Toffoli" | "toffoli" | "CCX" | "ccx" => Ok(PastOp::Toffoli),
        "Measure" | "MeasureZ" | "MeasureFree" => Ok(PastOp::Measure),
        "Reset" => Ok(PastOp::Reset),
        "QAlloc" | "q_alloc" | "AllocQubit" => Ok(PastOp::QAlloc),

        // Classical operations
        "Add" => Ok(PastOp::Add),
        "Sub" => Ok(PastOp::Sub),
        "Mul" => Ok(PastOp::Mul),
        "Div" => Ok(PastOp::Div),

        // Special nodes
        "Input" => {
            let port = if let Some(op_obj) = op_value.as_object() {
                op_obj
                    .get("port")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as usize
            } else {
                0
            };
            Ok(PastOp::Input(port))
        }
        "Output" => {
            let port = if let Some(op_obj) = op_value.as_object() {
                op_obj
                    .get("port")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0) as usize
            } else {
                0
            };
            Ok(PastOp::Output(port))
        }
        "Const" => {
            if let Some(op_obj) = op_value.as_object() {
                let value = parse_const_value(op_obj)?;
                Ok(PastOp::Const(value))
            } else {
                Err(PecosError::ParseSyntax {
                    language: "HUGR".to_string(),
                    message: "Const operation requires object form".to_string(),
                })
            }
        }

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
fn parse_angle_from_node(_node: &serde_json::Map<String, Value>) -> Result<f64, PecosError> {
    // WARNING: This is a stub implementation that always returns 0.0
    // In the new HUGR format, angles are passed as inputs to the operation
    // through the dataflow graph, not as direct attributes.
    // TODO: Implement proper angle extraction from HUGR dataflow by:
    //   1. Following the input edges to find the constant node
    //   2. Extracting the float value from the constant
    // For now, this will make all rotation gates no-ops!
    log::warn!("parse_angle_from_node: Returning 0.0 - rotation gates will be no-ops!");
    Ok(0.0)
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
        // Single qubit gates: 1 in, 1 out
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
        | PastOp::RZ(_) => (1, 1),

        // Two qubit gates: 2 in, 2 out
        PastOp::CX | PastOp::CY | PastOp::CZ | PastOp::CH | PastOp::CRZ(_) => (2, 2),

        // Three qubit gates: 3 in, 3 out
        PastOp::Toffoli => (3, 3),

        // Measurement: 1 qubit in, 1 bit out
        PastOp::Measure => (1, 1),

        // Reset: 1 in, 1 out
        PastOp::Reset => (1, 1),

        // Qubit allocation: 0 in, 1 out
        PastOp::QAlloc => (0, 1),

        // Binary operations: 2 in, 1 out
        PastOp::Add | PastOp::Sub | PastOp::Mul | PastOp::Div => (2, 1),

        // Allocation: 0 in, 1 out
        PastOp::AllocQubit | PastOp::AllocBit(_) => (0, 1),

        // Constants: 0 in, 1 out
        PastOp::Const(_) => (0, 1),

        // Input/Output nodes
        PastOp::Input(_) => (0, 1),
        PastOp::Output(_) => (1, 0),

        // Default for others
        _ => (1, 1),
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

    let node = array[0].as_u64().ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "Invalid node ID".to_string(),
    })? as usize;

    let port = array[1].as_u64().ok_or_else(|| PecosError::ParseSyntax {
        language: "HUGR".to_string(),
        message: "Invalid port ID".to_string(),
    })? as usize;

    Ok((node, port))
}

/// Build functions from nodes and edges
fn build_functions_from_graph(
    nodes: &[PastNode],
    edges: &[PastEdge],
) -> Result<Vec<PastFunction>, PecosError> {
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

    // Determine output types based on edges going into Output nodes
    let mut output_types = Vec::new();
    for exit_node in &exit_nodes {
        // Count how many edges go into this output node
        let incoming_edges: Vec<_> = edges.iter().filter(|e| e.dst == *exit_node).collect();

        // For each incoming edge, add a Bit type (assuming measurements produce bits)
        for _ in incoming_edges {
            output_types.push(PastType::Bit);
        }
    }

    // If no output types found, default to single bit
    if output_types.is_empty() {
        output_types.push(PastType::Bit);
    }

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
