# Boxing and Protocol Composition in PHIR

This document explains how PHIR uses MLIR's native structure to achieve the boxing/protocol approach for quantum computing, similar to the assembly macro pattern used in PECOS's Python side (slr/ and qeclib/).

## Overview

The boxing approach allows us to:
- Create reusable quantum protocols as composable units
- Preserve semantic context through attributes/metadata
- Enable optimization passes to understand protocol boundaries
- Maintain flexibility for different QEC codes and paradigms

## MLIR's Natural Boxing Hierarchy

```
Module
├── Function (Protocol/Macro)
│   ├── Region (Isolated Scope)
│   │   ├── Block (Protocol Step)
│   │   │   └── Operation (Atomic Action)
```

### 1. Functions as Protocols

Functions serve as reusable protocols or "assembly macros":

```rust
// Define a protocol as a function
let mut syndrome_protocol = Function::new(
    "x_syndrome_extraction",
    signature,
    Visibility::Public
);

// Tag it as a protocol with metadata
syndrome_protocol.attributes = AttributeBuilder::new()
    .with_tag("qec_protocol")
    .with_attr("syndrome_type", "X")
    .build();
```

### 2. Regions for Isolation

Regions provide natural isolation with clear interfaces:

```rust
// Each region has its own scope and execution semantics
let mut quantum_region = Region::new(RegionKind::PureQuantum);
let mut classical_region = Region::new(RegionKind::PureClassical);
```

### 3. Blocks as Protocol Steps

Blocks represent individual steps within a protocol:

```rust
let mut init_block = Block::new(Some("init_ancillas"));
init_block.attributes.insert(
    "protocol_step",
    AttributeValue::String("ancilla_preparation")
);
```

### 4. Composition through Function Calls

Protocols compose naturally through function calls:

```rust
// Surface code cycle composed from protocol calls
let x_syndrome_call = Instruction::new(
    Operation::ControlFlow(ControlFlowOp::Call(FunctionCall {
        name: "x_syndrome_extraction",
        args: vec![data_qubits, ancilla_qubits],
    }))
);
```

## Example: QEC Protocol Library

```mlir
module @qec_protocols {
  // Each function is a reusable protocol
  func @x_syndrome_extraction(...) -> ... attributes {
    qec_protocol,
    syndrome_type = "X"
  } {
    ^init_ancillas: attributes {protocol_step = "ancilla_preparation"}
      // Reset ancillas
      br ^entangle
      
    ^entangle: attributes {protocol_step = "stabilizer_entangling", can_parallelize}
      // CNOT gates
      br ^measure
      
    ^measure: attributes {protocol_step = "ancilla_measurement"}
      // Measure ancillas
      return %syndrome
  }
  
  // Composite protocol using other protocols
  func @surface_code_cycle(...) attributes {composite_protocol} {
    %x_syndrome = call @x_syndrome_extraction(...)
    %z_syndrome = call @z_syndrome_extraction(...)
    %corrections = call @decode_syndrome(%x_syndrome, %z_syndrome)
    call @apply_corrections(%data, %corrections)
  }
}
```

## Benefits

1. **No Special Constructs Needed**: MLIR's existing structure provides everything we need
2. **Semantic Preservation**: Attributes on all levels preserve protocol context
3. **Optimization-Friendly**: Clear boundaries enable protocol-aware optimizations
4. **Composable**: Protocols compose like assembly macros
5. **Flexible**: No hard-coded QEC assumptions, works for any quantum algorithm

## Attribute Tags for Common Protocols

```rust
pub mod tags {
    // QEC protocols
    pub const SYNDROME_EXTRACTION: &str = "syndrome_extraction";
    pub const DECODER: &str = "decoder";
    pub const LOGICAL_GATE: &str = "logical_gate";
    
    // Quantum algorithms
    pub const QFT: &str = "qft";
    pub const GROVER_ORACLE: &str = "grover_oracle";
    pub const PHASE_ESTIMATION: &str = "phase_estimation";
    
    // General tags
    pub const PROTOCOL: &str = "protocol";
    pub const COMPOSITE_PROTOCOL: &str = "composite_protocol";
    pub const CAN_PARALLELIZE: &str = "can_parallelize";
}
```

## Integration with Optimization Passes

Optimization passes can query attributes to understand protocol boundaries:

```rust
// Example: QEC-aware optimization pass
if func.attributes.has_tag("qec_protocol") {
    // Apply QEC-specific optimizations
    if let Some(syndrome_type) = func.attributes.get("syndrome_type") {
        // Optimize based on syndrome type
    }
}
```

This approach maintains the flexibility you requested while providing clear structure for experimentation with different QEC codes and quantum algorithms.