# PECOS PMIR (PECOS MLIR Pipeline)

PECOS PMIR provides an alternative compilation pipeline from HUGR to LLVM IR via MLIR (Multi-Level Intermediate Representation).

## Overview

The PMIR pipeline offers a multi-stage compilation approach:

```
HUGR → PAST (AST) → PMIR (MLIR) → LLVM IR
```

This provides several advantages:
- **Modular compilation**: Each stage can be inspected and debugged independently
- **MLIR optimizations**: Leverage MLIR's optimization infrastructure
- **Future extensibility**: Direct PMIR execution without LLVM compilation
- **Research flexibility**: Experiment with different compilation strategies

## Architecture

### Components

1. **HUGR Parser** (`hugr_parser.rs`)
   - Parses HUGR JSON format into PAST (PECOS AST) structures
   - Uses Pest parser for flexible parsing

2. **AST Definition** (`ast.rs`)
   - Defines PAST structures representing quantum programs
   - Serializable to RON format for debugging

3. **MLIR Lowering** (`mlir_lowering.rs`)
   - Converts PAST to MLIR text format
   - Generates quantum operation calls following QIR conventions

4. **MLIR Toolchain** (`mlir_toolchain.rs`)
   - Integrates with external MLIR tools (`mlir-opt`, `mlir-translate`)
   - Handles LLVM IR generation from MLIR

5. **Angle Resolver** (`angle_resolver.rs`)
   - Resolves rotation angles by following dataflow edges
   - Handles HUGR's representation of parameterized gates

## Usage

### Basic Compilation

```rust
use pecos_pmir::{PmirConfig, compile_hugr_via_pmir};

let config = PmirConfig {
    debug_output: true,
    optimization_level: 2,
    target_triple: None,
};

let llvm_ir = compile_hugr_via_pmir(hugr_json, &config)?;
```

### Inspecting Intermediate Representations

```rust
use pecos_pmir::{hugr_to_past_ron, hugr_to_pmir_mlir};

// Get PAST representation in RON format
let past_ron = hugr_to_past_ron(hugr_json)?;

// Get MLIR representation
let mlir_text = hugr_to_pmir_mlir(hugr_json, &config)?;
```

## Features

- `default`: Basic functionality
- `direct-execution`: Future feature for direct PMIR execution
- `mlir-compilation`: MLIR toolchain integration
- `python-bindings`: Python API support

## Requirements

The PMIR pipeline requires external MLIR tools to be installed:
- `mlir-opt`: For optimization and lowering passes
- `mlir-translate`: For MLIR to LLVM IR translation

These can typically be installed as part of an LLVM/MLIR distribution.

## Future Work

- **Direct Execution**: Execute PMIR directly using PECOS simulators without LLVM compilation
- **Custom MLIR Dialects**: Develop quantum-specific MLIR dialects for better optimization
- **Advanced Optimizations**: Implement quantum-aware optimization passes in MLIR