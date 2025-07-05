# PMIR Design Document

## Executive Summary

PMIR (PECOS Middle-level Intermediate Representation) is an MLIR-like intermediate representation written in Rust that bridges quantum circuit descriptions with multiple execution targets. It provides a unified framework for representing quantum-classical hybrid programs while maintaining the flexibility to target interpreters, native code generation, and hardware backends.

Key features:
- **MLIR-inspired architecture** with hierarchical regions and SSA form
- **Unified representation** from parsing through execution (no separate AST)
- **Three execution strategies**: Direct interpretation, Rust code generation, and MLIR/LLVM lowering
- **Domain-natural design** for quantum computing with built-in QEC support
- **Extensible dialect system** following "mechanism, not policy"
- **Progressive lowering** from high-level parsing ops to machine-level instructions

PMIR serves as the core IR for PECOS, enabling optimization, analysis, and execution of quantum programs across different backends while maintaining a clean separation between high-level quantum algorithms and low-level implementation details.

## Table of Contents

1. [Motivation and Vision](#1-motivation-and-vision)
2. [Architecture Overview](#2-architecture-overview)
3. [Design Principles](#3-design-principles)
4. [Core Design](#4-core-design)
5. [Execution Strategies](#5-execution-strategies)
6. [Frontend Support](#6-frontend-support)
7. [Key Features](#7-key-features)
8. [Implementation Plan](#8-implementation-plan)
9. [Design Decisions Summary](#9-design-decisions-summary)

**Related Documents:**
- [QEC.md](QEC.md) - Detailed quantum error correction support
- [IMPLEMENTATION.md](IMPLEMENTATION.md) - Phased implementation plan
- [EXAMPLES.md](EXAMPLES.md) - Extended usage examples

## 1. Motivation and Vision

### Why PMIR?

PECOS needs a flexible intermediate representation that can:
- Accept multiple input formats (HUGR, PHIR, OpenQASM, Guppy)
- Support multiple execution strategies without rewriting transformations
- Enable quantum-specific optimizations while maintaining classical performance
- Scale from small quantum circuits to large fault-tolerant algorithms

### Problems PMIR Solves

1. **Format Fragmentation**: Different quantum frameworks use incompatible representations
2. **Execution Flexibility**: Need both fast development (interpreter) and production performance (compilation)
3. **Quantum-Classical Integration**: Seamless mixing of quantum and classical computation
4. **QEC Support**: First-class support for error correction and fault tolerance
5. **Extensibility**: Easy addition of new operations and optimizations

### Vision

PMIR aims to be a complete, self-contained IR that can:
- Represent any quantum-classical hybrid program
- Execute directly for rapid development
- Compile to efficient native code for production
- Lower to standard formats for hardware execution
- Support advanced quantum compilation techniques

## 2. Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Input Formats                        │
│  (PHIR)  (HUGR)  (OpenQASM)  (Guppy)  (LLVM IR)       │
└────┬────────┬────────┬──────────┬────────┬────────────┘
     │        │        │          │        │
     v        v        v          v        v
┌─────────────────────────────────────────────────────────┐
│                PMIR (Parsing Ops)                       │
│   • UnresolvedCall, UnresolvedRef                      │
│   • ForLoop, IfElse (high-level control flow)          │
│   • InferType (type variables)                         │
└───────────────────────┬─────────────────────────────────┘
                        │ Lower
                        v
┌─────────────────────────────────────────────────────────┐
│                PMIR (Core Ops)                          │
│   • quantum.h, quantum.measure                         │
│   • control.branch, control.loop                       │
│   • arith.add, arith.mul                              │
├─────────────────────────────────────────────────────────┤
│ • Hierarchical: Operation → Region → Block → Operation │
│ • Extensible through dialects                          │
│ • SSA form with explicit dataflow                      │
└────────────┬──────────────┬──────────────┬─────────────┘
             │              │              │
             v              v              v
      ┌──────────┐   ┌──────────┐   ┌──────────┐
      │Interpreter│   │   Rust   │   │  MLIR/   │
      │          │   │ Codegen  │   │  LLVM    │
      └──────────┘   └──────────┘   └──────────┘
```

### Core Components

- **PMIR Core**: Hierarchical SSA IR with Operation → Region → Block structure
- **Parsing Operations**: Special ops for direct parsing (unresolved refs, type inference)
- **Dialects**: Extensible operation sets (quantum, classical, QEC, parsing, machine)
- **Progressive Lowering**: Multi-pass lowering from parsing ops to machine ops
- **Analysis**: Dominance, use-def chains, type inference, optimization

## 3. Design Principles

### 3.1 Core Philosophy: Simple, Composable, Progressive

PMIR's design is heavily influenced by PECOS's SLR (Simple Logical Representation), which demonstrated the power of building complex quantum protocols from simple, composable primitives. This philosophy permeates every level of PMIR:

**Simple Primitives**: Each operation has a single, well-defined purpose
- `quantum.h` - Apply Hadamard gate
- `quantum.measure` - Measure a qubit
- `control.branch` - Conditional branching
- No "kitchen sink" operations that try to do too much

**Natural Composition**: Structure mirrors conceptual quantum algorithm design
- Operations → Instructions (single quantum gates)
- Instructions → Blocks (quantum circuits)
- Blocks → Regions (subroutines/protocols)
- Regions → Operations (nested algorithms)

**Progressive Enhancement**: Start simple, add complexity only as needed
- Basic circuit: Just operations
- Add attributes: Noise models, optimization hints
- Add regions: Error correction protocols
- Add dialects: Custom operations for specific hardware

This approach ensures that:
1. Simple programs remain simple
2. Complex protocols are built from understandable parts
3. New users can start immediately without learning everything
4. Advanced users can add arbitrary complexity through composition

### 3.2 Flexibility Through Extensibility

PMIR follows MLIR's philosophy of providing mechanisms, not policies:

```rust
// Core provides structure
pub trait Dialect {
    fn name(&self) -> &str;
    fn register_ops(&self, registry: &mut OpRegistry);
}

// Extensions add semantics
let quantum_dialect = QuantumDialect::new();
let qec_dialect = QECDialect::new();
```

### 3.3 MLIR-Like but Domain-Natural

While adopting MLIR's architecture, PMIR makes quantum programming feel natural:

```rust
// Natural quantum circuit construction
circuit()
    .h(0)
    .cx(0, 1)
    .measure_all()
    .build()
```

### 3.4 Abstract QEC Through Boxing

Given the rapidly evolving landscape of quantum error correction, PMIR embraces abstraction:

**Philosophy**: Instead of hard-coding specific QEC schemes, use attributes and interfaces to represent QEC concepts abstractly. This allows:

- **Multiple paradigms**: Surface codes, color codes, LDPC codes can coexist
- **Research flexibility**: New schemes can be added without core changes
- **Progressive optimization**: Generic passes work on all codes, specialized passes optimize specific schemes

```rust
// Box a region with QEC metadata
region.attributes.insert("qec.logical_region", true);
region.attributes.insert("qec.code_family", "topological");
region.attributes.insert("qec.code_type", "surface_code");
region.attributes.insert("qec.distance", 7);

// Operations declare protocol interfaces
operation.attributes.insert("protocol.syndrome_extraction", true);
operation.attributes.insert("protocol.measurement_basis", "CSS");

// Optimization passes interpret based on capabilities
if has_attribute(region, "qec.code_type", "surface_code") {
    apply_surface_code_optimizations(region);
}
```

This "boxing" approach means:
- Core PMIR doesn't need to understand QEC details
- New QEC schemes are just new attribute conventions
- Passes can be as generic or specialized as needed
- Researchers can prototype new ideas easily

### 3.5 Progressive Complexity

Start simple, add complexity as needed:

```rust
// Simple usage
let module = PMIRBuilder::new()
    .with_function("main", |f| {
        f.h(0).cx(0, 1).measure(0)
    })
    .build();

// Advanced usage with QEC
let module = PMIRBuilder::new()
    .with_qec_dialect()
    .with_function("fault_tolerant", |f| {
        let logical = f.allocate_logical_qubits(SurfaceCode::new(21));
        f.logical_h(logical[0])
         .syndrome_extract()
         .error_correct()
    })
    .build();
```

### 3.6 Three Execution Strategies

Different strategies for different needs:

1. **Interpreter**: Fast startup, debugging, development
2. **Rust Codegen**: High performance, native integration
3. **MLIR Lowering**: Hardware compilation, LLVM optimizations

## 4. Core Design

### 4.1 PMIR Structure

PMIR follows MLIR's hierarchical organization:

```rust
pub struct Module {
    functions: Vec<Function>,
    globals: Vec<Global>,
    attributes: Attributes,
}

pub struct Function {
    signature: Signature,
    regions: Vec<Region>,
}

pub struct Region {
    blocks: Vec<Block>,
}

pub struct Block {
    arguments: Vec<BlockArg>,
    operations: Vec<Operation>,
    terminator: Terminator,
}

pub struct Operation {
    name: OpName,           // e.g., "quantum.h", "arith.add"
    operands: Vec<Value>,   // SSA inputs
    results: Vec<Value>,    // SSA outputs
    attributes: Attributes, // Compile-time metadata
    regions: Vec<Region>,   // Nested regions (for control flow)
}
```

#### SSA Form

All values are defined exactly once and have explicit use-def chains:

```rust
// %0 = quantum.alloc() : !quantum.qubit
// %1 = quantum.h(%0) : !quantum.qubit -> !quantum.qubit
// %2 = quantum.measure(%1) : !quantum.qubit -> !classical.bit
```

#### Type System

Extensible type system supporting quantum and classical types:

```rust
pub enum Type {
    // Quantum types
    Qubit,
    QuantumReg(usize),
    
    // Classical types
    Bit,
    Int(Width),
    Float(Width),
    
    // Composite types
    Array(Box<Type>, usize),
    Tuple(Vec<Type>),
    
    // Extension types
    Custom(String, TypeArgs),
}
```

### 4.2 PAST Structure

PAST is essentially **PMIR in tree form** - it uses the same operations and types but organized hierarchically:

```rust
// Shared operation definitions
pub mod ops {
    pub enum Operation {
        Quantum(QuantumOp),      // H, CNOT, Measure, etc.
        Classical(ClassicalOp),   // Add, Mul, Compare, etc.
        ControlFlow(ControlOp),   // Branch, Loop, Return, etc.
        Memory(MemoryOp),        // Alloc, Load, Store, etc.
    }
}

// PAST: Tree structure for parsing
pub struct PAST {
    root: NodeId,
    nodes: HashMap<NodeId, PASTNode>,
    hierarchy: HierarchyMap,    // Parent-child relationships
}

pub struct PASTNode {
    op: ops::Operation,         // Same ops as PMIR!
    children: Vec<NodeId>,      // Tree structure
}

// PMIR: Linear SSA for execution  
pub struct PMIROperation {
    op: ops::Operation,         // Same ops as PAST!
    operands: Vec<Value>,       // SSA inputs
    results: Vec<Value>,        // SSA outputs
}
```

The key insight: **PAST and PMIR share the same operation set** - they're just different structural views of the same program. See [PAST Design](PAST.md) for details.

### 4.3 Dialect System

Operations are organized into dialects:

```rust
// Core dialects
quantum_dialect    // Quantum operations: H, CNOT, measure
arith_dialect     // Arithmetic: add, mul, cmp
control_dialect   // Control flow: br, cond_br, return
memory_dialect    // Memory: alloc, load, store

// Extension dialects  
qec_dialect       // QEC operations: syndrome, correct
machine_dialect   // Hardware control: idle, transport
parallel_dialect  // Parallelism: parallel_for, sync
```

## 5. Execution Strategies

### 5.1 Strategy Selection

PMIR automatically selects the best execution strategy:

```rust
pub struct ExecutionPlanner {
    pub fn select_strategy(&self, module: &Module) -> ExecutionStrategy {
        let profile = self.analyze(module);
        
        match profile {
            // Small circuits with debugging: interpret
            Profile { qubits: 1..=10, debug: true, .. } => 
                ExecutionStrategy::Interpreter,
                
            // Repeated execution: compile
            Profile { iterations: 1000.., .. } => 
                ExecutionStrategy::RustCodegen,
                
            // Large-scale with QEC: MLIR
            Profile { qubits: 1000.., uses_qec: true, .. } => 
                ExecutionStrategy::MLIR,
                
            // Default: adaptive
            _ => ExecutionStrategy::Adaptive,
        }
    }
}
```

### 5.2 Interpreter

Direct execution for development and debugging:

```rust
pub struct Interpreter {
    quantum_state: QuantumSimulator,
    classical_state: Memory,
    
    pub fn execute(&mut self, op: &Operation) -> Result<Vec<Value>, Error> {
        match op.name {
            "quantum.h" => self.hadamard(op.operands[0]),
            "quantum.cx" => self.cnot(op.operands[0], op.operands[1]),
            "arith.add" => self.add(op.operands[0], op.operands[1]),
            // ...
        }
    }
}
```

### 5.3 Rust Code Generation

Compile to native Rust for performance:

```rust
pub struct RustCodegen {
    pub fn generate(&self, module: &Module) -> String {
        let mut code = String::new();
        
        // Generate imports
        code.push_str("use pecos_sim::*;\n\n");
        
        // Generate functions
        for func in &module.functions {
            code.push_str(&self.generate_function(func));
        }
        
        code
    }
}

// Example output:
// pub fn bell_circuit(sim: &mut Simulator) -> Result<Vec<bool>, Error> {
//     sim.h(0)?;
//     sim.cx(0, 1)?;
//     Ok(vec![sim.measure(0)?, sim.measure(1)?])
// }
```

### 5.4 MLIR/LLVM Lowering

Export to MLIR for hardware compilation:

```rust
impl Module {
    pub fn to_mlir_text(&self) -> String {
        // Module structure
        let mut mlir = String::from("module {\n");
        
        // Functions with quantum dialect
        for func in &self.functions {
            mlir.push_str(&format!(
                "  func @{}(%arg0: !quantum.qubit) -> !classical.bit {{\n",
                func.name
            ));
            
            // Operations
            for op in func.walk_ops() {
                mlir.push_str(&format!("    {}\n", op.to_mlir_text()));
            }
            
            mlir.push_str("  }\n");
        }
        
        mlir.push_str("}\n");
        mlir
    }
}
```

## 6. Frontend Support

### 6.1 Parser Architecture

Unified parser framework with incremental parsing:

```rust
pub trait QuantumParser {
    fn parse(&self, input: &str) -> Result<PAST, ParseError>;
}

pub struct ParserRegistry {
    parsers: HashMap<Format, Box<dyn QuantumParser>>,
    
    pub fn parse(&self, format: Format, input: &str) -> Result<PAST, Error> {
        let parser = self.parsers.get(&format)?;
        let ast = parser.parse(input)?;
        Ok(ast)
    }
}
```

### 6.2 Supported Formats

- **PHIR**: Direct mapping preserving machine operations
- **HUGR**: Natural hierarchical structure
- **OpenQASM 2.0**: Standard quantum assembly
- **Guppy**: High-level Python-like syntax
- **LLVM IR**: With quantum intrinsics

### 6.3 PHIR Optimization

Since PHIR is a primary frontend, optimize this path:

```rust
pub struct PHIRToPMIR {
    pub fn lower(&self, phir: PHIRProgram) -> Result<Module, Error> {
        let mut builder = PMIRBuilder::new();
        
        // Direct operation mapping
        for op in phir.ops {
            match op {
                PHIROp::QParallel { ops } => {
                    builder.create_parallel_region(|r| {
                        for qop in ops {
                            r.add_quantum_op(self.lower_qop(qop)?);
                        }
                    });
                }
                // ... other direct mappings
            }
        }
        
        builder.build()
    }
}
```

## 7. Key Features

### 7.1 Quantum Error Correction

First-class support for QEC (see [QEC.md](QEC.md) for details):

```rust
// QEC-aware types
LogicalQubit { code: SurfaceCode, distance: 21 }

// QEC operations
qec.syndrome_extract
qec.decode_syndrome  
qec.apply_correction

// Resource estimation
qec.estimate_resources
```

### 7.2 Parallelism Support

Explicit parallelism for quantum-classical coordination:

```rust
// Parallel quantum operations
quantum.parallel {
    quantum.h %q0
    quantum.h %q1
}

// Classical parallel decoding
parallel.for %i = 0 to %n {
    %syndrome = qec.extract_syndrome %logical[%i]
    %correction = qec.decode %syndrome
}
```

### 7.3 Verification

Built-in verification for quantum programs:

```rust
pub trait QuantumVerifier {
    fn verify_unitarity(&self, ops: &[Operation]) -> Result<(), Error>;
    fn verify_no_cloning(&self, module: &Module) -> Result<(), Error>;
    fn verify_measurement_consistency(&self, module: &Module) -> Result<(), Error>;
}
```

### 7.4 Resource Estimation

Analyze resource requirements without execution:

```rust
pub struct ResourceEstimator {
    pub fn estimate(&self, module: &Module) -> ResourceReport {
        ResourceReport {
            physical_qubits: self.count_physical_qubits(module),
            gate_count: self.count_gates(module),
            circuit_depth: self.calculate_depth(module),
            estimated_runtime: self.estimate_runtime(module),
        }
    }
}
```

## 8. Implementation Plan

### Phase 1: Minimal Core (Start Here)
- Basic Operation, Block, Region, Module types
- Simple SSA values and types
- Minimal builder API
- Basic validation

### Phase 2: Essential Dialects
- Quantum dialect (H, CNOT, measure)
- Arithmetic dialect (basic classical ops)
- Control flow dialect (branches, loops)
- Memory dialect (allocation, load/store)

### Phase 3: Execution Infrastructure
- Basic interpreter for quantum operations
- Integration with existing PECOS simulators
- Simple type checking
- Error reporting

### Phase 4: Frontend Support
- PHIR parser (priority)
- PAST to PMIR lowering
- Basic optimizations
- MLIR text output

See [IMPLEMENTATION.md](IMPLEMENTATION.md) for complete phased plan.

## 9. Design Decisions Summary

### What We Take From Each System

| System | Key Insights | How We Use It |
|--------|-------------|---------------|
| **MLIR** | Hierarchical regions, SSA form, dialects, passes | **PMIR structure** - Linear SSA, regions, dialect system |
| **PHIR** | Machine operations, explicit data flow, block structures | **Both** - Machine dialect, Result operations, block patterns |
| **HUGR** | Port-based connections, linear types, hierarchical AST | **PAST structure** - Tree hierarchy, port system, node weights |

### The Combination

- **PMIR** = MLIR's approach applied to quantum computing
- **PAST** = HUGR's tree structure + PMIR's operation semantics  
- **Together** = Best of both structural approaches with shared semantics

### Key Differentiators

1. **Three execution strategies** - No other quantum IR supports interpreter + codegen + lowering
2. **QEC as first-class** - Built-in support rather than library extensions
3. **Rust-native** - Leverage Rust's type system and performance
4. **Progressive complexity** - Simple things are simple, complex things are possible
5. **Dual representation** - PAST and PMIR share the same operations but optimize for different use cases

### PAST/PMIR Relationship

The design combines two complementary inspirations:

**PMIR** (MLIR-inspired):
- Linear SSA form optimized for transformations
- Region-based hierarchical structure  
- Dialect system for extensibility
- Progressive lowering capabilities

**PAST** (HUGR-inspired structure + PMIR semantics):
- Hierarchical tree structure optimized for parsing
- Same operations and types as PMIR
- Natural AST representation for source manipulation
- HUGR's proven patterns for tree-structured IRs

Key insight: **PAST is HUGR's structure filled with PMIR's operations**:

- **Shared Operations**: Both use identical operation definitions (quantum.h, arith.add, etc.)
- **Shared Types**: Both use the same type system (qubit, bit, arrays, etc.)  
- **Different Organization**: PAST uses HUGR-style trees, PMIR uses MLIR-style linear SSA
- **Seamless Conversion**: PAST lowers to PMIR by flattening the hierarchy to linear form

This means no semantic gap between parsing and execution - they're just structural views of the same program, each optimized for its use case.

### Design Trade-offs

| Decision | Benefit | Cost |
|----------|---------|------|
| MLIR-like structure | Proven architecture, easy MLIR export | Learning curve for developers |
| Three execution modes | Flexibility for different use cases | More implementation complexity |
| Dialect system | Extensibility | Indirection in operation dispatch |
| HUGR-inspired PAST | Natural AST representation | Two IRs to maintain |

## Next Steps

1. Review and refine this design
2. Implement Phase 1 minimal core
3. Create PHIR parser for immediate utility
4. Build basic interpreter for testing
5. Iterate based on usage experience

---

For additional details, see:
- [QEC.md](QEC.md) - Quantum error correction design
- [IMPLEMENTATION.md](IMPLEMENTATION.md) - Detailed implementation phases
- [EXAMPLES.md](EXAMPLES.md) - Usage examples and patterns