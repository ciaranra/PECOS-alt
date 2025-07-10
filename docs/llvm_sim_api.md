# LLVM Simulation API

The `llvm_sim()` API now provides full feature parity with `qasm_sim()`, including noise models, parallelization, and multiple quantum engines.

## Key Features

### 1. **Noise Models**
- Pass-through (no noise)
- Uniform depolarizing noise
- Custom depolarizing noise with different error rates
- Biased depolarizing noise
- General noise models

### 2. **Parallelization**
- Multi-threaded execution via `MonteCarloEngine`
- Configurable worker threads
- Efficient shot distribution

### 3. **Quantum Engines**
- State vector simulator (default)
- Sparse stabilizer simulator (efficient for Clifford circuits)

### 4. **Builder Pattern**
- Fluent API matching `qasm_sim()`
- Build once, run multiple times
- Flexible configuration

## API Examples

### Basic Usage

```rust
use pecos_llvm_runtime::llvm_sim;

// Simple execution
let results = llvm_sim(llvm_ir).run(1000)?;

// With seed for reproducibility
let results = llvm_sim(llvm_ir)
    .seed(42)
    .run(1000)?;
```

### Noise Models

```rust
// Uniform depolarizing noise
let results = llvm_sim(llvm_ir)
    .with_depolarizing_noise(0.01) // 1% error rate
    .run(1000)?;

// Custom depolarizing noise
let results = llvm_sim(llvm_ir)
    .with_custom_depolarizing_noise(
        0.02, // prep error
        0.03, // measurement error
        0.01, // single-qubit gate error
        0.05, // two-qubit gate error
    )
    .run(1000)?;

// Biased depolarizing noise
let results = llvm_sim(llvm_ir)
    .with_biased_depolarizing_noise(0.02)
    .run(1000)?;
```

### Parallelization

```rust
// Use multiple workers for parallel execution
let results = llvm_sim(llvm_ir)
    .workers(8) // Use 8 threads
    .run(10000)?;
```

### Quantum Engines

```rust
// Use sparse stabilizer engine
let results = llvm_sim(llvm_ir)
    .with_sparse_stabilizer_engine()
    .run(1000)?;

// Or specify explicitly
use pecos_llvm_runtime::QuantumEngineType;
let results = llvm_sim(llvm_ir)
    .quantum_engine(QuantumEngineType::SparseStabilizer)
    .run(1000)?;
```

### Build Once, Run Many

```rust
// Build the simulation once
let mut sim = llvm_sim(llvm_ir)
    .seed(42)
    .workers(4)
    .with_depolarizing_noise(0.01)
    .build()?;

// Run multiple times with different shot counts
let results1 = sim.run(100)?;
let results2 = sim.run(1000)?;
let results3 = sim.run(10000)?;

// Get statistics
let (total_shots, total_runs) = sim.stats();
```

### Advanced Options

```rust
// Full configuration example
let results = llvm_sim(llvm_ir)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .with_state_vector_engine()
    .verbose(true)
    .debug(false)
    .keep_temp_files(true)
    .run(10000)?;
```

## Result Format

Results are returned as `HashMap<String, Vec<i64>>` in columnar format:

```rust
// Example result structure for Bell state
{
    "c0": [0, 1, 1, 0, 1, ...], // 1000 values
    "c1": [0, 1, 1, 0, 1, ...], // 1000 values
}
```

## Integration with Guppy

The enhanced `llvm_sim()` can be used seamlessly with Guppy-generated LLVM IR:

```python
# Python side
from guppylang import compile_to_llvm
llvm_ir = compile_to_llvm(guppy_function)

# Rust side
let results = llvm_sim(llvm_ir)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(10000)?;
```

## Performance Considerations

1. **Compilation**: LLVM IR is compiled once and cached
2. **Parallelization**: Use `workers` equal to CPU cores - 1
3. **Noise**: Higher noise levels require more shots for accuracy
4. **Engine Choice**: 
   - StateVector: General purpose, handles all gates
   - SparseStabilizer: Fast for Clifford-only circuits

## Migration from `qasm_sim()`

The API is designed to be familiar to `qasm_sim()` users:

```rust
// qasm_sim
let results = qasm_sim(qasm)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(1000)?;

// llvm_sim (identical API)
let results = llvm_sim(llvm_ir)
    .seed(42)
    .workers(8)
    .with_depolarizing_noise(0.01)
    .run(1000)?;
```

## Future Enhancements

1. **True In-Memory Execution**: Eliminate temporary files entirely
2. **Module Caching**: Cache compiled LLVM modules
3. **Custom Noise Models**: User-defined noise models
4. **Progress Callbacks**: Track simulation progress
5. **Streaming Results**: Handle very large shot counts efficiently