/*!
Angle Resolver for HUGR Rotation Gates

This module provides functionality to resolve angle values for rotation gates
by following the dataflow edges in HUGR graphs.
*/

use crate::ast::{PastEdge, PastNode, PastOp};
use pecos_core::errors::PecosError;

/// Resolve angles for all rotation gates in the node list by following dataflow edges
pub fn resolve_rotation_angles(nodes: &mut Vec<PastNode>, edges: &[PastEdge]) -> Result<(), PecosError> {
    // First, build a map of constant values by node ID
    let mut const_values = std::collections::HashMap::new();
    for node in nodes.iter() {
        if let PastOp::Const(value) = &node.op {
            if let crate::ast::PastValue::Float(f) = value {
                const_values.insert(node.id, *f);
            }
        }
    }

    // Now update rotation gates with their angle values
    for node in nodes.iter_mut() {
        match &mut node.op {
            PastOp::RX(angle) | PastOp::RY(angle) | PastOp::RZ(angle) => {
                // Find edge that connects to this node's port 1 (angle input)
                if let Some(angle_value) = find_angle_input(node.id, edges, &const_values) {
                    *angle = angle_value;
                } else {
                    log::warn!("No angle input found for rotation gate at node {}", node.id);
                }
            }
            PastOp::CRZ(angle) => {
                // For controlled rotation, angle is on port 2
                if let Some(angle_value) = find_angle_input_at_port(node.id, 2, edges, &const_values) {
                    *angle = angle_value;
                } else {
                    log::warn!("No angle input found for CRZ gate at node {}", node.id);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Find the angle value connected to a specific node's port 1
fn find_angle_input(
    node_id: usize,
    edges: &[PastEdge],
    const_values: &std::collections::HashMap<usize, f64>,
) -> Option<f64> {
    find_angle_input_at_port(node_id, 1, edges, const_values)
}

/// Find the angle value connected to a specific node and port
fn find_angle_input_at_port(
    node_id: usize,
    port: usize,
    edges: &[PastEdge],
    const_values: &std::collections::HashMap<usize, f64>,
) -> Option<f64> {
    // Find edge that targets this node at the specified port
    for edge in edges {
        if edge.dst == node_id && edge.dst_port == port {
            // Found the edge! Check if source is a constant
            if let Some(&value) = const_values.get(&edge.src) {
                return Some(value);
            }
        }
    }
    None
}