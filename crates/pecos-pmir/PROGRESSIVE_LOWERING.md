# Progressive Lowering in PMIR

## Overview

PMIR supports progressive lowering - gradually transforming high-level quantum programs into low-level machine instructions. Each stage preserves correctness while losing human readability.

## Lowering Stages

### 1. PHIR-JSON (User Level)
**Purpose**: Human-readable, high-level quantum programs
**Readability**: Excellent - designed for humans to write and understand

```json
{
  "ops": [
    {"qop": "PrepareLogicalPlus", "qubit": "logical_q"},
    {"qop": "LogicalCNOT", "control": "logical_q1", "target": "logical_q2"},
    {"qop": "ErrorCorrect", "qubits": ["logical_q1", "logical_q2"]}
  ]
}
```

### 2. PMIR High-Level (After Parsing)
**Purpose**: Resolved names, explicit structure, but maintains algorithmic clarity
**Readability**: Good - quantum algorithms still recognizable

```mlir
module @quantum_program {
  func @main() {
    %logical_q1 = qec.allocate_logical(code="steane")
    %logical_q2 = qec.allocate_logical(code="steane")
    
    qec.prepare_plus(%logical_q1)
    qec.logical_cnot(%logical_q1, %logical_q2)
    qec.error_correct(%logical_q1, %logical_q2)
  }
}
```

### 3. PMIR Mid-Level (After Protocol Expansion)
**Purpose**: Protocols expanded to constituent operations
**Readability**: Moderate - can follow with effort

```mlir
region @error_correct {
  ^measure_syndromes:
    %x_syndromes = qec.measure_stabilizers(%data, type="X")
    %z_syndromes = qec.measure_stabilizers(%data, type="Z")
    
  ^decode:
    %x_errors = qec.decode_syndrome(%x_syndromes, decoder="lookup")
    %z_errors = qec.decode_syndrome(%z_syndromes, decoder="lookup")
    
  ^correct:
    qec.apply_pauli_corrections(%data, %x_errors, %z_errors)
}
```

### 4. PMIR Low-Level (After QEC Lowering)
**Purpose**: Physical qubit operations, no logical abstraction
**Readability**: Poor - hundreds of gates, lost high-level structure

```mlir
// Syndrome extraction expanded to physical gates
%a0 = quantum.alloc()
%a1 = quantum.alloc()
quantum.h(%a0)
quantum.cx(%d0, %a0)
quantum.cx(%d1, %a0)
quantum.cx(%d2, %a0)
quantum.h(%a0)
%s0 = quantum.measure(%a0)
// ... 50+ more operations per round
```

### 5. PMIR Machine-Level (After Hardware Mapping)
**Purpose**: Hardware-specific operations with timing
**Readability**: Very poor - hardware details dominate

```mlir
machine.move_to_zone(%q0, zone="interaction")
machine.wait(150) // ns
machine.calibrated_gate(id="cz_q0_q1_v3", power=0.97)
machine.dynamic_decoupling(%q2, sequence="XY4", duration=500)
machine.transport(%q0, path=[z1, z2, z3], duration=1200)
```

### 6. LLVM IR (Final Target)
**Purpose**: Executable code
**Readability**: Essentially unreadable for quantum algorithms

```llvm
%1 = call i8* @__quantum__rt__qubit_allocate()
%2 = call i8* @__quantum__rt__qubit_allocate()
call void @__quantum__qis__h__body(i8* %1)
call void @__quantum__qis__cnot__body(i8* %1, i8* %2)
%3 = call i8* @__quantum__rt__result_get_zero()
call void @__quantum__qis__mz__body(i8* %1, i8* %3)
```

## Preserving Debuggability

Even as readability decreases, we preserve debugging information:

### 1. Source Locations
```mlir
quantum.h(%q0) {phir.source = "line 5, col 3"}
```

### 2. Protocol Attributes
```mlir
// Even after lowering, we know this came from a logical operation
quantum.cx(%p1, %p4) {
  qec.original_op = "logical_cnot",
  qec.logical_qubit = 0,
  qec.physical_index = 4
}
```

### 3. Hierarchical Debug Info
```mlir
region {phir.original = "ErrorCorrect", phir.level = "protocol"} {
  region {phir.original = "SyndromeExtraction", phir.level = "sub-protocol"} {
    // Physical operations retain context
  }
}
```

## Best Practices for Lowering

### 1. Preserve High-Level Intent
Always attach attributes indicating the original high-level operation:
```rust
lowered_op.attributes["lowered_from"] = "LogicalCNOT";
lowered_op.attributes["protocol.step"] = "transversal_application";
```

### 2. Group Related Operations
Even after lowering, maintain region structure:
```rust
Region::new("syndrome_round_1")
    .with_attr("qec.round", 1)
    .with_attr("qec.purpose", "X_stabilizer_measurement")
```

### 3. Progressive Validation
Each level should be validatable:
- High: Validate logical operations
- Mid: Validate protocol correctness  
- Low: Validate gate sequences
- Machine: Validate timing constraints

## Viewing Different Levels

Users typically want to see the highest level that's relevant:

```rust
// Print at different verbosity levels
pmir.print(Level::UserFriendly);    // PHIR-JSON style
pmir.print(Level::Algorithmic);      // High-level PMIR
pmir.print(Level::Protocol);         // Expanded protocols
pmir.print(Level::Physical);         // Physical gates
pmir.print(Level::Machine);          // Hardware operations
```

## Conclusion

Progressive lowering is essential for compilation, but each level serves different audiences:

- **Researchers**: Want high-level protocol view
- **Algorithm developers**: Need to see logical operations
- **Hardware engineers**: Need machine-level details
- **Debuggers**: Need to trace from high to low level

PHIR-JSON represents the "user interface" to this lowering chain - the level where quantum algorithms are still clearly expressed and understood.