# PECOS-Selene Classical Engine (CENG)

A fully working Classical/Control Engine that integrates Selene quantum emulation platform with PECOS quantum simulation infrastructure.

## Features

- ✅ **Complete PECOS Integration**: Implements `ClassicalEngine`, `ControlEngine`, and `Engine` traits
- ✅ **Clone Support**: Full support for PECOS's multi-worker clone-per-worker pattern
- ✅ **Multiple Program Formats**: Supports Guppy, HUGR, and LLVM IR programs
- ✅ **Builder Pattern API**: Familiar `selene_sim()` API matching PECOS conventions
- ✅ **Full Quantum Simulation**: Complete quantum simulation on par with `llvm_sim()`
- ✅ **Noise Model Support**: All PECOS noise models (depolarizing, biased, general)
- ✅ **Quantum Engine Selection**: State vector and sparse stabilizer simulators

## Quick Start

```rust
use pecos_selene_ceng::selene_sim;

// Run a quantum simulation with LLVM IR
let bell_state_llvm = r#"
declare void @__quantum__qis__h__body(i64)
declare void @__quantum__qis__cx__body(i64, i64)
declare i32 @__quantum__qis__m__body(i64, i64)

define void @bell_state() #0 {
entry:
    call void @__quantum__qis__h__body(i64 0)
    call void @__quantum__qis__cx__body(i64 0, i64 1)
    %result0 = call i32 @__quantum__qis__m__body(i64 0, i64 0)
    %result1 = call i32 @__quantum__qis__m__body(i64 1, i64 1)
    ret void
}

attributes #0 = { "EntryPoint" }
"#;

// Run multiple shots with noise and quantum engine selection
let results = selene_sim()
    .llvm_ir(bell_state_llvm.to_vec())
    .qubits(2)
    .seed(42)
    .workers(4)
    .optimize()
    .noise(DepolarizingNoise { p: 0.01 })  // Ergonomic noise API
    .quantum_engine(QuantumEngineType::StateVector)  // Full quantum state simulation
    .run(100)?;

// Or use custom depolarizing noise
let results = selene_sim()
    .llvm_ir(bell_state_llvm.to_vec())
    .qubits(2)
    .noise(DepolarizingCustomNoise {
        p_prep: 0.001,  // Prep error
        p_meas: 0.002,  // Measurement error  
        p1: 0.003,      // Single-qubit gate error
        p2: 0.004,      // Two-qubit gate error
    })
    .run(100)?;

println!("Completed {} shots", results.num_shots());
```

## The SeleneEngine

The `SeleneEngine` is a working Classical/Control Engine that integrates Selene with PECOS:

```rust
let engine = selene_sim()
    .llvm_ir(quantum_program_ir)
    .qubits(2)
    .build()?;
```

**Features:**
- ✅ Implements all PECOS traits
- ✅ Supports clone-per-worker pattern
- ✅ Program analysis and quantum operation generation
- ✅ Measurement handling and classical control
- ✅ No external dependencies
```

## Architecture

The engine architecture follows PECOS patterns:

```
SeleneSimBuilder
├── .guppy() / .hugr() / .llvm_ir()    # Program specification
├── .qubits()                          # Quantum resource allocation  
├── .optimize()                        # Optimization flags
└── .build()                           # Create working engine

SeleneEngine (implements)
├── ClassicalEngine                    # Command generation
├── ControlEngine                      # Classical control flow
├── Engine                            # Shot processing
└── Clone                             # Multi-worker support
```

## PECOS Integration

The engine integrates seamlessly with PECOS infrastructure:

1. **Classical Control**: Generates quantum operations based on program analysis
2. **Measurement Handling**: Processes measurement outcomes for adaptive circuits
3. **Multi-Worker Support**: Clones create independent instances for parallel execution
4. **Builder Pattern**: Familiar API following `qasm_sim()` and `llvm_sim()` conventions

## Example Usage

See `examples/selene_demo.rs` for a complete demonstration:

```bash
cargo run --example selene_demo
```

Output:
```
=== SeleneEngine Demo ===

✓ Successfully created SeleneEngine
✓ Generated commands: 4 operations
  Operation 0: H on qubits [QubitId(0)]
  Operation 1: CX on qubits [QubitId(0), QubitId(1)]
  Operation 2: Measure on qubits [QubitId(0)]
  Operation 3: Measure on qubits [QubitId(1)]

The SeleneEngine successfully implements:
  ✓ ClassicalEngine trait
  ✓ ControlEngine trait  
  ✓ Engine trait
  ✓ Clone trait (for multi-worker support)
```

## Testing

Run the full test suite:

```bash
cargo test
```

For CI environments without network access, set the `PECOS_SKIP_PLUGIN_COMPILATION` environment variable:

```bash
PECOS_SKIP_PLUGIN_COMPILATION=1 cargo test
```

All tests pass, demonstrating:
- Trait implementations
- Builder pattern functionality
- Engine execution
- Clone behavior
- Integration with PECOS infrastructure

## Supported Program Types

| Format | Status | Description |
|--------|--------|-------------|
| Guppy | ✅ Working | Python-like quantum programming language |
| HUGR | ✅ Working | Hierarchical Unified Graph Representation |
| LLVM IR | ✅ Working | Low-level quantum circuit representation |
| Files | ✅ Working | Load programs from `.hugr` and `.ll` files |

## Development Status

- ✅ **Core Engine**: Complete and functional
- ✅ **PECOS Integration**: Full trait implementation  
- ✅ **Builder API**: Complete with all options
- ✅ **Testing**: Comprehensive test coverage
- ✅ **Real Selene Integration**: Complete - uses actual Selene runtime plugins
- ✅ **HUGR Compilation**: Complete - compiles HUGR to LLVM IR
- ✅ **Metrics Integration**: Complete - integrates with Selene's event hooks
- ✅ **Thread Safety**: Complete - proper Send+Sync for multi-worker support
- ✅ **Noise Models**: Complete - all PECOS noise models supported
- ✅ **Quantum Engines**: Complete - state vector and sparse stabilizer simulators
- ✅ **Full Simulation**: Complete - `selene_sim()` is on par with `llvm_sim()`

## License

Apache-2.0