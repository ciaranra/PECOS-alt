# PHIR Architecture

## Overview

PHIR (PECOS High-level Intermediate Representation) is an MLIR-inspired quantum compiler infrastructure that provides a unified representation from parsing through execution. Unlike traditional compilers with separate AST and IR phases, PHIR leverages MLIR's hierarchical structure to serve as both AST and IR.

## Design Goals

1. **Unified Representation**: One IR from parsing to execution, following MLIR's philosophy
2. **Progressive Lowering**: Gradually transform high-level operations to machine-level operations
3. **Multiple Backends**: Support interpretation, native code generation, and LLVM compilation
4. **Extensibility**: Easy addition of new operations and dialects
5. **Quantum-Native**: First-class support for quantum operations and error correction

## Philosophical Foundation

PHIR's architecture is deeply influenced by PECOS's SLR (Simple Logical Representation), which proved that complex quantum protocols can be built from simple, composable primitives. This philosophy shapes PHIR at every level:

- **Simple Operations**: Each operation does one thing well (H gate, measurement, branch)
- **Natural Composition**: Operations → Blocks → Regions → Modules (mirrors how we think about quantum algorithms)
- **Progressive Complexity**: Start with basic gates, add QEC protocols through composition and attributes
- **Mechanism over Policy**: PHIR provides the structure; users define the quantum protocols

This approach ensures that PHIR can represent everything from simple quantum circuits to complex fault-tolerant algorithms without compromising simplicity or performance.

## Architecture

The PHIR pipeline uses progressive lowering through the same IR structure:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Source    │────▶│ PHIR (Parse)│────▶│ PHIR (High) │────▶│ PHIR (Low)  │
│   (HUGR,    │     │   parse.*   │     │  quantum.*  │     │   llvm.*    │
│   OpenQASM) │     │             │     │  control.*  │     │   machine.* │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │                    │                    │
                           ▼                    ▼                    ▼
                    ┌─────────────┐      ┌─────────────┐     ┌─────────────┐
                    │Type Inference│     │Optimization │     │ Execution:  │
                    │Symbol Resolve│     │  Passes     │     │ • Interpret │
                    └─────────────┘      └─────────────┘     │ • Rust Gen  │
                                                              │ • LLVM      │
                                                              └─────────────┘
```

## Core Components

### 1. Hierarchical Structure

PHIR follows MLIR's recursive structure:

```rust
Operation → Region(s) → Block(s) → Operation(s) → ...
```

- **Operation**: Everything is an operation (Module, Function, quantum gates, loops, etc.)
- **Region**: A collection of blocks with specific execution semantics
- **Block**: A sequence of operations ending with an optional terminator
- **SSA Values**: All values follow Single Static Assignment form

### 2. Operation Categories

#### Parsing Operations (`parsing_ops.rs`)
- `UnresolvedCall`: Function calls before name resolution
- `UnresolvedRef`: Variable references before resolution
- `ForLoop`/`IfElse`: High-level control flow
- `InferType`: Type variables for inference
- `ImplicitCast`: Type coercions

#### Core Operations (`ops.rs`)
- **Builtin**: Module, Function, Return
- **Quantum**: H, CNOT, Measure, StatePrep
- **Classical**: Add, Mul, Compare
- **Control**: Branch, Loop, Call
- **Memory**: Alloc, Load, Store

#### Custom Operations
- Dialect-specific operations (QEC, pulse control, etc.)
- Machine-specific operations

### 3. Progressive Lowering

PHIR uses multiple passes to gradually lower operations:

1. **Parsing → High-level**:
   - Resolve names and forward references
   - Infer types and insert implicit casts
   - Lower ForLoop/IfElse to CFG with branches

2. **High-level → Low-level**:
   - Lower quantum operations to runtime calls
   - Convert control flow to basic blocks
   - Optimize based on operation traits

3. **Low-level → Execution**:
   - Generate MLIR text for LLVM backend
   - Generate Rust code for native execution
   - Interpret directly for debugging

### 4. Interface-Based Abstract QEC Representation

PHIR takes an abstract approach to quantum error correction and emerging quantum paradigms:

#### The Interface Philosophy

Instead of hard-coding specific QEC schemes or quantum protocols into the IR, PHIR uses MLIR's interface approach - attaching semantic metadata through attributes to indicate which interfaces an operation implements:

```mlir
// A syndrome extraction operation implementing the QEC protocol interface
"qec.syndrome"() {
  qec.code_type = "surface_code",
  qec.syndrome_type = "X_stabilizers", 
  qec.extraction_round = 3 : i32,
  qec.ancilla_qubits = [5, 6, 7, 8],
  qec.data_qubits = [0, 1, 2, 3, 4]
} : () -> (i1, i1, i1, i1)

