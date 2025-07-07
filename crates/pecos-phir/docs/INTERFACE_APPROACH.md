# Interface-Based Protocol Composition in PHIR

This document explains how PHIR uses MLIR's native interface system to achieve semantic tagging and protocol composition for quantum computing, similar to the assembly macro pattern used in PECOS's Python side (slr/ and qeclib/).

## Overview

The interface-based approach allows us to:
- Create reusable quantum protocols as composable units
- Preserve semantic context through attributes/metadata
- Enable optimization passes to understand protocol boundaries and interfaces
- Maintain flexibility for different QEC codes and paradigms

## MLIR's Interface System and Natural Hierarchy

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

// Tag it with interface attributes
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

## Example: QEC Protocol Library with Interfaces

```mlir
module @qec_protocols {
  // Each function implements a protocol interface
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
  
  // Composite protocol using other protocol interfaces
  func @surface_code_cycle(...) attributes {composite_protocol} {
    %x_syndrome = call @x_syndrome_extraction(...)
    %z_syndrome = call @z_syndrome_extraction(...)
    %corrections = call @decode_syndrome(%x_syndrome, %z_syndrome)
    call @apply_corrections(%data, %corrections)
  }
}
```

## Interface Implementation in PHIR

### Defining Interfaces through Attributes

```rust
// Define protocol interfaces through attributes
pub trait QECProtocol {
    fn required_attributes() -> Vec<&'static str>;
    fn validate_attributes(attrs: &Attributes) -> Result<()>;
}

// Example: Surface code syndrome extraction interface
impl QECProtocol for SurfaceCodeSyndrome {
    fn required_attributes() -> Vec<&'static str> {
        vec!["syndrome_type", "stabilizer_operators", "ancilla_layout"]
    }
}
```

### Passes Interpreting Interface Implementations

```rust
// Optimization passes check for interface implementations
pub struct SurfaceCodeOptimization;
impl Pass for SurfaceCodeOptimization {
    fn run_on_operation(&mut self, op: &Operation) -> Result<()> {
        // Check if operation implements surface code interface
        if op.get_attribute("qec.code_type") == Some("surface_code") {
            // Apply surface code specific optimizations
            self.optimize_syndrome_extraction(op)?;
            self.minimize_logical_gate_overhead(op)?;
        }
        Ok(())
    }
}
```

## Benefits

1. **No Special Constructs Needed**: MLIR's existing structure provides everything we need
2. **Semantic Preservation**: Attributes on all levels preserve protocol context and interface information
3. **Optimization-Friendly**: Clear boundaries and interfaces enable protocol-aware optimizations
4. **Composable**: Protocols compose like assembly macros
5. **Flexible**: No hard-coded QEC assumptions, works for any quantum algorithm or protocol
6. **Standards-Based**: Uses standard MLIR interface patterns

## Common Interface Tags

```rust
pub mod tags {
    // QEC protocol interfaces
    pub const SYNDROME_EXTRACTION: &str = "syndrome_extraction";
    pub const DECODER: &str = "decoder";
    pub const LOGICAL_GATE: &str = "logical_gate";
    
    // Quantum algorithm interfaces
    pub const QFT: &str = "qft";
    pub const GROVER_ORACLE: &str = "grover_oracle";
    pub const PHASE_ESTIMATION: &str = "phase_estimation";
    
    // General interface tags
    pub const PROTOCOL: &str = "protocol";
    pub const COMPOSITE_PROTOCOL: &str = "composite_protocol";
    pub const CAN_PARALLELIZE: &str = "can_parallelize";
}
```

## Integration with Optimization Passes

Optimization passes can query attributes to understand interface implementations:

```rust
// Example: QEC-aware optimization pass
if func.attributes.has_tag("qec_protocol") {
    // Apply QEC-specific optimizations based on interface
    if let Some(syndrome_type) = func.attributes.get("syndrome_type") {
        // Optimize based on specific syndrome extraction interface
        match syndrome_type.as_str() {
            "X" => self.optimize_x_syndrome_extraction(func),
            "Z" => self.optimize_z_syndrome_extraction(func),
            _ => Ok(())
        }
    }
}
```

## Interface-Based Development Workflow

1. **Define Protocol Interfaces**: Specify required attributes and behavior
2. **Implement Protocols**: Create functions/regions with proper interface attributes
3. **Compose Protocols**: Build complex algorithms from simple protocol calls
4. **Write Interface-Aware Passes**: Optimize based on interface implementations
5. **Validate Interfaces**: Ensure all required attributes are present

This approach maintains the flexibility needed for quantum computing research while providing clear structure for optimization and experimentation with different QEC codes and quantum algorithms.