# PECOS PHIR - MLIR-Inspired Quantum Compiler IR

PECOS PHIR (PECOS High-level Intermediate Representation) is an MLIR-inspired compiler infrastructure for quantum programs, providing a unified representation from parsing through execution.

## Overview

PHIR follows MLIR's design philosophy where everything is an Operation. This provides a single, hierarchical representation throughout the compilation pipeline:

```
Source → PHIR (parsing ops) → PHIR (high-level) → PHIR (low-level) → Execution
         ↓                     ↓                   ↓
         parse.unresolved_ref  quantum.h           llvm.call
         parse.for_loop        control.if          llvm.add
```

Key features:
- **Unified representation**: No separate AST - parse directly to PHIR
- **Progressive lowering**: Gradually lower from high-level to machine-level operations
- **MLIR compatibility**: Can generate MLIR text and integrate with MLIR toolchain
- **Multiple backends**: Interpreter, Rust codegen, or LLVM compilation
- **Extensible**: Add new operations and types through the dialect system

## Design Principles

PHIR follows key design principles inspired by PECOS's SLR (Simple Logical Representation):

### 1. Simple Primitives
Keep the fundamental building blocks simple and well-defined. Complex behavior emerges from composition, not from complex primitives.

```rust
// Simple operations with clear semantics
Operation::Quantum(QuantumOp::H)        // Hadamard gate
Operation::Quantum(QuantumOp::Measure)  // Measurement
Operation::Classical(ClassicalOp::Add)  // Addition
```

### 2. Natural Composition
Make it easy to combine simple operations into more complex protocols. The structure should mirror how quantum algorithms are conceptually built.

```rust
// Operations compose into blocks
let syndrome_extraction = Block::new()
    .add(measure_x_stabilizers)
    .add(measure_z_stabilizers)
    .add(decode_syndrome);

// Blocks compose into regions
let qec_cycle = Region::new()
    .add_block(syndrome_extraction)
    .add_block(apply_corrections);

// Regions compose into larger protocols
let fault_tolerant_gate = Operation::with_regions(vec![
    prepare_logical_state,
    qec_cycle,
    apply_logical_gate,
    qec_cycle,
]);
```

### 3. Mechanism, Not Policy
PHIR provides the mechanisms for representing quantum programs. Users define the policies (specific QEC schemes, optimization strategies) through attributes and passes.

```rust
// PHIR provides the mechanism (operations, regions, attributes)
region.attributes["protocol.type"] = "surface_code_cycle";

// Users/passes define the policy (how to optimize surface codes)
if region.get_attr("protocol.type") == "surface_code_cycle" {
    apply_surface_code_optimizations(&mut region);
}
```

### 4. Progressive Enhancement
Start with simple programs and progressively add complexity only as needed. Attributes and metadata can be added incrementally.

```rust
// Start simple
let h_gate = Operation::Quantum(QuantumOp::H);

// Add metadata as understanding grows
h_gate.attributes["noise.model"] = "depolarizing";
h_gate.attributes["pulse.calibration"] = "optimal_h_pulse_v2";

// Complex protocols built from enhanced simple operations
let logical_h = region.with_attr("qec.logical_gate", "H");
```

These principles ensure PHIR remains flexible enough for research while providing structure for production use.

## Architecture

### Core Structure

PHIR follows MLIR's hierarchical structure:

```
Operation → Region(s) → Block(s) → Operation(s) → ...
```

- **Operations**: Everything is an operation (modules, functions, quantum gates, control flow)
- **Regions**: Contain blocks with specific execution semantics (SSACFG or Graph)
- **Blocks**: Sequences of operations with optional terminator
- **SSA Values**: Single Static Assignment for all values

### Key Components

1. **Core IR** (`pmir.rs`)
   - Defines Region, Block, and Instruction structures
   - Implements SSA value management

2. **Operations** (`ops.rs`)
   - Builtin: Module, Function, Return
   - Quantum: Gates, measurements, state prep
   - Classical: Arithmetic, logic, comparisons
   - Control flow: Branches, loops, calls
   - Parsing: Unresolved refs, type inference

3. **Type System** (`types.rs`)
   - Quantum types: Qubit, quantum registers
   - Classical types: Int, Float, Bool, Arrays
   - Function types with variadic support

4. **Parsing Operations** (`parsing_ops.rs`)
   - UnresolvedCall/Ref for forward references
   - ForLoop/IfElse for high-level control flow
   - InferType for type inference
   - ImplicitCast for type coercion

5. **Analysis** (`analysis.rs`)
   - Dominance analysis
   - Use-def chains
   - Liveness analysis
   - Dead code detection

