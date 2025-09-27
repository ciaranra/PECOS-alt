# PECOS LLVM Runtime

This crate provides LLVM IR execution capabilities for hybrid quantum/classical programs in the PECOS framework.

## Overview

The PECOS LLVM Runtime crate enables execution of quantum programs compiled to LLVM Intermediate Representation (LLVM IR). While originally designed for QIR (Quantum Intermediate Representation), this crate now supports any LLVM IR that follows the quantum runtime conventions for gate calls and measurement operations.

This crate contains all LLVM runtime functionality, providing a flexible execution environment for hybrid quantum/classical programs that can work with various quantum programming frontends.

## Requirements

- LLVM version 14.x with the 'llc' tool is required for LLVM IR compilation
  - Linux: `sudo apt install llvm-14 llvm-14-dev`
  - macOS: `brew install llvm@14`
  - Windows: Download LLVM 14.x installer from [LLVM releases](https://releases.llvm.org/download.html#14.0.0)

**Note**: Only LLVM version 14.x is compatible. LLVM 15 or later versions will not work with PECOS's LLVM runtime implementation.

## Usage

### From Rust

```rust
use pecos_llvm_runtime::LlvmEngine;
use std::path::PathBuf;

fn main() {
    // Create an LLVM engine for a specific LLVM IR file
    let llvm_path = PathBuf::from("path/to/your/program.ll");
    let mut engine = LlvmEngine::new(llvm_path);

    // Pre-compile the LLVM IR program for better performance
    engine.pre_compile().expect("Failed to pre-compile LLVM IR program");

    // Run the LLVM IR program (for a complete workflow, see examples)
    // ...
}
```

### From CLI

PECOS includes a command-line interface that supports executing LLVM IR programs:

```sh
# Run an LLVM IR program
pecos run path/to/program.ll

# Run with specific number of shots
pecos run path/to/program.ll -s 100

# Run with noise model
pecos run path/to/program.ll -p 0.01
```

## Architecture

The LLVM Runtime crate includes several components:

- **LlvmEngine**: The main entry point for executing LLVM IR programs
- **LlvmLinker**: Handles compilation of LLVM IR programs to native code
- **LlvmLibrary**: Manages loading and interaction with compiled LLVM libraries
- **RuntimeBuilder**: Builds and manages the static runtime library
- **Platform-specific modules**: Handle differences between Linux, macOS, and Windows

## Contributing

Contributions to improve the LLVM runtime implementation are welcome! Please follow the contribution guidelines in the main PECOS repository.

## License

This crate is licensed under the Apache-2.0 License, as is the rest of the PECOS project.
