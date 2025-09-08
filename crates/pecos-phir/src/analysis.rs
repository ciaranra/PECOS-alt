/*!
Analysis infrastructure for PHIR

This module provides various analyses including dominance, use-def chains,
and other dataflow analyses that are essential for optimizations.
*/

use crate::ops::SSAValue;
use crate::phir::{BlockRef, Function, Region};
use crate::traits::OperationInterface;
use std::collections::{HashMap, HashSet};

/// Dominance information for a function
#[allow(dead_code)]
pub struct DominanceInfo {
    /// Maps each block to its immediate dominator
    idom: HashMap<BlockRef, BlockRef>,
    /// Maps each block to the set of blocks it dominates
    dominates: HashMap<BlockRef, HashSet<BlockRef>>,
    /// Dominance tree children
    dom_tree: HashMap<BlockRef, Vec<BlockRef>>,
}

impl DominanceInfo {
    /// Compute dominance information for a region
    #[must_use]
    pub fn compute(region: &Region) -> Self {
        let mut info = Self {
            idom: HashMap::new(),
            dominates: HashMap::new(),
            dom_tree: HashMap::new(),
        };

        // TODO: Implement proper dominance algorithm
        // For now, just mark entry block as dominating all others
        if let Some(_entry) = region.blocks.first() {
            let entry_ref = BlockRef::Index(0);
            info.dominates.insert(entry_ref.clone(), HashSet::new());

            for (idx, _) in region.blocks.iter().enumerate().skip(1) {
                let block_ref = BlockRef::Index(idx);
                info.idom.insert(block_ref.clone(), entry_ref.clone());
                if let Some(entry_dominates) = info.dominates.get_mut(&entry_ref) {
                    entry_dominates.insert(block_ref);
                }
            }
        }

        info
    }

    /// Check if block A dominates block B
    #[must_use]
    pub fn dominates(&self, a: &BlockRef, b: &BlockRef) -> bool {
        if a == b {
            return true;
        }
        self.dominates.get(a).is_some_and(|set| set.contains(b))
    }

    /// Get immediate dominator of a block
    #[must_use]
    pub fn idom(&self, block: &BlockRef) -> Option<&BlockRef> {
        self.idom.get(block)
    }
}

/// Use-def chain information
pub struct UseDefInfo {
    /// Maps SSA values to their defining instruction
    definitions: HashMap<SSAValue, InstructionRef>,
    /// Maps SSA values to all instructions that use them
    uses: HashMap<SSAValue, Vec<InstructionRef>>,
    /// Maps instructions to the values they define
    inst_defs: HashMap<InstructionRef, Vec<SSAValue>>,
    /// Maps instructions to the values they use
    inst_uses: HashMap<InstructionRef, Vec<SSAValue>>,
}

/// Reference to an instruction within a function
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstructionRef {
    pub region_idx: usize,
    pub block_idx: usize,
    pub inst_idx: usize,
}

impl UseDefInfo {
    /// Build use-def chains for a function
    #[must_use]
    pub fn compute(function: &Function) -> Self {
        let mut info = Self {
            definitions: HashMap::new(),
            uses: HashMap::new(),
            inst_defs: HashMap::new(),
            inst_uses: HashMap::new(),
        };

        // Scan all instructions
        for (region_idx, region) in function.regions().iter().enumerate() {
            for (block_idx, block) in region.blocks.iter().enumerate() {
                // Block arguments are definitions
                for arg in &block.arguments {
                    let inst_ref = InstructionRef {
                        region_idx,
                        block_idx,
                        inst_idx: usize::MAX, // Special marker for block arguments
                    };
                    info.definitions.insert(arg.value, inst_ref.clone());
                    info.inst_defs.entry(inst_ref).or_default().push(arg.value);
                }

                // Process instructions
                for (inst_idx, inst) in block.operations.iter().enumerate() {
                    let inst_ref = InstructionRef {
                        region_idx,
                        block_idx,
                        inst_idx,
                    };

                    // Record definitions
                    for result in &inst.results {
                        info.definitions.insert(*result, inst_ref.clone());
                        info.inst_defs
                            .entry(inst_ref.clone())
                            .or_default()
                            .push(*result);
                    }

                    // Record uses
                    for operand in &inst.operands {
                        info.uses
                            .entry(*operand)
                            .or_default()
                            .push(inst_ref.clone());
                        info.inst_uses
                            .entry(inst_ref.clone())
                            .or_default()
                            .push(*operand);
                    }
                }
            }
        }

        info
    }

    /// Get the instruction that defines a value
    #[must_use]
    pub fn get_definition(&self, value: &SSAValue) -> Option<&InstructionRef> {
        self.definitions.get(value)
    }

