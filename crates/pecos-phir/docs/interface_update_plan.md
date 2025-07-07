# Plan to Update "Boxing" Terminology to MLIR Interfaces

## Overview

The codebase currently uses "boxing" terminology to describe what is actually MLIR's interface approach for semantic tagging with attributes. This is confusing and should be updated to properly reflect MLIR concepts.

## Key Changes Needed

### 1. File Renames
- `boxing_example.rs` → `interface_example.rs` or `semantic_tagging_example.rs`
- `mlir_native_boxing.rs` → `mlir_native_interfaces.rs`
- `BOXING_APPROACH.md` → `INTERFACE_APPROACH.md`

### 2. Terminology Updates

Current Term | New Term | Explanation
------------|----------|-------------
"boxing" | "interface implementation" | Operations/regions implementing specific interfaces
"boxed with metadata" | "tagged with interface attributes" | Semantic metadata attached via attributes
"boxing approach" | "interface-based approach" | Using MLIR interfaces for semantic tagging
"box the region" | "attach interface attributes" | Adding semantic tags to regions/operations

### 3. Documentation Updates

Update all references in:
- ARCHITECTURE.md - Section on "Boxing and Abstract QEC Representation"
- quantum-compiler-design.md - References to boxing approach
- QEC.md - References to boxing terminology
- IMPLEMENTATION.md - Any boxing references

### 4. Code Updates

Update comments and variable names that reference "boxing" to use proper interface terminology.

## Implementation Steps

1. **Update Documentation Files** (Priority: High)
   - Start with BOXING_APPROACH.md → INTERFACE_APPROACH.md
   - Update ARCHITECTURE.md boxing section
   - Update other docs

2. **Update Examples** (Priority: High)
   - Rename and update boxing_example.rs
   - Rename and update mlir_native_boxing.rs
   - Update all comments and output messages

3. **Search for Remaining References** (Priority: Medium)
   - Grep for "box", "boxing", "boxed" in all files
   - Update any remaining references

4. **Update Tests** (Priority: Low)
   - Any tests that reference boxing terminology
   - Update test names and comments

## Key Concepts to Emphasize

1. **MLIR Interfaces**: Operations can implement interfaces that define expected behavior
2. **Semantic Tagging**: Attributes provide semantic context for optimization passes
3. **Protocol Composition**: Functions act as reusable protocols (like assembly macros)
4. **Progressive Optimization**: Generic passes work on all implementations, specialized passes optimize specific interfaces