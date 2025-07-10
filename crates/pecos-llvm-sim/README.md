# pecos-llvm-sim

Unified LLVM-based quantum simulation with support for multiple input formats in PECOS.

## Overview

This crate provides a flexible builder pattern API for quantum circuit simulation that accepts:
- LLVM IR (strings or files)
- HUGR (in-memory objects, bytes, or files)

It handles the compilation pipeline automatically and provides consistent simulation capabilities with noise models, parallelization, and multiple quantum engines.

## Usage

```rust
use pecos_llvm_sim::{llvm_sim, LlvmSim};

// From LLVM IR string
let results = llvm_sim(llvm_ir)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(1000)?;

// From HUGR object
let results = LlvmSim::new().hugr(hugr)
    .with_state_vector_engine()
    .run(1000)?;

// From files
let results = LlvmSim::new().llvm_file("circuit.ll")
    .run(1000)?;

let results = LlvmSim::new().hugr_file("circuit.hugr")
    .run(1000)?;
```

## Features

- **Multiple input formats**: LLVM IR, HUGR, or files
- **Automatic compilation**: HUGR → LLVM IR conversion handled internally
- **Noise models**: Depolarizing, biased depolarizing, and custom noise
- **Parallel execution**: Multi-threaded shot distribution
- **Quantum engines**: State vector and sparse stabilizer backends
- **Builder pattern**: Intuitive configuration API

## Architecture

This crate orchestrates:
- `pecos-hugr-llvm` for HUGR → LLVM compilation
- `pecos-llvm-runtime` for LLVM execution
- `pecos-engines` for quantum simulation backends

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed design information.