// A logical operation with multiple implementation strategies
"protocol.logical_gate"() {
  protocol.gate_type = "CNOT",
  protocol.implementations = ["transversal", "lattice_surgery", "code_deformation"],
  protocol.distance_preserved = true,
  protocol.resource_estimate = {time = 100, space = 50}
} : () -> ()
```

#### Benefits of the Interface Approach

1. **Future-proof**: New QEC codes (LDPC, floquet, quantum polar) can be added without changing core IR
2. **Research-friendly**: Experimentalists can prototype new protocols with custom interface attributes
3. **Multi-paradigm**: Different QEC schemes can coexist in the same program by implementing different interfaces
4. **Progressive optimization**: 
   - Generic passes operate on all interface implementations
   - Specialized passes optimize specific interface implementations
   - New passes can be added for new interfaces

#### Implementation Strategy

```rust
// Define protocol interfaces through attributes
pub trait QECProtocol {
    fn required_attributes() -> Vec<&'static str>;
    fn validate_attributes(attrs: &Attributes) -> Result<()>;
}

// Passes interpret operations based on their interface implementations
pub struct SurfaceCodeOptimization;
impl Pass for SurfaceCodeOptimization {
    fn run_on_operation(&mut self, op: &Operation) -> Result<()> {
        if op.get_attribute("qec.code_type") == Some("surface_code") {
            // Apply surface code specific optimizations
            self.optimize_syndrome_extraction(op)?;
            self.minimize_logical_gate_overhead(op)?;
        }
        Ok(())
    }
}
```

Example MLIR output:
```mlir
func.func private @__quantum__rt__qubit_allocate() -> !llvm.ptr
func.func private @__quantum__qis__h__body(!llvm.ptr)
func.func private @__quantum__qis__mz__body(!llvm.ptr, !llvm.ptr)
func.func private @__quantum__qis__read_result__body(!llvm.ptr) -> i1

func.func @main() -> i1 {
  %0 = func.call(@__quantum__rt__qubit_allocate()) : () -> !llvm.ptr
  func.call(@__quantum__qis__h__body(%0)) : (!llvm.ptr) -> ()
  %result_2 = func.call(@__quantum__rt__result_get_zero()) : () -> !llvm.ptr
  func.call(@__quantum__qis__mz__body(%0, %result_2)) : (!llvm.ptr, !llvm.ptr) -> ()
  %2 = func.call(@__quantum__qis__read_result__body(%result_2)) : (!llvm.ptr) -> i1
  func.return(%2)
}
```

### 4. MLIR Processing (mlir_toolchain.rs)

- **Tools Used**:
  - `mlir-opt`: Applies optimization and lowering passes
  - `mlir-translate`: Converts MLIR to LLVM IR
- **Passes Applied**:
  - Quantum dialect lowering (when available)
  - Function to LLVM conversion
  - Arithmetic to LLVM conversion
- **Flexibility**: Can add custom passes or optimizations

### 5. LLVM IR Generation

The final LLVM IR is produced by the MLIR toolchain and includes:
- Quantum runtime calls (`__quantum__rt__*`, `__quantum__qis__*`)
- Standard LLVM types and operations
- Metadata for debugging and optimization

## Benefits of This Architecture

1. **Separation of Concerns**: Each stage has a clear responsibility
2. **MLIR Ecosystem**: Can leverage existing MLIR passes, optimizations, and tools
3. **Extensibility**: Easy to add new operations, optimizations, or target backends
4. **Debugging**: Can inspect output at each stage (PHIR operations, MLIR text, LLVM IR)
5. **Standards-Based**: Uses standard MLIR format, compatible with any MLIR toolchain
6. **Future-Proof**: As MLIR's quantum dialect evolves, we can adopt improvements

## Usage

### Direct PHIR Construction

```rust
use pecos_pmir::{Module, Function, Block, Instruction};
use pecos_pmir::ops::{Operation, QuantumOp};

