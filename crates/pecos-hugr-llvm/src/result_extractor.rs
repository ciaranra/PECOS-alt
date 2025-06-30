/*!
Result Name Extractor for HUGR→LLVM compilation

This module extracts result names from tket2.result operations in HUGR graphs
and maps them to corresponding measurement operations.
*/

use hugr_core::ops::ExtensionOp;
use hugr_core::{HugrView, Node};
use pecos_core::errors::PecosError;
use std::collections::HashMap;

/// Maps measurement nodes to their corresponding result names
pub type ResultNameMapping = HashMap<Node, String>;

/// Extracts result names from tket2.result operations in a HUGR
pub struct ResultNameExtractor;

impl ResultNameExtractor {
    /// Traverse the HUGR and build a mapping from measurement operations to result names
    ///
    /// This function:
    /// 1. Finds all tket2.result operations and extracts their string argument (result name)
    /// 2. Traces the dataflow backwards to find the corresponding measurement operations
    /// 3. If no explicit result operations found, generates default names for all measurements
    /// 4. Returns a mapping from measurement node IDs to result names
    ///
    /// # Errors
    /// Returns `PecosError` if the HUGR traversal fails or if connected measurements cannot be found
    pub fn extract_result_names<H: HugrView<Node = Node>>(
        hugr: &H,
    ) -> Result<ResultNameMapping, PecosError> {
        let mut result_mapping = HashMap::new();

        // First, find all tket2.result operations and extract their result names
        let mut result_nodes_with_names = HashMap::new();

        for node in hugr.nodes() {
            if let Some(op) = hugr.get_optype(node).as_extension_op() {
                if op.def().name() == "result_bool"
                    || op.def().name() == "result_int"
                    || op.def().name() == "result_f64"
                {
                    // Extract the string argument from the result operation
                    if let Some(result_name) = Self::extract_string_arg_from_result_op(op) {
                        result_nodes_with_names.insert(node, result_name);
                    }
                }
            }
        }

        // Now trace backwards from each result operation to find connected measurements
        for (result_node, result_name) in result_nodes_with_names {
            if let Some(measurement_node) = Self::find_connected_measurement(hugr, result_node) {
                result_mapping.insert(measurement_node, result_name);
            }
        }

        // If no explicit result operations were found, create default names for all measurements
        if result_mapping.is_empty() {
            let mut measurement_count = 0;
            for node in hugr.nodes() {
                if let Some(op) = hugr.get_optype(node).as_extension_op() {
                    if op.def().name() == "MeasureFree" {
                        let default_name = if measurement_count == 0 {
                            "c".to_string()
                        } else {
                            format!("c{measurement_count}")
                        };
                        result_mapping.insert(node, default_name);
                        measurement_count += 1;
                    }
                }
            }
        }

        Ok(result_mapping)
    }

    /// Extract the string argument from a tket2.result operation
    fn extract_string_arg_from_result_op(op: &ExtensionOp) -> Option<String> {
        // tket2.result operations have a string parameter as their first argument
        let args = op.args();
        if let Some(hugr_core::types::TypeArg::String { arg }) = args.first() {
            return Some(arg.clone());
        }
        None
    }

    /// Find the measurement operation that feeds into this result operation
    ///
    /// This traces the dataflow graph backwards from a result operation to find
    /// the corresponding measurement operation that produced the data.
    fn find_connected_measurement<H: HugrView<Node = Node>>(
        hugr: &H,
        result_node: Node,
    ) -> Option<Node> {
        // Start from the result node and trace backwards through the dataflow
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(result_node);
        visited.insert(result_node);

        while let Some(current_node) = queue.pop_front() {
            // Get all input connections to this node
            for input_port in hugr.node_inputs(current_node) {
                if let Some((source_node, _output_port)) =
                    hugr.linked_outputs(current_node, input_port).next()
                {
                    if visited.contains(&source_node) {
                        continue;
                    }
                    visited.insert(source_node);

                    // Check if this is a measurement operation
                    if let Some(op) = hugr.get_optype(source_node).as_extension_op() {
                        if op.def().name() == "MeasureFree" {
                            return Some(source_node);
                        }
                    }

                    // Continue searching backwards
                    queue.push_back(source_node);
                }
            }
        }

        None
    }
}
