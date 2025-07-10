# Guppy Simulation Builder Pattern

## Overview

The `guppy_sim()` function provides a builder pattern API for Guppy quantum programs, matching the design of `qasm_sim()` for consistency across PECOS.

## Architecture

### Python-Rust Split

1. **Python Layer** (`quantum-pecos`):
   - Handles Guppy → HUGR conversion (must be in Python as Guppy functions are Python objects)
   - Provides the builder pattern API
   - Manages configuration and user interface

2. **Rust Layer** (`pecos-rslib`):
   - Takes over once HUGR bytes are available
   - Handles HUGR → LLVM compilation (via `pecos-hugr-llvm`)
   - Executes LLVM IR via `pecos-llvm-runtime`
   - Provides high-performance simulation

### Key Components

```
User Code (Python)
    ↓
guppy_sim() → GuppySimulationBuilder
    ↓
.build() → Guppy → HUGR (via guppylang)
    ↓
         → HUGR → LLVM (via Rust)
    ↓
GuppySimulation
    ↓
.run(shots) → Execute via pecos-llvm-runtime
    ↓
Results (columnar format)
```

## API Design

### Builder Pattern

```python
from pecos import guppy_sim
from guppylang import guppy
from guppylang.std.quantum import qubit, h, cx, measure

@guppy
def bell_state() -> tuple[bool, bool]:
    q0, q1 = qubit(), qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)

# Build once, run multiple times
sim = guppy_sim(bell_state).seed(42).build()
results_100 = sim.run(100)
results_1000 = sim.run(1000)

# Or run directly
results = guppy_sim(bell_state).seed(42).run(1000)
```

### Configuration Options

- `.seed(int)` - Set random seed
- `.workers(int)` - Number of worker threads
- `.noise(NoiseModel)` - Noise model (when implemented)
- `.engine(str)` - Quantum engine ("StateVector", "SparseStabilizer")
- `.verbose(bool)` - Enable verbose output
- `.debug(bool)` - Enable debug information
- `.optimize(bool)` - Enable LLVM optimizations
- `.keep_intermediate_files(bool)` - Keep compilation artifacts
- `.config(dict)` - Apply configuration from dictionary

### Result Format

Results are returned in columnar format matching `qasm_sim`:

```python
# For bell_state() returning tuple[bool, bool]:
{
    "_result": [0, 3, 0, 3, ...],  # 0 = |00⟩, 3 = |11⟩
    "_metadata": {
        "shots": 1000,
        "execution_time": 0.123,
        "function_name": "bell_state",
        "total_runs": 1,
        "total_shots": 1000
    }
}
```

## Implementation Details

### File Structure

```
quantum-pecos/src/pecos/frontends/
├── guppy_sim_builder.py  # Builder pattern implementation
├── run_guppy.py          # Original run_guppy() function
└── guppy_frontend.py     # Low-level compilation frontend
```

### Key Classes

1. **GuppySimulationBuilder**
   - Fluent interface for configuration
   - Handles compilation on first `.build()`
   - Returns reusable `GuppySimulation` instance

2. **GuppySimulation**
   - Holds compiled HUGR and LLVM IR
   - Can be run multiple times with different shots
   - Tracks execution statistics
   - Formats results in columnar format

3. **GuppySimulationConfig**
   - Dataclass holding all configuration options
   - Easily extensible for new features

## Future Rust Implementation

To maximize performance, future work should move more functionality to Rust:

1. Create `GuppySimulationBuilder` struct in Rust
2. Store compiled LLVM module in Rust memory
3. Expose via PyO3 bindings in `pecos-rslib`
4. Python layer only handles Guppy → HUGR conversion

Example Rust structure:
```rust
// In pecos-rslib
pub struct GuppySimulationBuilder {
    hugr_bytes: Vec<u8>,
    config: SimulationConfig,
}

pub struct GuppySimulation {
    llvm_module: LlvmModule,
    config: SimulationConfig,
}

#[pyfunction]
pub fn guppy_sim(hugr_bytes: Vec<u8>) -> GuppySimulationBuilder {
    GuppySimulationBuilder::new(hugr_bytes)
}
```

## Comparison with qasm_sim

| Feature | qasm_sim | guppy_sim |
|---------|----------|-----------|
| Input | QASM string | Guppy function |
| Builder pattern | ✓ | ✓ |
| Build once, run many | ✓ | ✓ |
| Columnar results | ✓ | ✓ |
| Noise models | ✓ | Future |
| Multiple engines | ✓ | Future |
| Rust backend | Full | Partial |

## Testing

See `python/tests/guppy/test_guppy_sim_builder.py` for comprehensive tests of:
- Builder pattern API
- Multiple runs with same compilation
- Seeded reproducibility
- Configuration options
- Result format validation

## Next Steps

1. **Implement Rust bindings** for `GuppySimulationBuilder` in `pecos-rslib`
2. **Add noise model support** once LLVM runtime supports it
3. **Optimize compilation caching** to avoid recompilation
4. **Add progress callbacks** for long-running simulations
5. **Support classical type operations** (int, float) in HUGR → LLVM