// Build quantum circuit directly
let mut module = Module::new("quantum_circuit");
let mut func = Function::new("main", function_type);

// Add operations to entry block
let entry_block = func.entry_region_mut()?.entry_block_mut()?;
entry_block.add_instruction(hadamard_op);
entry_block.add_instruction(cnot_op);
entry_block.add_instruction(measure_op);

module.add_function(func);
```

### Parsing to PHIR

```rust
use pecos_pmir::{Pipeline, PhirConfig, InputFormat};

let pipeline = Pipeline::new(PhirConfig::default());

// Parse source directly to PHIR
let module = pipeline.parse_to_pmir(source_code, InputFormat::HUGR)?;

// Module contains parsing operations that need lowering
// e.g., UnresolvedCall, ForLoop, InferType

// Lower to executable PHIR
let lowered = pipeline.lower_pmir(module)?;

// Execute using chosen strategy
let result = pipeline.execute_pmir(lowered)?;
```

### Inspecting PHIR

```rust
// Print MLIR text representation
println!("{}", module.to_mlir_text());

// Walk operations
use pecos_pmir::traits::OperationInterface;
for inst in &block.operations {
    println!("Op: {}, Side effects: {}", 
             inst.operation.name(),
             inst.operation.has_side_effects());
}
```

## Future Enhancements

1. **Parser Implementations**: Complete parsers for HUGR, PHIR, OpenQASM, Guppy
2. **Optimization Passes**: 
   - Quantum gate fusion and optimization
   - Classical subexpression elimination
   - Dead code elimination using analysis infrastructure
3. **Type System Enhancements**:
   - Linear types for quantum values
   - Effect types for side-effect tracking
   - Dependent types for sized arrays
4. **Execution Backends**:
   - Direct integration with PECOS simulators
   - GPU acceleration for classical simulation
   - Quantum hardware backends
5. **Tooling**:
   - Language server for IDE support
   - Debugger with breakpoints and stepping
   - Profiler for performance analysis

## Design Decisions

### Why No Separate AST?

Traditional compilers use separate AST and IR representations, but PHIR follows MLIR's approach of using a single hierarchical IR throughout:

1. **Simplicity**: One representation to learn, debug, and optimize
2. **Power**: MLIR's structure can represent anything an AST can
3. **Efficiency**: No conversion overhead or information loss
4. **Uniformity**: Same infrastructure (visitors, builders, verifiers) works everywhere
5. **Precedent**: MLIR has proven this approach works for many languages

### Parsing Strategy

Instead of parsing to an AST first, we parse directly to PHIR using special parsing operations:

- **Multi-pass**: Parse with placeholders → resolve → type check → lower
- **SSA construction**: Build SSA form incrementally during parsing
- **Type inference**: Use type variables, collect constraints, solve later
- **Progressive**: Mix high-level and low-level ops in same module

### Why Not a Custom Quantum Dialect (Yet)?

While we explored creating a custom quantum dialect for MLIR, we chose to generate standard MLIR with function calls for the initial implementation because:

1. **Simplicity**: Using standard dialects allows the pipeline to work with stock MLIR tools
2. **Compatibility**: No need to build custom MLIR tools or integrate C++ code with Rust
3. **Pragmatism**: The QIR functions are the ultimate target anyway
4. **Future-Proof**: We can add a quantum dialect later as an optimization pass

## Dependencies

- **Rust Dependencies**:
  - `pest` & `pest_derive`: Parser generator (prepared for future use)
  - `ron`: Rust Object Notation for serialization
  - `serde` & `serde_json`: Serialization framework
  
- **External Tools** (required for full pipeline):
  - MLIR toolchain (`mlir-opt`, `mlir-translate`)
  - LLVM toolchain (for linking and execution)

## Testing

The PHIR pipeline includes comprehensive tests:
- Unit tests for each stage
- Integration tests with quantum circuits (Hadamard, Bell states)
- RON serialization round-trip tests
- MLIR generation validation

Run tests with:
```bash
cargo test -p pecos-llvm-runtime test_pmir
```