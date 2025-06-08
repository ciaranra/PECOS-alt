# QASM Simulation API

The `qasm_sim` API provides a fluent builder interface for running QASM simulations with various configurations including noise models, quantum engines, and execution parameters.

## Quick Start

```rust
use pecos_qasm::prelude::*;

// Simple simulation
let results = qasm_sim(qasm_code).run(1000)?;

// With configuration
let results = qasm_sim(qasm_code)
    .seed(42)
    .noise(DepolarizingNoise { p: 0.01 })
    .run(1000)?;
```

## Builder Pattern

The API uses a builder pattern that allows you to chain configuration methods:

```rust
qasm_sim(qasm_code)
    .seed(42)                    // Set random seed
    .workers(4)                  // Number of threads
    .auto_workers()              // Or auto-detect CPU cores
    .quantum_engine(engine)      // Simulation backend
    .noise(noise_config)         // Noise model
    .build()?                    // Build reusable simulation
    .run(shots)?                 // Run simulation
```

## Build Once, Run Multiple Times

You can build a simulation once and run it multiple times with different shot counts:

```rust
let sim = qasm_sim(qasm_code)
    .seed(42)
    .noise(DepolarizingNoise { p: 0.01 })
    .build()?;

// Run multiple times with different shots
let results_100 = sim.run(100)?;
let results_1000 = sim.run(1000)?;
let results_10000 = sim.run(10000)?;
```

## Noise Models

PECOS provides a unified approach to noise configuration. All noise models can be specified using either:
1. **Config structs** - Simple POD-style structs for common cases
2. **Builders** - For complex configurations with many parameters
3. **Direct enum variants** - When you need explicit control

All three approaches work seamlessly with both `qasm_sim()` and `run_qasm()` APIs.

### No Noise (Default)
```rust
qasm_sim(qasm).noise(PassThroughNoise)
```

### Depolarizing Noise
```rust
// Uniform depolarizing
qasm_sim(qasm).noise(DepolarizingNoise { p: 0.01 })

// Custom depolarizing per operation type
qasm_sim(qasm).noise(DepolarizingCustomNoise {
    p_prep: 0.001,  // State preparation error
    p_meas: 0.002,  // Measurement error
    p1: 0.003,      // Single-qubit gate error
    p2: 0.004,      // Two-qubit gate error
})
```

### Biased Noise Models
```rust
// Biased depolarizing
qasm_sim(qasm).noise(BiasedDepolarizingNoise { p: 0.01 })

// Biased measurement (asymmetric bit flips)
qasm_sim(qasm).noise(BiasedMeasurementNoise {
    p0: 0.01,  // Probability of 0→1 flip
    p1: 0.02,  // Probability of 1→0 flip
})
```

### General Noise Model

Simple usage:
```rust
qasm_sim(qasm).noise(GeneralNoise)
```

Advanced configuration with builder:
```rust
// Create a custom noise model using the builder
let noise_builder = GeneralNoiseModel::builder()
    .with_prep_probability(0.001)        // State prep error
    .with_meas_0_probability(0.005)      // Measurement error |0> -> |1>
    .with_meas_1_probability(0.01)       // Measurement error |1> -> |0>
    .with_p1_probability(0.0001)         // Single-qubit gate error
    .with_p2_probability(0.01)           // Two-qubit gate error
    .with_seed(42);                      // Noise RNG seed

qasm_sim(qasm).noise(noise_builder)
```

The builder provides many more configuration options:
- Idle noise rates and models
- Leakage and emission probabilities
- Custom Pauli error distributions
- Crosstalk effects
- Gate-specific error rates

## Quantum Engines

Choose between different simulation backends:

```rust
// State vector simulator (default for non-Clifford circuits)
qasm_sim(qasm).quantum_engine(QuantumEngineType::StateVector)

// Sparse stabilizer simulator (default, efficient for Clifford circuits)
qasm_sim(qasm).quantum_engine(QuantumEngineType::SparseStabilizer)
```

## Worker Configuration

Control parallelization:

```rust
// Single threaded (default)
qasm_sim(qasm).workers(1)

// Explicit thread count
qasm_sim(qasm).workers(4)

// Auto-detect CPU cores
qasm_sim(qasm).auto_workers()
```

## Deterministic Simulations

For reproducible results, set a seed:

```rust
let results = qasm_sim(qasm).seed(42).run(1000)?;
```

## Result Format

The simulation returns a `ShotVec` which can be converted to different formats:

```rust
let shot_vec = qasm_sim(qasm).run(1000)?;

// Convert to ShotMap for columnar access
let shot_map = shot_vec.try_as_shot_map()?;

// Access measurement results by register name
let c_register_values = shot_map.try_bits_as_u64("c")?;
```

## Complete Examples

### Using Config Structs

```rust
use pecos_qasm::prelude::*;

fn run_bell_state_simulation() -> Result<(), PecosError> {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    // Build simulation with noise
    let sim = qasm_sim(qasm)
        .seed(42)
        .workers(4)
        .quantum_engine(QuantumEngineType::StateVector)
        .noise(DepolarizingNoise { p: 0.01 })
        .build()?;

    // Run multiple times
    for shots in [100, 1000, 10000] {
        let results = sim.run(shots)?;
        let shot_map = results.try_as_shot_map()?;

        println!("Results for {} shots:", shots);
        println!("{}", shot_map.display());
    }

    Ok(())
}
```

### Using Noise Model Builders

```rust
use pecos_qasm::prelude::*;
use pecos_engines::noise::GeneralNoiseModel;

fn run_advanced_noise_simulation() -> Result<(), PecosError> {
    let qasm = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[3];
        creg c[3];
        h q[0];
        cx q[0], q[1];
        cx q[1], q[2];
        measure q -> c;
    "#;

    // Create advanced noise model with builder
    let noise = GeneralNoiseModel::builder()
        .with_prep_probability(0.001)      // 0.1% state prep error
        .with_p1_probability(0.0001)       // 0.01% single-qubit gate error
        .with_p2_probability(0.01)         // 1% two-qubit gate error
        .with_meas_0_probability(0.02)     // 2% false positive rate
        .with_meas_1_probability(0.03)     // 3% false negative rate
        .with_seed(12345);                 // Deterministic noise

    // Use with run_qasm
    let results = run_qasm(qasm, 1000, noise, None, Some(4), Some(42))?;

    // Or with qasm_sim builder
    let results2 = qasm_sim(qasm)
        .noise(noise.clone())
        .workers(4)
        .seed(42)
        .run(1000)?;

    Ok(())
}
```

## Error Handling

All methods that can fail return `Result<T, PecosError>`:

- `build()` - Can fail during QASM parsing
- `run()` - Can fail during simulation execution
- `try_as_shot_map()` - Can fail during result conversion

## Performance Tips

1. **Build once, run multiple times**: Parse QASM once and reuse the simulation
2. **Use auto_workers()** for CPU-bound simulations with many shots
3. **Choose the right engine**:
   - `SparseStabilizer` for Clifford-only circuits (very fast)
   - `StateVector` for circuits with non-Clifford gates
4. **Batch similar simulations**: Use the same noise model and engine settings when possible
