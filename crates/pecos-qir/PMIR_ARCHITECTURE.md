# PMIR (PECOS MLIR) Architecture

## Overview

PMIR (PECOS MLIR) is an alternative compilation pipeline that leverages the MLIR (Multi-Level Intermediate Representation) infrastructure to compile quantum programs from HUGR format to executable LLVM IR.

## Design Goals

1. **Leverage MLIR Infrastructure**: Use MLIR's powerful optimization and lowering infrastructure rather than reimplementing it
2. **Modular Design**: Clear separation between parsing, AST representation, and code generation
3. **Debuggability**: Intermediate representations can be inspected, serialized, and manipulated
4. **Standards Compliance**: Generate standard MLIR text that can be processed by any MLIR toolchain

## Architecture

The PMIR (PECOS Middle-level IR) pipeline consists of the following stages:

```
┌─────────────┐     ┌──────────┐     ┌──────────┐     ┌─────────────┐     ┌──────────┐
│   HUGR      │────▶│   PAST   │────▶│   PMIR   │────▶│  LLVM IR    │────▶│ Execution│
│   (JSON)    │     │  (AST)   │     │  (MLIR)  │     │   (.ll)     │     │          │
└─────────────┘     └──────────┘     └──────────┘     └─────────────┘     └──────────┘
      │                   │                 │                 │
      │                   ▼                 ▼                 ▼
      │              ┌──────────┐    ┌─────────────┐   ┌─────────────┐
      └─────────────▶│   RON    │    │  mlir-opt   │   │   PECOS     │
                     │ (Debug)  │    │mlir-translate│   │   Runtime   │
                     └──────────┘    └─────────────┘   └─────────────┘
```

Note: PMIR (PECOS Middle-level IR) represents the middle-level representation in this pipeline, sitting between the high-level PAST and the low-level LLVM IR. It's expressed as MLIR text format.

### 1. HUGR Parsing (hugr_parser.rs)

- **Input**: HUGR JSON format (the serialized quantum program representation)
- **Output**: PAST (PECOS AST) - a Rust data structure
- **Method**: Currently uses serde_json, with Pest grammar prepared for future use
- **Purpose**: Parse and validate the input, creating a strongly-typed AST

### 2. PAST (PECOS AST) - ast.rs

The PAST (PECOS Abstract Syntax Tree) is the central intermediate representation:

- **Rust Native**: Defined as Rust enums and structs for type safety
- **Serializable**: Can be serialized to/from RON (Rust Object Notation) for debugging
- **Complete**: Represents all quantum and classical operations
- **Graph-Based**: Maintains the dataflow graph structure from HUGR

Key structures:
- `PastModule`: Top-level container with functions and metadata
- `PastFunction`: Function with input/output types and body graph
- `PastGraph`: Nodes and edges representing computation
- `PastOp`: Enum of all supported operations (quantum gates, measurements, classical ops)

### 3. MLIR Generation (mlir_lowering.rs)

- **Input**: PAST data structure
- **Output**: MLIR text in standard format
- **Dialects Used**:
  - `func`: For function definitions and calls
  - `arith`: For arithmetic operations
  - `llvm`: For pointer types and eventual lowering
- **Approach**: Generate standard MLIR with QIR function calls

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
4. **Debugging**: Can inspect output at each stage (PAST as RON, MLIR text, LLVM IR)
5. **Standards-Based**: Uses standard MLIR format, compatible with any MLIR toolchain
6. **Future-Proof**: As MLIR's quantum dialect evolves, we can adopt improvements

## Usage

### Basic Compilation

```rust
use pecos_qir::pmir::{compile_hugr_via_pmir, PmirConfig};

let config = PmirConfig {
    debug_output: true,
    optimization_level: 2,
    target_triple: None,
};

let llvm_ir = compile_hugr_via_pmir(hugr_json, &config)?;
```

### Debugging with RON

```rust
// Parse to PAST
let past = hugr_parser::parse_hugr_to_past(hugr_json)?;

// Serialize to RON for inspection
let ron_string = past.to_ron_string()?;
println!("PAST in RON:\n{}", ron_string);

// Can also deserialize from RON
let past_from_ron = PastModule::from_ron_string(&ron_string)?;
```

### Custom MLIR Processing

```rust
// Generate MLIR text
let mlir_module = mlir_lowering::lower_past_to_pmir(&past, &config)?;
let mlir_text = mlir_module.to_string();

// Write to file for manual processing
std::fs::write("output.mlir", mlir_text)?;

// Run custom MLIR passes
let custom_config = MlirToolchainConfig {
    optimization_passes: vec![
        "--my-custom-pass".to_string(),
        "--convert-func-to-llvm".to_string(),
    ],
    ..Default::default()
};
```

## Future Enhancements

1. **Pest Grammar**: Implement full HUGR parsing using the Pest grammar for better error messages
2. **In-Memory MLIR**: Direct C++ API integration to avoid file I/O
3. **Quantum Dialect**: Create a proper MLIR quantum dialect for better optimization opportunities
   - Define quantum types (`!quantum.qubit`, `!quantum.result`)
   - Implement quantum operations as first-class MLIR ops
   - Write lowering passes from quantum dialect to QIR calls
4. **Custom Passes**: Quantum-specific optimization passes in MLIR
5. **Python Bindings**: Complete Python API for use from quantum frameworks
6. **Additional Backends**: Target other backends through MLIR (GPU, TPU, specialized quantum hardware)

## Design Decisions

### Why Not a Custom Quantum Dialect (Yet)?

While we explored creating a custom quantum dialect for MLIR (see `quantum_dialect.td` and `quantum_to_llvm.cpp`), we chose to generate standard MLIR with function calls for the initial implementation because:

1. **Simplicity**: Using standard dialects allows the pipeline to work with stock MLIR tools
2. **Compatibility**: No need to build custom MLIR tools or integrate C++ code with Rust
3. **Pragmatism**: The QIR functions are the ultimate target anyway
4. **Future-Proof**: We can add a quantum dialect later as an optimization pass

The quantum dialect approach remains valuable for future optimization work, where quantum-specific transformations (gate fusion, circuit optimization) would benefit from higher-level operation semantics.

## Dependencies

- **Rust Dependencies**:
  - `pest` & `pest_derive`: Parser generator (prepared for future use)
  - `ron`: Rust Object Notation for serialization
  - `serde` & `serde_json`: Serialization framework
  
- **External Tools** (required for full pipeline):
  - MLIR toolchain (`mlir-opt`, `mlir-translate`)
  - LLVM toolchain (for linking and execution)

## Testing

The PMIR pipeline includes comprehensive tests:
- Unit tests for each stage
- Integration tests with quantum circuits (Hadamard, Bell states)
- RON serialization round-trip tests
- MLIR generation validation

Run tests with:
```bash
cargo test -p pecos-qir test_pmir
```