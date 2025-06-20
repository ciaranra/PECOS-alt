/*!
HUGR Parser using Pest

This module parses HUGR JSON format into PAST (PECOS AST) structures.
*/

use pest_derive::Parser;
use pecos_core::errors::PecosError;
use serde_json::Value;
use std::collections::HashMap;

use super::ast::*;

#[derive(Parser)]
#[grammar = "pmir/hugr.pest"]
pub struct HugrParser;

/// Parse HUGR JSON into PAST representation
pub fn parse_hugr_to_past(hugr_json: &str) -> Result<PastModule, PecosError> {
    // For now, we'll use serde_json for initial parsing and convert to PAST
    // In the future, we can use the Pest grammar for more control
    let json_value: Value = serde_json::from_str(hugr_json)
        .map_err(|e| PecosError::ParseSyntax { 
            language: "HUGR".to_string(), 
            message: format!("Invalid HUGR JSON: {}", e) 
        })?;
    
    convert_json_to_past(json_value)
}

/// Convert JSON Value to PAST module
fn convert_json_to_past(json: Value) -> Result<PastModule, PecosError> {
    let obj = json.as_object()
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "HUGR root must be an object".to_string()
        })?;
    
    // Extract module information
    let name = obj.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed_module")
        .to_string();
    
    let version = obj.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();
    
    // Parse nodes and edges
    let nodes = parse_nodes(obj)?;
    let edges = parse_edges(obj)?;
    
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
    let nodes_array = obj.get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing 'nodes' array".to_string()
        })?;
    
    let mut past_nodes = Vec::new();
    
    for (idx, node_value) in nodes_array.iter().enumerate() {
        let node_obj = node_value.as_object()
            .ok_or_else(|| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: format!("Node {} is not an object", idx)
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
    let op_obj = node.get("op")
        .and_then(|v| v.as_object())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing 'op' in node".to_string()
        })?;
    
    let op_type = op_obj.get("op_type")
        .or_else(|| op_obj.get("type"))
        .or_else(|| op_obj.get("op"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing operation type".to_string()
        })?;
    
    match op_type {
        // Quantum gates
        "H" | "Hadamard" => Ok(PastOp::H),
        "X" | "PauliX" => Ok(PastOp::X),
        "Y" | "PauliY" => Ok(PastOp::Y),
        "Z" | "PauliZ" => Ok(PastOp::Z),
        "CX" | "CNOT" => Ok(PastOp::CX),
        "CZ" => Ok(PastOp::CZ),
        "RX" => {
            let angle = parse_angle(op_obj)?;
            Ok(PastOp::RX(angle))
        },
        "RY" => {
            let angle = parse_angle(op_obj)?;
            Ok(PastOp::RY(angle))
        },
        "RZ" => {
            let angle = parse_angle(op_obj)?;
            Ok(PastOp::RZ(angle))
        },
        "Measure" | "MeasureZ" => Ok(PastOp::Measure),
        "Reset" => Ok(PastOp::Reset),
        
        // Classical operations
        "Add" => Ok(PastOp::Add),
        "Sub" => Ok(PastOp::Sub),
        "Mul" => Ok(PastOp::Mul),
        "Div" => Ok(PastOp::Div),
        
        // Memory operations
        "QAlloc" | "AllocQubit" => Ok(PastOp::AllocQubit),
        
        // Special nodes
        "Input" => {
            let port = op_obj.get("port")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            Ok(PastOp::Input(port))
        },
        "Output" => {
            let port = op_obj.get("port")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            Ok(PastOp::Output(port))
        },
        "Const" => {
            let value = parse_const_value(op_obj)?;
            Ok(PastOp::Const(value))
        },
        
        // Function operations
        "Call" => {
            let func_name = op_obj.get("function")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Ok(PastOp::Call(func_name))
        },
        
        _ => Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: format!("Unknown operation type: {}", op_type)
        }),
    }
}

/// Parse angle parameter for rotation gates
fn parse_angle(op_obj: &serde_json::Map<String, Value>) -> Result<f64, PecosError> {
    op_obj.get("angle")
        .or_else(|| op_obj.get("param"))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing angle parameter".to_string()
        })
}

/// Parse constant value
fn parse_const_value(op_obj: &serde_json::Map<String, Value>) -> Result<PastValue, PecosError> {
    let value = op_obj.get("value")
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing value in Const".to_string()
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
            message: "Invalid constant value type".to_string()
        })
    }
}

/// Count input/output ports for an operation
fn count_ports(op: &PastOp) -> (usize, usize) {
    match op {
        // Single qubit gates: 1 in, 1 out
        PastOp::H | PastOp::X | PastOp::Y | PastOp::Z |
        PastOp::RX(_) | PastOp::RY(_) | PastOp::RZ(_) => (1, 1),
        
        // Two qubit gates: 2 in, 2 out
        PastOp::CX | PastOp::CZ => (2, 2),
        
        // Measurement: 1 qubit in, 1 bit out
        PastOp::Measure => (1, 1),
        
        // Reset: 1 in, 1 out
        PastOp::Reset => (1, 1),
        
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
    let edges_array = obj.get("edges")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Missing 'edges' array".to_string()
        })?;
    
    let mut past_edges = Vec::new();
    
    for edge_value in edges_array {
        let edge_obj = edge_value.as_object()
            .ok_or_else(|| PecosError::ParseSyntax {
                language: "HUGR".to_string(),
                message: "Edge is not an object".to_string()
            })?;
        
        let (src, src_port) = parse_node_port(edge_obj.get("src"))?;
        let (dst, dst_port) = parse_node_port(edge_obj.get("dst"))?;
        
        // Determine edge type based on context
        let edge_type = EdgeType::Data(PastType::Qubit); // Default for now
        
        past_edges.push(PastEdge {
            src,
            src_port,
            dst,
            dst_port,
            edge_type,
        });
    }
    
    Ok(past_edges)
}

/// Parse node and port from array like [node_id, port_id]
fn parse_node_port(value: Option<&Value>) -> Result<(usize, usize), PecosError> {
    let array = value
        .and_then(|v| v.as_array())
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid node port format".to_string()
        })?;
    
    if array.len() != 2 {
        return Err(PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Node port must have 2 elements".to_string()
        });
    }
    
    let node = array[0].as_u64()
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid node ID".to_string()
        })? as usize;
    
    let port = array[1].as_u64()
        .ok_or_else(|| PecosError::ParseSyntax {
            language: "HUGR".to_string(),
            message: "Invalid port ID".to_string()
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
    
    let main_func = PastFunction {
        name: "main".to_string(),
        inputs: vec![],  // TODO: Identify input parameters
        outputs: vec![PastType::Bit],  // TODO: Identify output types
        body: PastGraph {
            nodes: nodes.to_vec(),
            edges: edges.to_vec(),
            entry: 0,  // TODO: Find actual entry point
            exits: vec![nodes.len() - 1],  // TODO: Find actual exits
        },
    };
    
    Ok(vec![main_func])
}

/// Find the entry point function
fn find_entry_point(functions: &[PastFunction]) -> Option<String> {
    // Look for "main" or the first function
    functions.iter()
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
            "version": "0.1.0",
            "name": "test",
            "nodes": [
                {"op": {"type": "H"}},
                {"op": {"type": "Measure"}}
            ],
            "edges": [
                {"src": [0, 0], "dst": [1, 0]}
            ]
        }"#;
        
        let past = parse_hugr_to_past(hugr_json).unwrap();
        assert_eq!(past.name, "test");
        assert_eq!(past.functions.len(), 1);
    }
}