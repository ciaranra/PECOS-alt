# PECOS Architecture Documentation

This directory contains detailed documentation about the PECOS engine system architecture.

## Overview

PECOS uses a layered, composable architecture that separates concerns between:
- **User APIs**: High-level interfaces (`qasm_sim`, `llvm_sim`, `guppy_sim`)
- **Orchestration**: Parallel execution and shot management (`MonteCarloEngine`)
- **Coordination**: Control flow between classical and quantum components (`HybridEngine`)
- **Engines**: Core execution components (Classical, Quantum, Noise)

## Documentation Files

### 1. [ENGINE_SYSTEM_ARCHITECTURE.md](ENGINE_SYSTEM_ARCHITECTURE.md)
Complete overview of the engine system architecture including:
- Architecture layers and components
- Data flow through the system
- Parallelization with MonteCarloEngine
- User API design patterns
- Testing strategies

### 2. [ENGINE_TRAITS_GUIDE.md](ENGINE_TRAITS_GUIDE.md)
Detailed guide to engine traits and implementation:
- Core `Engine` trait
- `ClassicalEngine` and `ControlEngine` traits
- Implementation templates and examples
- Best practices for custom engines

### 3. [engine_system_diagram.py](engine_system_diagram.py)
Python script to generate architecture diagrams:
```bash
python engine_system_diagram.py
```
Generates:
- `engine_architecture.png/pdf` - Overall system architecture
- `engine_data_flow.png/pdf` - Data flow through engines

## Quick Start

### Understanding the Architecture

1. Start with [ENGINE_SYSTEM_ARCHITECTURE.md](ENGINE_SYSTEM_ARCHITECTURE.md) for the big picture
2. Run `python engine_system_diagram.py` to generate visual diagrams
3. Consult [ENGINE_TRAITS_GUIDE.md](ENGINE_TRAITS_GUIDE.md) when implementing custom engines

### Key Concepts

#### Engine Traits Hierarchy
```
Engine (base trait)
  ├─> ClassicalEngine (quantum program control)
  ├─> ControlEngine (execution flow management)
  ├─> QuantumEngine (quantum state simulation)
  └─> NoiseModel (error injection)

ClassicalControlEngine = ClassicalEngine + ControlEngine
```

#### Component Relationships
```
llvm_sim() → MonteCarloEngine → HybridEngine → LlvmEngine + QuantumSystem
                                                            └─> NoiseModel + QuantumEngine
```

### Example: Using LlvmEngine

```rust
// Direct usage (low-level)
let engine = LlvmEngine::new(llvm_file);
let results = MonteCarloEngine::run_with_noise_model(
    Box::new(engine),
    Box::new(DepolarizingNoiseModel::new_uniform(0.01)),
    shots,
    workers,
    seed,
)?;

// Via llvm_sim API (high-level)
let results = llvm_sim(llvm_file)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(shots)?;
```

## Architecture Principles

1. **Separation of Concerns**: Each component has a single, well-defined responsibility
2. **Composability**: Components can be combined using the `EngineSystem` pattern
3. **Extensibility**: New engines, noise models, and quantum backends can be added
4. **Type Safety**: Strong typing ensures compile-time verification
5. **Performance**: Designed for parallel execution and minimal overhead

## Related Documentation

- [LLVM Simulation API](../llvm_sim_api.md) - User guide for `llvm_sim()`
- [LLVM_SIM_ARCHITECTURE.md](../development/LLVM_SIM_ARCHITECTURE.md) - LLVM simulation specifics
- Engine implementations in `/crates/pecos-*/src/`

## Contributing

When adding new engines or modifying the architecture:
1. Update relevant documentation in this directory
2. Ensure new components follow established patterns
3. Add comprehensive tests for trait implementations
4. Update diagrams if the architecture changes