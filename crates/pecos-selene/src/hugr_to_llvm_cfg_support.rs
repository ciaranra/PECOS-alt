// Enhanced HUGR to LLVM compiler with proper CFG support
// This module adds support for Control Flow Graphs in HUGR compilation

use crate::SeleneError;
use std::collections::HashMap;
use std::fmt::Write;
use serde_json::Value;
use log;

/// Enhanced compiler that processes CFG structures in HUGR
pub struct CFGAwareCompiler {
    llvm_ir: String,
    qubit_counter: u32,
    result_counter: u32,
    label_counter: u32,
    qubit_vars: HashMap<usize, String>,
    result_vars: HashMap<usize, String>,
    block_labels: HashMap<usize, String>,
    current_block: Option<String>,
}

impl CFGAwareCompiler {
    pub fn new() -> Self {
        Self {
            llvm_ir: String::new(),
            qubit_counter: 0,
            result_counter: 0,
            label_counter: 0,
            qubit_vars: HashMap::new(),
            result_vars: HashMap::new(),
            block_labels: HashMap::new(),
            current_block: None,
        }
    }

    /// Process a CFG node and all its children blocks
    pub fn process_cfg(&mut self, nodes: &[Value], cfg_node_id: usize, edges: &[Value]) -> Result<(), SeleneError> {
        log::info!("Processing CFG at node {}", cfg_node_id);
        
        // Find all DataflowBlock children of this CFG
        let mut dataflow_blocks = Vec::new();
        let mut entry_block = None;
        let mut exit_blocks = Vec::new();
        
        for (node_id, node) in nodes.iter().enumerate() {
            if let Some(parent) = node.get("parent").and_then(|p| p.as_u64()) {
                if parent as usize == cfg_node_id {
                    if let Some(op) = node.get("op").and_then(|o| o.as_str()) {
                        match op {
                            "DataflowBlock" => {
                                dataflow_blocks.push(node_id);
                                // Generate label for this block
                                let label = format!("block{}", self.label_counter);
                                self.label_counter += 1;
                                self.block_labels.insert(node_id, label);
                            }
                            "Entry" => entry_block = Some(node_id),
                            "ExitBlock" => exit_blocks.push(node_id),
                            _ => {}
                        }
                    }
                }
            }
        }
        
        log::info!("Found {} dataflow blocks in CFG", dataflow_blocks.len());
        
        // Process control flow edges to understand block connections
        let mut block_successors: HashMap<usize, Vec<(usize, usize)>> = HashMap::new(); // block_id -> [(target_block, branch_idx)]
        
        for edge in edges {
            if let Some(edge_array) = edge.as_array() {
                if edge_array.len() >= 2 {
                    if let (Some(src_arr), Some(tgt_arr)) = (
                        edge_array[0].as_array(),
                        edge_array[1].as_array()
                    ) {
                        if let (Some(src_node), Some(src_port), Some(tgt_node), Some(_tgt_port)) = (
                            src_arr.get(0).and_then(|n| n.as_u64()),
                            src_arr.get(1).and_then(|n| n.as_u64()),
                            tgt_arr.get(0).and_then(|n| n.as_u64()),
                            tgt_arr.get(1).and_then(|n| n.as_u64()),
                        ) {
                            let src_id = src_node as usize;
                            let tgt_id = tgt_node as usize;
                            
                            // Check if this is a control flow edge between blocks
                            if dataflow_blocks.contains(&src_id) && dataflow_blocks.contains(&tgt_id) {
                                block_successors.entry(src_id)
                                    .or_insert_with(Vec::new)
                                    .push((tgt_id, src_port as usize));
                            }
                        }
                    }
                }
            }
        }
        
        // Process blocks in topological order
        if !dataflow_blocks.is_empty() {
            // Start with the first block (or entry block if identified)
            let start_block = entry_block
                .and_then(|e| self.find_entry_successor(nodes, edges, e))
                .unwrap_or(dataflow_blocks[0]);
            
            self.process_dataflow_block(nodes, edges, start_block)?;
            
            // Process remaining blocks
            for &block_id in &dataflow_blocks {
                if block_id != start_block {
                    // Add label for this block
                    if let Some(label) = self.block_labels.get(&block_id) {
                        writeln!(&mut self.llvm_ir, "{}:", label)?;
                    }
                    self.process_dataflow_block(nodes, edges, block_id)?;
                }
            }
        }
        
        Ok(())
    }

