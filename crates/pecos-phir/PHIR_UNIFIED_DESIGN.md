# PHIR: Unified Design Document

## Executive Summary

PHIR (PECOS High-level Intermediate Representation) is evolving from a JSON format to a complete quantum compiler IR. This document captures the unified design where:

- **PHIR** = The quantum compiler IR (formerly called PMIR)
- **PHIR-JSON** = Stable, human-readable serialization
- **PHIR-RON** = Serialization that closely matches PHIR structure for debugging and bridging

## Design Philosophy

### 1. Single Conceptual Model
Users learn one set of concepts (operations, regions, blocks, SSA values) that apply everywhere - from the JSON they write to the IR the compiler optimizes.

### 2. Multiple Representations
```
PHIR (in-memory IR) ←→ PHIR-JSON (human-readable) ←→ PHIR-RON (debug/bridge)
         ↑                    ↑                           ↑
    Fast, mutable      Stable, versioned          IR-mirroring, typed
```

### 3. Progressive Lowering with Appropriate Visibility
- **PHIR-JSON**: Shows high-level, human-readable operations
- **PHIR in-memory**: Contains all lowering stages
- **PHIR-RON**: Can serialize any lowering level

## Complete Serialization Suite

```
                    ┌─────────────────┐
                    │   PHIR Core     │
                    │  (In-Memory IR) │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  PHIR-JSON    │   │   PHIR-RON    │   │  PHIR-MLIR    │
│               │   │               │   │               │
│ Human-first   │   │ Debug/Bridge  │   │ MLIR standard │
│ Stable API    │   │ IR-mirroring  │   │ Tool chain    │
│ Versioned     │   │ Type-safe     │   │ Optimizable   │
└───────────────┘   └───────────────┘   └───────────────┘
```

## Architecture Overview

### PHIR Core (In-Memory IR)

**Purpose**: Efficient compiler IR for optimization and transformation

**Key Features**:
- MLIR-inspired hierarchical structure
- SSA form with use-def chains
- Progressive lowering stages
- Extensible operation/type system
- Free to evolve and optimize

**Example Structure**:
```rust
Module {
    body: Region {
        blocks: vec![Block {
            operations: vec![
                Instruction { operation: Quantum(H), ... },
                Instruction { operation: Quantum(CNOT), ... },
            ]
        }]
    }
}
```

### PHIR-JSON (External Interface)

**Purpose**: Stable, versioned, human-readable format

**Design Principles**:
1. **Readability First**: Clear, self-documenting syntax
2. **Stability**: Versioned with backward compatibility
3. **High-Level View**: Shows logical operations by default
4. **Progressive Disclosure**: Can show lowered forms when needed
5. **Tool-Friendly**: Valid JSON, parseable everywhere

**Example**:
```json
{
  "format": "PHIR/JSON",
  "version": "0.2.0",
  "metadata": {
    "description": "Bell state with error correction"
  },
  "ops": [
    {
      "qop": "PrepareLogicalPlus",
      "qubit": "logical_q",
      "attributes": {
        "qec.code": "steane",
        "qec.distance": 3
      }
    },
    {
      "qop": "LogicalH", 
      "qubit": "logical_q"
    },
    {
      "qop": "MeasureLogical",
      "qubit": "logical_q",
      "basis": "Z",
      "returns": ["result"]
    }
  ]
}
```

### PHIR-RON (Debug/Bridge Interface)

**Purpose**: Serialization that mirrors the internal IR structure for debugging and bridging between formats

**Benefits**:
- Directly mirrors internal IR structure
- Excellent for debugging and understanding IR transformations
- Bridges the gap between stable PHIR-JSON and evolving PHIR
- Type-safe with Rust enums and structures

**Example**:
```ron
PhirModule(
    name: "bell_state",
    body: Region(
        kind: SSACFG,
        blocks: [
            Block(
                operations: [
                    Instruction(
                        operation: Quantum(H),
                        operands: [SSAValue(0)],
                        results: [SSAValue(1)],
                        result_types: [Qubit],
                    ),
                ],
            ),
        ],
    ),
)
```

### PHIR-MLIR (MLIR Ecosystem Interface)

**Purpose**: Standard MLIR textual representation for compatibility with MLIR tools

**Benefits**:
- Full MLIR ecosystem compatibility
- Use MLIR passes and optimizations
- Standard tooling (mlir-opt, mlir-translate)
- Path to LLVM compilation
- Formal IR semantics

**Example**:
```mlir
module @bell_state {
  phir.func @main() -> !phir.measurement {
    %q0 = phir.alloc_qubit : !phir.qubit
    %q1 = phir.alloc_qubit : !phir.qubit
    
    %q0_h = phir.h %q0 : !phir.qubit
    %q0_out, %q1_out = phir.cnot %q0_h, %q1 : !phir.qubit, !phir.qubit
    
    %m0 = phir.measure %q0_out : !phir.qubit -> !phir.bit
    %m1 = phir.measure %q1_out : !phir.qubit -> !phir.bit
    
    %result = phir.make_measurement %m0, %m1 : !phir.bit, !phir.bit -> !phir.measurement
    phir.return %result : !phir.measurement
  }
}
```