    /// Get all instructions that use a value
    #[must_use]
    pub fn get_uses(&self, value: &SSAValue) -> Option<&Vec<InstructionRef>> {
        self.uses.get(value)
    }

    /// Check if a value has any uses
    #[must_use]
    pub fn has_uses(&self, value: &SSAValue) -> bool {
        self.uses.get(value).is_some_and(|v| !v.is_empty())
    }

    /// Get all values defined by an instruction
    #[must_use]
    pub fn get_instruction_defs(&self, inst: &InstructionRef) -> Option<&Vec<SSAValue>> {
        self.inst_defs.get(inst)
    }

    /// Get all values used by an instruction
    #[must_use]
    pub fn get_instruction_uses(&self, inst: &InstructionRef) -> Option<&Vec<SSAValue>> {
        self.inst_uses.get(inst)
    }
}

/// Liveness analysis information
pub struct LivenessInfo {
    /// Live-in sets for each block
    live_in: HashMap<BlockRef, HashSet<SSAValue>>,
    /// Live-out sets for each block
    live_out: HashMap<BlockRef, HashSet<SSAValue>>,
}

impl LivenessInfo {
    /// Compute liveness information for a region
    #[must_use]
    pub fn compute(region: &Region, _use_def: &UseDefInfo) -> Self {
        let mut info = Self {
            live_in: HashMap::new(),
            live_out: HashMap::new(),
        };

        // Initialize empty sets
        for (idx, _) in region.blocks.iter().enumerate() {
            let block_ref = BlockRef::Index(idx);
            info.live_in.insert(block_ref.clone(), HashSet::new());
            info.live_out.insert(block_ref.clone(), HashSet::new());
        }

        // TODO: Implement proper liveness analysis
        // This requires iterating until fixpoint

        info
    }

    /// Check if a value is live at the start of a block
    #[must_use]
    pub fn is_live_in(&self, block: &BlockRef, value: &SSAValue) -> bool {
        self.live_in
            .get(block)
            .is_some_and(|set| set.contains(value))
    }

    /// Check if a value is live at the end of a block
    #[must_use]
    pub fn is_live_out(&self, block: &BlockRef, value: &SSAValue) -> bool {
        self.live_out
            .get(block)
            .is_some_and(|set| set.contains(value))
    }
}

/// Dead code analysis
pub struct DeadCodeInfo {
    /// Set of instructions that are dead (can be eliminated)
    dead_instructions: HashSet<InstructionRef>,
}

impl DeadCodeInfo {
    /// Identify dead code in a function
    #[must_use]
    pub fn compute(function: &Function, use_def: &UseDefInfo) -> Self {
        let mut info = Self {
            dead_instructions: HashSet::new(),
        };

        // Find instructions whose results are never used
        for (region_idx, region) in function.regions().iter().enumerate() {
            for (block_idx, block) in region.blocks.iter().enumerate() {
                for (inst_idx, inst) in block.operations.iter().enumerate() {
                    let inst_ref = InstructionRef {
                        region_idx,
                        block_idx,
                        inst_idx,
                    };

                    // Check if instruction can be eliminated
                    if inst.is_dead_if_unused() {
                        // Check if any results are used
                        let all_dead = inst.results.iter().all(|result| !use_def.has_uses(result));

                        if all_dead && !inst.results.is_empty() {
                            info.dead_instructions.insert(inst_ref);
                        }
                    }
                }
            }
        }

        info
    }

    /// Check if an instruction is dead
    #[must_use]
    pub fn is_dead(&self, inst: &InstructionRef) -> bool {
        self.dead_instructions.contains(inst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phir::{Block, Function, Region};
    use crate::region_kinds::RegionKind;
    use crate::types::FunctionType;

    #[test]
    fn test_dominance_info() {
        let mut region = Region::new(RegionKind::SSACFG);
        region.add_block(Block::new(Some("entry".to_string())));
        region.add_block(Block::new(Some("bb1".to_string())));
        region.add_block(Block::new(Some("bb2".to_string())));

        let dom_info = DominanceInfo::compute(&region);

        let entry = BlockRef::Index(0);
        let bb1 = BlockRef::Index(1);
        let bb2 = BlockRef::Index(2);

        assert!(dom_info.dominates(&entry, &bb1));
        assert!(dom_info.dominates(&entry, &bb2));
        assert!(!dom_info.dominates(&bb1, &bb2));
    }

    #[test]
    fn test_use_def_info() {
        let function = Function::new_with_visibility(
            "test",
            FunctionType::default(),
            crate::phir::Visibility::Private,
        );

        let use_def = UseDefInfo::compute(&function);

        // Basic test - should have empty maps for empty function
        assert!(use_def.definitions.is_empty());
        assert!(use_def.uses.is_empty());
    }
}
