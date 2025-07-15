# pecos-selene-eng Architecture

This crate provides integration between PECOS and Selene, focusing on classical control flow.

## Design Philosophy

The `selene_sim()` builder follows the same pattern as `qasm_sim()` and `llvm_sim()` in PECOS:

1. **Familiar API**: Chainable builder pattern with methods like `.seed()`, `.workers()`, etc.
2. **Classical Control Focus**: The engine handles program compilation, control flow, and command generation
3. **Quantum Simulation Separation**: Actual quantum simulation is handled by PECOS's QuantumEngine infrastructure

## Architecture

### SeleneEngine (Classical Control)
- Compiles quantum programs (Guppy, HUGR, LLVM IR)
- Manages classical control flow (if/while/functions)
- Generates quantum commands (ByteMessages)
- Processes measurement results for branching decisions

### Builder Pattern
The `selene_sim()` builder provides:
- **Program Input**: `.guppy()`, `.hugr()`, `.llvm_ir()`, `.hugr_file()`, `.llvm_file()`
- **Configuration**: `.qubits()`, `.seed()`, `.workers()`
- **Runtime Selection**: `.with_simple_runtime()`, `.with_soft_rz_runtime()`
- **Options**: `.with_optimization()`, `.verbose()`
- **Execution**: `.build()`, `.run(shots)`

### Mock Components
Since Selene isn't available as a dependency yet, we use mocks:
- `MockInstance`: Simulates Selene's program execution state
- `MockRuntime`: Handles operation transformations (e.g., gate decompositions)

## Integration Pattern

For testing and standalone use:
```rust
let results = selene_sim()
    .llvm_ir(program_ir.to_vec())
    .qubits(4)
    .seed(42)
    .workers(4)
    .with_optimization()
    .run(1000)?;
```

For production use with PECOS infrastructure:
```rust
// 1. Create classical control engine (Selene)
let classical_engine = selene_sim()
    .llvm_ir(program_ir.to_vec())
    .qubits(n)
    .build()?;

// 2. Pair with a quantum engine
// The seed, workers, noise model, etc. would be configured on the quantum engine
let quantum_engine = quest_sim()
    .qubits(n)
    .seed(42)
    .workers(4)
    .with_depolarizing_noise(0.01)
    .build()?;

// 3. Combine with coordinator
let mut coordinator = Coordinator::new(classical_engine, quantum_engine);

// 4. Run the program
let results = coordinator.run_shots(1000)?;
```

## Key Design Decisions

1. **Builder Pattern Consistency**: Methods like `.seed()` and `.workers()` are included in the builder for API consistency with `qasm_sim()` and `llvm_sim()`, even though they would be passed to the quantum engine in production.

2. **Test Convenience**: The `.run(shots)` method provides a way to test the classical control engine in isolation by simulating the full execution.

3. **Runtime Transformations**: Gate decompositions and other transformations are handled by the runtime in the classical engine, not the quantum engine.

4. **Clear Boundaries**: The engine strictly handles classical control - no quantum state manipulation.

## Future Work

When Selene becomes available as a dependency:
1. Replace mock implementations with actual Selene types
2. Add support for Selene's full plugin system
3. Integrate Selene's compiler optimizations
4. Add Python bindings following the PECOS pattern

## Lessons Learned

1. **Follow existing patterns**: The PECOS builder pattern is well-established and users expect consistency
2. **Separation of concerns**: Classical control and quantum simulation must be clearly separated
3. **Test infrastructure matters**: Being able to test the engine in isolation is valuable
4. **Mock early**: Creating mocks helped clarify the interface requirements