# Translating from SLR/qeclib to PHIR

This guide explains how to translate quantum programs written in PECOS's SLR (Simple Logical Representation) and qeclib to PHIR.

## Core Concept Mapping

### 1. Basic Structures

| SLR/qeclib | PHIR | Notes |
|------------|------|-------|
| `Block` | `Block` or `Region` | Direct mapping for operation sequences |
| `QReg[n]` | `Vec<SSAValue>` with `Type::Qubit` | Quantum register to SSA values |
| `CReg[n]` | `Vec<SSAValue>` with `Type::Bit` | Classical register to SSA values |
| `Qubit` | `SSAValue` with `Type::Qubit` | Single qubit |
| `Bit` | `SSAValue` with `Type::Bit` | Single classical bit |

### 2. Operations

| SLR/qeclib | PHIR |
|------------|------|
| `qubit.X(q)` | `Operation::Quantum(QuantumOp::X)` |
| `qubit.H(q)` | `Operation::Quantum(QuantumOp::H)` |
| `qubit.CNOT(q1, q2)` | `Operation::Quantum(QuantumOp::CNOT)` |
| `qubit.Measure(q, c)` | `Operation::Quantum(QuantumOp::Measure)` |
| `Comment("text")` | Custom operation with comment attribute |

### 3. Control Flow

| SLR/qeclib | PHIR |
|------------|------|
| `If(condition, block)` | `ParsingOp::IfElse` → lowered to branches |
| `Repeat(n, block)` | `ParsingOp::ForLoop` → lowered to loop |
| Function calls | `ParsingOp::UnresolvedCall` → resolved to `Call` |

## Translation Examples

### Example 1: Simple Quantum Circuit

**SLR/qeclib:**
```python
Block(
    Comment("Bell pair preparation"),
    qubit.H(q[0]),
    qubit.CNOT(q[0], q[1]),
    qubit.Measure(q[0], c[0]),
    qubit.Measure(q[1], c[1])
)
```

**PHIR:**
```rust
use pecos_pmir::slr_helpers::*;

Block::new(Some("bell_pair"))
    .with_instruction(comment("Bell pair preparation"))
    .with_instruction(quantum_h(q[0]))
    .with_instruction(quantum_cx(q[0], q[1]))
    .with_instruction(measure(q[0]).0)
    .with_instruction(measure(q[1]).0)
```

### Example 2: Logical Gate (Steane Code)

**SLR/qeclib (from qeclib/steane/gates_sq/paulis.py):**
```python
class X(Block):
    def __init__(self, q: QReg):
        super().__init__(
            Comment("Logical X"),
            qubit.X(q[4]),
            qubit.X(q[5]),
            qubit.X(q[6]),
        )
```

**PHIR:**
```rust
fn logical_x_steane(data_qubits: &[SSAValue]) -> Block {
    Block::new(Some("logical_x"))
        .with_instruction(comment("Logical X"))
        .with_instruction(quantum_x(data_qubits[4]))
        .with_instruction(quantum_x(data_qubits[5]))
        .with_instruction(quantum_x(data_qubits[6]))
        .with_attr("qec.logical_gate", AttributeValue::String("X".to_string()))
        .with_attr("qec.code", AttributeValue::String("steane".to_string()))
}
```

### Example 3: QEC Protocol Composition

**SLR/qeclib pattern:**
```python
class QECCycle(Block):
    def __init__(self, data, ancilla):
        super().__init__(
            SyndromeExtraction(data, ancilla),
            DecodeSyndrome(),
            ApplyCorrections(data)
        )
```

**PHIR:**
```rust
fn qec_cycle(data_qubits: &[SSAValue], ancilla_qubits: &[SSAValue]) -> Region {
    Region::new(RegionKind::SSACFG)
        .with_block(syndrome_extraction_block(data_qubits, ancilla_qubits))
        .with_block(decode_syndrome_block())
        .with_block(apply_corrections_block(data_qubits))
        .with_attr("protocol", AttributeValue::String("qec_cycle".to_string()))
}
```

## Key Differences and Considerations

### 1. SSA Form
PHIR uses SSA (Single Static Assignment) form, so each value is defined exactly once:

```rust
// SLR might reuse variables
// q = H(q)
// q = X(q)

// PHIR creates new SSA values
let q1 = quantum_h(q0);
let q2 = quantum_x(q1);
```

### 2. Explicit Type Information
PHIR requires explicit type information for all operations:

```rust
Instruction::new(
    Operation::Quantum(QuantumOp::H),
    vec![input_qubit],      // operands
    vec![output_qubit],     // results
    vec![Type::Qubit],      // result types
)
```

### 3. Attributes for Metadata
Where SLR/qeclib uses class inheritance and naming conventions, PHIR uses attributes:

```rust
// Instead of class LogicalX(Block):
block.with_attr("qec.logical_gate", "X")
     .with_attr("qec.code", "steane")
```

### 4. Progressive Lowering
PHIR supports high-level operations that get lowered progressively:

```rust
// High-level (parsing phase)
ParsingOp::ForLoop { ... }

// Mid-level (after lowering)
ControlFlowOp::Loop { ... }

// Low-level (for execution)
Branch and phi operations
```

## Using the Helper Functions

The `slr_helpers` module provides convenience functions to make translation easier:

```rust
use pecos_pmir::slr_helpers::*;

// Direct operation helpers
let h_gate = quantum_h(qubit);
let x_gate = quantum_x(qubit);
let (measure_op, result) = measure(qubit);

// Logical gate helpers (Steane code)
let logical_x = logical_x_steane(&data_qubits);
let logical_z = logical_z_steane(&data_qubits);

// Protocol helpers
let syndrome = syndrome_extraction(&data_qubits, &ancilla_qubits);
let qec = qec_cycle(&data_qubits, &ancilla_qubits);
```

## Best Practices

1. **Preserve Protocol Structure**: Use regions and blocks to mirror the hierarchical structure of qeclib protocols

2. **Use Attributes Liberally**: Tag operations and regions with protocol information that optimization passes can use

3. **Keep It Simple**: Start with basic operations and compose them - don't create complex "kitchen sink" operations

4. **Document Intent**: Use comment operations and attributes to preserve the high-level intent from SLR/qeclib

5. **Progressive Enhancement**: Start with a basic translation, then add attributes for optimization hints, error models, etc.

## Future Enhancements

As PHIR evolves, we plan to add:

1. **Automated Translation Tool**: A Python tool that can automatically translate SLR/qeclib code to PHIR
2. **Protocol Library**: Pre-built PHIR implementations of common qeclib protocols
3. **Verification**: Tools to verify that PHIR translations preserve the semantics of the original SLR/qeclib code
4. **Round-trip Support**: Ability to generate SLR code from PHIR for testing and validation