Or using standard MLIR dialects when lowered:
```mlir
module @bell_state {
  func.func @main() -> i2 {
    %q0 = call @__quantum__rt__qubit_allocate() : () -> !llvm.ptr
    %q1 = call @__quantum__rt__qubit_allocate() : () -> !llvm.ptr
    
    call @__quantum__qis__h__body(%q0) : (!llvm.ptr) -> ()
    call @__quantum__qis__cnot__body(%q0, %q1) : (!llvm.ptr, !llvm.ptr) -> ()
    
    %m0 = call @__quantum__qis__mz__body(%q0) : (!llvm.ptr) -> i1
    %m1 = call @__quantum__qis__mz__body(%q1) : (!llvm.ptr) -> i1
    
    %result = call @__phir__pack_measurement(%m0, %m1) : (i1, i1) -> i2
    return %result : i2
  }
}
```

## Progressive Lowering and Visibility

### Lowering Stages in PHIR

1. **Source Level** (what PHIR-JSON shows)
   ```json
   {"qop": "LogicalCNOT", "control": "q1", "target": "q2"}
   ```

2. **Protocol Level** (expanded protocols)
   ```
   region "logical_cnot" {
     syndrome_extract()
     transversal_cnot()
     syndrome_extract()
     error_correct()
   }
   ```

3. **Physical Level** (hardware gates)
   ```
   // 100+ physical gates implementing the logical operation
   ```

4. **Machine Level** (with timing/transport)
   ```
   transport q[0] zone="interaction" duration=500ns
   calibrated_gate "cz_q0_q1_v3"
   ```

### Serialization at Different Levels

```rust
// Users can control what they see
module.to_json(Level::Source);      // High-level operations only
module.to_json(Level::Protocol);    // Show protocol structure
module.to_json(Level::Physical);    // Show gate decomposition
module.to_json(Level::Machine);     // Include hardware details

// Or get progressive views
module.to_json_with_expansion(2);   // Expand 2 levels deep
```

## Migration Path

### From Current PHIR v0.1
```
PHIR v0.1 JSON → Parse → PHIR IR → Serialize → PHIR v0.2 JSON
                           ↓
                    (Can also output)
                           ↓
                      PHIR-RON
```

### From PMIR (Renaming)
1. Rename `pecos-pmir` → `pecos-phir` (or merge into existing crate)
2. Update imports: `use pecos_phir::{Module, Operation, ...}`
3. Add version handling for existing PHIR v0.1 files

## Benefits of Unified Design

### For Users
- **Single mental model**: Learn once, use everywhere
- **Stable interface**: PHIR-JSON remains compatible
- **Progressive complexity**: Start simple, add detail as needed
- **Better tooling**: Same analysis works on all representations

### For Developers  
- **Flexibility**: Internal IR can evolve freely
- **Clean architecture**: Clear separation of concerns
- **Reusable code**: Serialization separate from core logic
- **Easier testing**: Can test with readable JSON

### For the Ecosystem
- **Clear story**: "PHIR is PECOS's quantum IR"
- **Multiple entry points**: JSON for humans, RON for tools
- **Standard compliance**: Can generate MLIR, LLVM IR, etc.
- **Research friendly**: Easy to experiment with new features

## Implementation Roadmap

### Phase 1: Unification (Current)
- [x] Design unified architecture
- [x] Implement PHIR-RON serialization framework
- [x] Implement PHIR-JSON serialization framework
- [ ] Rename PMIR → PHIR throughout codebase
- [ ] Add Serialize/Deserialize traits to all types

### Phase 2: Compatibility
- [ ] Parse PHIR v0.1 files into new PHIR IR
- [ ] Version detection and auto-upgrade
- [ ] Compatibility test suite
- [ ] Migration documentation

### Phase 3: Features
- [ ] Level-aware serialization
- [ ] Streaming support for large programs
- [ ] Schema generation
- [ ] Pretty-printing options

### Phase 4: Optimization
- [ ] Binary format for performance
- [ ] Incremental serialization
- [ ] Compression support
- [ ] Parallel parsing

## Design Decisions

### Why Unify?
- Reduces confusion between PMIR and PHIR
- Single conceptual model is easier to learn
- Natural evolution of PHIR from format to IR

### Why Keep JSON Separate?
- Stability for users is paramount
- Human readability requires different trade-offs
- JSON is universal, Rust IR is not

### Why Add RON?
- Natural fit for Rust's type system
- More compact than JSON
- Better for tool-to-tool communication

## Future Considerations

### Additional Formats
- **PHIR-Binary**: For performance-critical applications
- **PHIR-QIR**: Compatibility with Microsoft's QIR
- **PHIR-OpenQASM**: Export to OpenQASM 3.0

### Language Bindings
- Python: `phir.Module` class that mirrors Rust structure
- C API: For integration with other languages
- JavaScript: For web-based tools

### Tooling
- VSCode extension with PHIR-JSON syntax highlighting
- Formatter/linter for PHIR-JSON
- Visualization tools for PHIR structure
- Debugger integration

## Conclusion

By unifying PMIR and PHIR into a single conceptual model with multiple representations, we get:

1. **Simplicity**: One IR to learn and understand
2. **Flexibility**: Multiple serialization formats for different needs  
3. **Stability**: PHIR-JSON provides a stable user interface
4. **Power**: Full MLIR-style IR capabilities internally

This positions PHIR as a comprehensive quantum compiler infrastructure that can evolve with the needs of quantum computing while maintaining stability for users.