## Usage

### Direct PMIR Construction

```rust
use pecos_pmir::{Module, Function, Instruction, Operation};
use pecos_pmir::ops::{QuantumOp, SSAValue};
use pecos_pmir::types::{Type, FunctionType};

// Create a module
let mut module = Module::new("quantum_program");

// Create a function
let mut func = Function::new("bell_pair", FunctionType {
    inputs: vec![],
    outputs: vec![Type::Bit, Type::Bit],
    variadic: false,
});

// Add quantum operations
let q0 = SSAValue::new(1);
let hadamard = Instruction::new(
    Operation::Quantum(QuantumOp::H),
    vec![],
    vec![q0],
    vec![Type::Qubit],
);

// Add to function body
func.entry_region_mut()
    .unwrap()
    .entry_block_mut()
    .unwrap()
    .add_instruction(hadamard);

module.add_function(func);
```

### Parsing to PMIR

```rust
use pecos_pmir::{Pipeline, PMIRConfig, InputFormat};

let config = PMIRConfig {
    debug: true,
    optimization_level: 2,
    execution_strategy: None,
    target_triple: None,
};

let pipeline = Pipeline::new(config);

// Parse directly to PMIR (no AST!)
let result = pipeline.compile_and_execute::<i32>(
    source_code, 
    InputFormat::HUGR
)?;
```

### Progressive Lowering

```rust
// Start with high-level parsing operations
let unresolved_call = Operation::Parsing(ParsingOp::UnresolvedCall(...));

// Lower to resolved operations
let resolved_call = Operation::ControlFlow(ControlFlowOp::Call(...));

// Further lower to LLVM operations
let llvm_call = Operation::Custom(CustomOp {
    dialect: "llvm".to_string(),
    name: "call".to_string(),
    ...
});
```

## Key Design Decisions

### Why No Separate AST?

PMIR follows MLIR's approach of using a single IR throughout compilation:

1. **Simplicity**: One representation to maintain, debug, and optimize
2. **Power**: MLIR's hierarchical structure can represent anything an AST can
3. **Efficiency**: No conversion overhead between representations
4. **Flexibility**: Mix high-level and low-level operations in the same module

### Parsing Strategy

Instead of parsing to an AST, we parse directly to PMIR using special parsing operations:

1. **Multi-pass parsing**: Collect declarations → parse with placeholders → resolve → lower
2. **SSA construction**: Build SSA form incrementally with phi nodes at merge points
3. **Type inference**: Use type variables and constraints, resolve in a separate pass
4. **Symbol resolution**: Hierarchical symbol tables that mirror region structure

### Interface-Based Protocols

PMIR embraces an abstract, extensible approach to quantum error correction and emerging quantum computing paradigms through MLIR's interface system - using attributes to indicate which interfaces an operation or region implements:

```rust
// Tag a region as containing a QFT algorithm
region.attributes.insert("quantum.algorithm", "QFT");
region.attributes.insert("quantum.parallelizable", true);

// QEC interface implementation - abstract representation allows multiple QEC schemes
region.attributes.insert("qec.syndrome_extraction", true);
region.attributes.insert("qec.code_type", "surface_code");
region.attributes.insert("qec.distance", 5);

// Protocol interfaces - passes can interpret based on capabilities
operation.attributes.insert("protocol.interface", "stabilizer_measurement");
operation.attributes.insert("protocol.fault_tolerant", true);
```

This approach provides several key benefits:

1. **Future-proof**: New QEC schemes and quantum algorithms can be added without changing core IR
2. **Multiple paradigms**: Surface codes, color codes, LDPC codes can coexist
3. **Progressive optimization**: Generic passes can ignore QEC details, specialized passes can optimize
4. **Research friendly**: Easy to experiment with new protocols and techniques
5. **Standard compliant**: Uses MLIR's standard attribute mechanism

## Execution Strategies

PMIR supports multiple execution backends:

1. **Interpreter**: Direct execution for debugging and small programs
2. **Rust Codegen**: Generate optimized Rust code
3. **MLIR/LLVM**: Lower to MLIR text → LLVM IR → native code
4. **Adaptive**: Automatically choose based on program characteristics

## Future Work

- **Parser implementations** for HUGR, PHIR, OpenQASM, etc.
- **Optimization passes**: Quantum-specific and classical optimizations
- **Direct simulator integration**: Connect to PECOS quantum simulators
- **Advanced type system**: Linear types for quantum values
- **Dialect extensions**: QEC, pulse-level control, chemistry