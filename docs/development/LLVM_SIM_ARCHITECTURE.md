# LLVM Simulation Architecture

## Overview

The `llvm_sim()` API provides a unified builder pattern for executing quantum programs compiled to LLVM IR, regardless of their source language (QASM, Guppy, or custom).

## Design Principles

1. **In-Memory First**: Prioritize in-memory LLVM IR to avoid file I/O
2. **Builder Pattern**: Consistent API matching `qasm_sim()` 
3. **Language Agnostic**: Any valid quantum LLVM IR can be executed
4. **Efficient Reuse**: Compile once, run multiple times

## Architecture Layers

```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   qasm_sim()    │  │   guppy_sim()   │  │  Custom Source  │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                     │                     │
         ▼                     ▼                     ▼
┌────────────────────────────────────────────────────────────┐
│                      llvm_sim()                            │
│  • Accepts in-memory LLVM IR strings                       │
│  • Provides builder pattern API                            │
│  • Manages temporary files efficiently                     │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                   pecos-llvm-runtime                       │
│  • LLVM JIT compilation                                    │
│  • Quantum instruction execution                           │
│  • Result collection                                       │
└────────────────────────────────────────────────────────────┘
```

## API Design

### Core Function

```rust
pub fn llvm_sim<T: Into<LlvmSimSource>>(source: T) -> LlvmSimBuilder
```

### Builder Pattern

```rust
// From in-memory LLVM IR
let results = llvm_sim(llvm_ir_string)
    .seed(42)
    .workers(4)
    .noise("depolarizing")
    .build()?
    .run(1000)?;

// From file
let results = llvm_sim("circuit.ll")
    .verbose(true)
    .run(1000)?;

// Build once, run many
let mut sim = llvm_sim(llvm_ir).build()?;
let r1 = sim.run(100)?;
let r2 = sim.run(1000)?;
```

### Result Format

Returns columnar format matching `qasm_sim`:
```rust
HashMap<String, Vec<i64>>
// Example: {"c": [0, 3, 0, 3, ...]} for Bell state
```

## Integration with Existing Systems

### QASM Integration

```rust
// Current: QASM → AST → Commands → Execution
// Proposed: QASM → AST → LLVM IR → llvm_sim()

impl QasmSimulation {
    fn to_llvm_ir(&self) -> String {
        // Convert QASM AST to LLVM IR
    }
    
    fn run(&self, shots: usize) -> Result<...> {
        let llvm_ir = self.to_llvm_ir();
        llvm_sim(llvm_ir)
            .seed(self.config.seed)
            .run(shots)
    }
}
```

### Guppy Integration

```rust
// Current: Guppy → HUGR → LLVM → File → Execute
// With llvm_sim: Guppy → HUGR → LLVM → llvm_sim()

impl GuppySimulation {
    fn run(&self, shots: usize) -> Result<...> {
        // self.llvm_ir is already in memory
        llvm_sim(&self.llvm_ir)
            .seed(self.config.seed)
            .run(shots)
    }
}
```

## Performance Benefits

1. **No File I/O**: LLVM IR stays in memory
2. **Shared Infrastructure**: All quantum languages use same execution path
3. **Better Caching**: Compiled modules can be cached in memory
4. **Parallel Execution**: Built-in support for worker threads

## Current Implementation

The `llvm_sim()` API provides:
- **In-Memory First**: Accepts LLVM IR strings, creates temp files only when needed
- **Full Feature Parity**: Noise models, parallelization, multiple quantum engines
- **Builder Pattern**: Consistent API matching `qasm_sim()`
- **Efficient Reuse**: Compile once, run multiple times

## Future Enhancements

### True In-Memory Execution
- Use LLVM's `MemoryBuffer` API
- JIT compile directly from memory
- No temporary files at all

### Module Caching
- Cache compiled LLVM modules
- Instant re-execution of same circuit
- Memory-mapped shared libraries

### Advanced Features
- Streaming results for large shots
- Progress callbacks
- Custom memory allocators

## Example: Complete Pipeline

```rust
// High-level language
let guppy_code = r#"
@guppy
def bell_state() -> tuple[bool, bool]:
    q0, q1 = qubit(), qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)
"#;

// Compile to LLVM IR (in Python/Guppy compiler)
let llvm_ir = compile_guppy_to_llvm(guppy_code);

// Execute with llvm_sim (in Rust)
let results = llvm_sim(llvm_ir)
    .seed(42)
    .workers(8)
    .build()?
    .run(10000)?;

// Results in standard columnar format
// {"_result": [0, 3, 0, 3, ...]}
```

## Benefits of Unified Architecture

1. **Consistency**: Same API for all quantum languages
2. **Maintenance**: Single execution engine to optimize
3. **Features**: Noise models, parallelization, etc. available to all
4. **Testing**: One test suite for execution engine
5. **Performance**: Optimizations benefit all languages

## Implementation Status

✅ **Completed**:
1. Implement `llvm_sim` in `pecos-llvm-runtime` with full feature parity
2. Add PyO3 bindings for Python access
3. Support for noise models and parallelization
4. Builder pattern matching `qasm_sim()` API

🔄 **In Progress**:
- Migrate `guppy_sim` to use `llvm_sim` internally

📋 **Future Work**:
- Consider migrating `qasm_sim` to use `llvm_sim`
- Add true in-memory execution without temp files
- Implement module caching for repeated execution