    /// Find the successor of an Entry node
    fn find_entry_successor(&self, _nodes: &[Value], edges: &[Value], entry_id: usize) -> Option<usize> {
        for edge in edges {
            if let Some(edge_array) = edge.as_array() {
                if edge_array.len() >= 2 {
                    if let Some(src_arr) = edge_array[0].as_array() {
                        if let Some(src_node) = src_arr.get(0).and_then(|n| n.as_u64()) {
                            if src_node as usize == entry_id {
                                if let Some(tgt_arr) = edge_array[1].as_array() {
                                    if let Some(tgt_node) = tgt_arr.get(0).and_then(|n| n.as_u64()) {
                                        return Some(tgt_node as usize);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Process a single DataflowBlock
    pub fn process_dataflow_block(&mut self, nodes: &[Value], edges: &[Value], block_id: usize) -> Result<(), SeleneError> {
        log::info!("Processing DataflowBlock at node {}", block_id);
        
        // Find all operations within this block
        let mut block_ops = Vec::new();
        let mut block_branches = Vec::new();
        
        for (node_id, node) in nodes.iter().enumerate() {
            if let Some(parent) = node.get("parent").and_then(|p| p.as_u64()) {
                if parent as usize == block_id {
                    if let Some(op) = node.get("op").and_then(|o| o.as_str()) {
                        match op {
                            "Extension" => block_ops.push(node_id),
                            "Branch" | "Case" => block_branches.push(node_id),
                            _ => {}
                        }
                    }
                }
            }
        }
        
        log::info!("Found {} operations and {} branches in block {}", 
                   block_ops.len(), block_branches.len(), block_id);
        
        // Process operations in this block
        for op_id in block_ops {
            self.process_operation(nodes, edges, op_id)?;
        }
        
        // Process branches/control flow
        for branch_id in block_branches {
            self.process_branch(nodes, edges, branch_id)?;
        }
        
        Ok(())
    }

    /// Process a quantum operation
    fn process_operation(&mut self, nodes: &[Value], edges: &[Value], op_id: usize) -> Result<(), SeleneError> {
        let node = &nodes[op_id];
        
        if let Some(op_str) = node.get("op").and_then(|o| o.as_str()) {
            if op_str == "Extension" {
                let extension = node.get("extension").and_then(|e| e.as_str()).unwrap_or("");
                let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("");
                
                log::info!("Processing operation: {}::{} at node {}", extension, name, op_id);
                
                match (extension, name) {
                    ("tket.quantum", "QAlloc") => {
                        let qubit_var = format!("%q{}", self.qubit_counter);
                        writeln!(&mut self.llvm_ir, "  {} = call i64 @__quantum__rt__qubit_allocate()", qubit_var)?;
                        self.qubit_vars.insert(op_id, qubit_var);
                        self.qubit_counter += 1;
                    }
                    ("tket.quantum", "H") => {
                        if let Some(qubit_var) = self.find_input_qubit(op_id, edges, nodes) {
                            writeln!(&mut self.llvm_ir, "  call void @__quantum__qis__h__body(i64 {})", qubit_var)?;
                        }
                    }
                    ("tket.quantum", "X") => {
                        if let Some(qubit_var) = self.find_input_qubit(op_id, edges, nodes) {
                            writeln!(&mut self.llvm_ir, "  call void @__quantum__qis__x__body(i64 {})", qubit_var)?;
                        }
                    }
                    ("tket.quantum", "Measure") => {
                        if let Some(qubit_var) = self.find_input_qubit(op_id, edges, nodes) {
                            let result_var = format!("%r{}", self.result_counter);
                            writeln!(&mut self.llvm_ir, "  {} = call i1 @__quantum__qis__mz__body(i64 {})", result_var, qubit_var)?;
                            self.result_vars.insert(op_id, result_var);
                            self.result_counter += 1;
                        }
                    }
                    _ => {
                        log::warn!("Unhandled quantum operation: {}::{}", extension, name);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Process a branch/conditional node
    fn process_branch(&mut self, nodes: &[Value], edges: &[Value], branch_id: usize) -> Result<(), SeleneError> {
        log::info!("Processing branch at node {}", branch_id);
        
        // Find the condition input
        if let Some(condition_var) = self.find_input_result(branch_id, edges, nodes) {
            // Find branch targets
            let mut branch_targets = Vec::new();
            
            for edge in edges {
                if let Some(edge_array) = edge.as_array() {
                    if edge_array.len() >= 2 {
                        if let Some(src_arr) = edge_array[0].as_array() {
                            if let Some(src_node) = src_arr.get(0).and_then(|n| n.as_u64()) {
                                if src_node as usize == branch_id {
                                    if let Some(tgt_arr) = edge_array[1].as_array() {
                                        if let Some(tgt_node) = tgt_arr.get(0).and_then(|n| n.as_u64()) {
                                            branch_targets.push(tgt_node as usize);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            if branch_targets.len() >= 2 {
                let true_label = self.block_labels.get(&branch_targets[0])
                    .cloned()
                    .unwrap_or_else(|| format!("block{}", branch_targets[0]));
                let false_label = self.block_labels.get(&branch_targets[1])
                    .cloned()
                    .unwrap_or_else(|| format!("block{}", branch_targets[1]));
                    
                writeln!(&mut self.llvm_ir, "  br i1 {}, label %{}, label %{}", 
                         condition_var, true_label, false_label)?;
            }
        }
        
        Ok(())
    }

    /// Find input qubit for an operation
    fn find_input_qubit(&self, target_id: usize, edges: &[Value], _nodes: &[Value]) -> Option<String> {
        // Similar to existing implementation but check our stored qubit_vars
        for edge in edges {
            if let Some(edge_array) = edge.as_array() {
                if edge_array.len() >= 2 {
                    if let Some(tgt_arr) = edge_array[1].as_array() {
                        if let Some(tgt_node) = tgt_arr.get(0).and_then(|n| n.as_u64()) {
                            if tgt_node as usize == target_id {
                                if let Some(src_arr) = edge_array[0].as_array() {
                                    if let Some(src_node) = src_arr.get(0).and_then(|n| n.as_u64()) {
                                        let src_id = src_node as usize;
                                        if let Some(qubit) = self.qubit_vars.get(&src_id) {
                                            return Some(qubit.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Find input result (measurement) for a branch
    fn find_input_result(&self, target_id: usize, edges: &[Value], _nodes: &[Value]) -> Option<String> {
        for edge in edges {
            if let Some(edge_array) = edge.as_array() {
                if edge_array.len() >= 2 {
                    if let Some(tgt_arr) = edge_array[1].as_array() {
                        if let Some(tgt_node) = tgt_arr.get(0).and_then(|n| n.as_u64()) {
                            if tgt_node as usize == target_id {
                                if let Some(src_arr) = edge_array[0].as_array() {
                                    if let Some(src_node) = src_arr.get(0).and_then(|n| n.as_u64()) {
                                        let src_id = src_node as usize;
                                        if let Some(result) = self.result_vars.get(&src_id) {
                                            return Some(result.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Generate the final LLVM IR
    pub fn generate_llvm_ir(self) -> Result<String, SeleneError> {
        let mut ir = String::new();
        
        // Add function declarations
        writeln!(&mut ir, "declare i64 @__quantum__rt__qubit_allocate()")?;
        writeln!(&mut ir, "declare void @__quantum__qis__h__body(i64)")?;
        writeln!(&mut ir, "declare void @__quantum__qis__x__body(i64)")?;
        writeln!(&mut ir, "declare i1 @__quantum__qis__mz__body(i64)")?;
        writeln!(&mut ir, "declare void @__quantum__rt__result_record_output(i1)")?;
        writeln!(&mut ir)?;
        
        // Add main function
        writeln!(&mut ir, "define void @main() {{")?;
        writeln!(&mut ir, "entry:")?;
        write!(&mut ir, "{}", self.llvm_ir)?;
        writeln!(&mut ir, "  ret void")?;
        writeln!(&mut ir, "}}")?;
        
        Ok(ir)
    }
}

/// Enhanced compile function that handles CFG properly
pub fn compile_guppylang_json_with_cfg_support(json: &Value) -> Result<String, SeleneError> {
    log::warn!("ENTERING CFG-AWARE COMPILER - This message should appear if CFG support is working");
    
    // Get the first module
    let modules = json.get("modules")
        .and_then(|m| m.as_array())
        .ok_or_else(|| SeleneError::HugrError("No modules array found".to_string()))?;
        
    let first_module = modules.first()
        .ok_or_else(|| SeleneError::HugrError("No modules in array".to_string()))?;
    
    // Get the nodes and edges arrays
    let nodes = first_module.get("nodes")
        .and_then(|n| n.as_array())
        .ok_or_else(|| SeleneError::HugrError("No nodes array found".to_string()))?;
    
    let empty_edges = Vec::new();
    let edges = first_module.get("edges")
        .and_then(|e| e.as_array())
        .unwrap_or(&empty_edges);
    
    // Create enhanced compiler instance
    let mut compiler = CFGAwareCompiler::new();
    
    // Find all CFG nodes and process them
    let mut found_cfg = false;
    for (node_id, node) in nodes.iter().enumerate() {
        if let Some(op) = node.get("op").and_then(|o| o.as_str()) {
            if op == "CFG" {
                found_cfg = true;
                compiler.process_cfg(nodes, node_id, edges)?;
            }
        }
    }
    
    if !found_cfg {
        log::warn!("No CFG found, falling back to basic compilation");
        // Fall back to existing implementation
        return crate::hugr_to_llvm::compile_guppylang_json_to_llvm(json);
    }
    
    // Generate the final LLVM IR
    compiler.generate_llvm_ir()
}