# pecos-llvm-sim Architecture

## Overview

`pecos-llvm-sim` is a unified simulation crate that orchestrates LLVM-based quantum circuit simulation in PECOS. It provides a builder pattern API that accepts multiple input formats (LLVM IR, HUGR, files) and handles the compilation and execution pipeline.

## Design Goals

1. **Single entry point** for all LLVM-based simulations
2. **Clean separation of concerns** between compilation and execution
3. **Flexible input formats** with automatic conversion
4. **Consistent API** matching other PECOS simulation functions

## Architecture

```
┌─────────────────────────────────────────────────┐
│              pecos-llvm-sim                     │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │         LlvmSim                          │   │
│  │                                          │   │
│  │  .llvm(String)      → LlvmIr            │   │
│  │  .llvm_file(Path)   → LlvmFile          │   │
│  │  .hugr(Hugr)        → Hugr              │   │
│  │  .hugr_bytes(Vec)   → HugrBytes         │   │
│  │  .hugr_file(Path)   → HugrFile          │   │
│  │                                          │   │
│  │  .with_noise_model()                     │   │
│  │  .seed()                                 │   │
│  │  .workers()                              │   │
│  │  .run() → SimulationResults              │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  Uses:                                          │
│  - pecos-hugr-llvm (for HUGR → LLVM)          │
│  - pecos-llvm-runtime (for execution)          │
└─────────────────────────────────────────────────┘
```

## Dependencies

- `pecos-hugr-llvm`: For HUGR to LLVM compilation
- `pecos-llvm-runtime`: For LLVM execution engine (`LlvmEngine`)
- `pecos-engines`: For noise models and quantum engines
- `pecos-core`: For error types and utilities

## Migration Plan

1. **Phase 1: Create new crate**
   - Set up `Cargo.toml` with dependencies
   - Create builder pattern structure
   - Implement multiple input format support

2. **Phase 2: Move simulation code**
   - Move `llvm_sim()` and related types from `pecos-llvm-runtime/src/simulation.rs`
   - Keep `LlvmEngine` in `pecos-llvm-runtime` (it's the core engine)
   - Update imports and re-exports

3. **Phase 3: Enhance functionality**
   - Add HUGR input support using `pecos-hugr-llvm`
   - Add file-based input support
   - Ensure all noise models and options work

4. **Phase 4: Update Python bindings**
   - Update `python/pecos-rslib/src/llvm_v3.rs` to use new crate
   - Maintain backward compatibility during transition

## API Design

### Builder Pattern

```rust
// From LLVM IR string
let results = LlvmSim::new().llvm(llvm_ir)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(1000)?;

// From HUGR
let results = LlvmSim::new().hugr(hugr)
    .with_state_vector_engine()
    .run(1000)?;

// From files
let results = LlvmSim::new().llvm_file("circuit.ll")
    .run(1000)?;
    
let results = LlvmSim::new().hugr_file("circuit.hugr")
    .run(1000)?;
```

### Convenience Functions

```rust
// Shortcuts matching existing API
pub fn llvm_sim(source: impl Into<String>) -> LlvmSim {
    LlvmSim::new().llvm(source)
}

// Note: Instead of a separate hugr_sim function, use:
// LlvmSim::new().hugr(hugr)
```

## Implementation Notes

1. The builder should lazily compile HUGR to LLVM only when `run()` is called
2. Error handling should provide clear messages about which stage failed
3. The simulation object should be reusable for multiple runs
4. Configuration should be immutable after building

## Testing Strategy

1. Test each input format independently
2. Test that HUGR and LLVM inputs produce identical results
3. Test all noise models and quantum engines
4. Test file I/O error handling
5. Benchmark compilation overhead for HUGR inputs