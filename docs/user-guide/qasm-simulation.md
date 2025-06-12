# Running QASM Simulations with PECOS

This guide will walk you through running quantum circuit simulations using PECOS's QASM interface. Whether you're simulating ideal quantum circuits or studying the effects of noise, PECOS provides the tools you need.

## What You'll Learn

- How to run your first QASM simulation
- Adding realistic noise models to your circuits
- Optimizing performance for large simulations
- Analyzing simulation results
- Choosing the right simulation engine for your needs

## Getting Started: Your First Simulation

Let's start with a simple example - creating and measuring a Bell state. First, we'll define our QASM code, which creates a Bell state by applying a Hadamard gate to the first qubit and then a CNOT gate to entangle both qubits:

```qasm
OPENQASM 2.0;
include "qelib1.inc";
qreg q[2];
creg c[2];
h q[0];
cx q[0], q[1];
measure q -> c;
```

Now, let's run this code using PECOS's simple `run_qasm` function:

=== "Rust"

    ```rust
    use pecos_qasm::prelude::*;

    // Define the Bell state QASM code
    let qasm_code = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    // Simple simulation with just shots count
    let num_shots = 1000;
    let results = run_qasm(qasm_code, num_shots, None, None, None, None)?;

    // With configuration using named variables for clarity
    let num_shots = 1000;
    let noise_model = Some(DepolarizingNoise { p: 0.01 });
    let engine_type = None;  // Use default (SparseStabilizer for this circuit)
    let worker_count = None; // Use default (1 thread) or Some(4) for 4 threads
    let random_seed = Some(42);

    let results = run_qasm(
        qasm_code,
        num_shots,
        noise_model,
        engine_type,
        worker_count,
        random_seed
    )?;
    ```

=== "Python"

    ```python
    from pecos.rslib import run_qasm, DepolarizingNoise

    # Define the Bell state QASM code
    qasm_code = '''
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    '''

    # Simple simulation
    results = run_qasm(qasm_code, shots=1000)

    # With configuration
    results = run_qasm(
        qasm_code,
        shots=1000,
        noise_model=DepolarizingNoise(p=0.01),
        seed=42
    )
    ```

## Using the Builder API

For more complex simulations or when you need finer control, you can use the builder-style API. This approach offers more flexibility, including the ability to automatically use all available CPU cores with `auto_workers()`, which isn't available in the simple `run_qasm` function:

=== "Rust"

    ```rust
    use pecos_qasm::prelude::*;

    // Define the Bell state QASM code (as above)
    let qasm_code = r#"
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    "#;

    // Simple simulation with builder pattern
    let results = qasm_sim(qasm_code).run(1000)?;

    // With more configuration options
    let results = qasm_sim(qasm_code)
        .seed(42)
        .noise(DepolarizingNoise { p: 0.01 })
        .workers(4)        // Explicitly set number of threads
        // .auto_workers() // Or use all available CPU cores
        .run(1000)?;
    ```

=== "Python"

    ```python
    from pecos.rslib import qasm_sim, DepolarizingNoise

    # Define the Bell state QASM code (as above)
    qasm_code = '''
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    '''

    # Simple simulation with builder pattern
    results = qasm_sim(qasm_code).run(1000)

    # With more configuration options
    results = (qasm_sim(qasm_code)
        .seed(42)
        .noise(DepolarizingNoise(p=0.01))
        .workers(4)             # Explicitly set number of threads
        # .auto_workers()       # Or use all available CPU cores
        .run(1000))
    ```

## Running Multiple Shots

Real quantum computers run circuits multiple times ("shots") to build up statistics. PECOS simulates this behavior and
lets you build the experiment once and rerun it multiple times:

=== "Rust"

    ```rust
    let sim = qasm_sim(qasm_code)
        .seed(42)                    // Set random seed
        .workers(4)                  // Number of threads
        .auto_workers()              // Or auto-detect CPU cores
        .quantum_engine(engine)      // Simulation backend
        .noise(noise_config)         // Noise model
        .build()?;                   // Build reusable simulation

    // Run multiple times
    let results_100 = sim.run(100)?;
    let results_1000 = sim.run(1000)?;
    ```

=== "Python"

    ```python
    # Build once, run multiple times
    sim = (qasm_sim(qasm)
        .seed(42)
        .noise(DepolarizingNoise(p=0.01))
        .workers(4)
        .build())

    # Run with different shot counts
    results_100 = sim.run(100)
    results_1000 = sim.run(1000)
    ```

## Adding Noise to Your Simulations

Real quantum computers are noisy. PECOS helps you understand how noise affects your circuits by providing several noise models.

### Common Noise Types

=== "Rust"

    ```rust
    // No noise (ideal simulation)
    PassThroughNoise

    // Standard depolarizing
    DepolarizingNoise { p: 0.01 }

    // Custom depolarizing per operation type
    DepolarizingCustomNoise {
        p_prep: 0.001,  // State preparation error
        p_meas: 0.002,  // Measurement error
        p1: 0.003,      // Single-qubit gate error
        p2: 0.004,      // Two-qubit gate error
    }

    // Biased depolarizing (asymmetric error distribution)
    BiasedDepolarizingNoise { p: 0.01 }
    ```

=== "Python"

    ```python
    # No noise (ideal simulation)
    PassThroughNoise()

    # Standard depolarizing
    DepolarizingNoise(p=0.01)

    # Custom depolarizing per operation type
    DepolarizingCustomNoise(
        p_prep=0.001,  # State preparation error
        p_meas=0.002,  # Measurement error
        p1=0.003,      # Single-qubit gate error
        p2=0.004       # Two-qubit gate error
    )

    # Biased depolarizing (asymmetric error distribution)
    BiasedDepolarizingNoise(p=0.01)
    ```

### Creating Custom Noise Models

For research or to match specific hardware characteristics, you can create detailed noise models:

=== "Rust"

    ```rust
    use pecos_engines::noise::GeneralNoiseModel;

    let noise = GeneralNoiseModel::builder()
        .with_prep_probability(0.001)      // State prep error
        .with_meas_0_probability(0.005)    // Measurement error |0> → |1>
        .with_meas_1_probability(0.01)     // Measurement error |1> → |0>
        .with_p1_probability(0.0001)       // Single-qubit gate error
        .with_p2_probability(0.01)         // Two-qubit gate error
        .with_idle_linear_rate(0.0001)     // Idle noise rate
        .with_seed(42);                    // Deterministic noise

    // Use with either API
    qasm_sim(qasm).noise(noise)
    run_qasm(qasm, 1000, noise, None, None, None)?
    ```

=== "Python"

    ```python
    # Note: Python bindings for builders are planned for future release
    # Currently, use the dataclasses above or the Rust API for advanced configurations

    # Future API (not yet available):
    # noise = (GeneralNoiseModelBuilder()
    #     .with_prep_probability(0.001)
    #     .with_meas_0_probability(0.005)
    #     .with_meas_1_probability(0.01)
    #     .with_p1_probability(0.0001)
    #     .with_p2_probability(0.01)
    #     .build())
    ```

The builder provides many configuration options:
- Idle noise rates and models
- Leakage and emission probabilities
- Custom Pauli error distributions
- Crosstalk effects
- Gate-specific error rates
- Coherent vs. incoherent noise

## Choosing the Right Simulation Engine

PECOS provides different engines optimized for different types of circuits:

=== "Rust"

    ```rust
    // Sparse stabilizer (default, efficient for Clifford circuits)
    QuantumEngineType::SparseStabilizer

    // State vector (for non-Clifford circuits)
    QuantumEngineType::StateVector
    ```

=== "Python"

    ```python
    from pecos.rslib import QuantumEngine

    # Sparse stabilizer (default, efficient for Clifford circuits)
    QuantumEngine.SparseStabilizer

    # State vector (for non-Clifford circuits)
    QuantumEngine.StateVector
    ```

## Understanding Your Results

Simulation results come back as measurement outcomes for each shot. These can be processed in different ways depending on your needs:

=== "Rust"

    ```rust
    let shot_vec = qasm_sim(qasm).run(1000)?;

    // Convert to ShotMap for columnar access
    let shot_map = shot_vec.try_as_shot_map()?;

    // Access measurement results by register name
    let c_values = shot_map.try_bits_as_u64("c")?;
    // Returns Vec<u64> where each value is the decimal encoding

    // Or get results as binary strings
    let results = qasm_sim(qasm)
        .with_binary_string_format()
        .run(1000)?;
    let shot_map = results.try_as_shot_map()?;
    let binary_values = shot_map.try_bits_as_binary("c")?;
    // Returns Vec<String> where each string is like "00", "11", etc.
    ```

=== "Python"

    ```python
    results = run_qasm(qasm, shots=1000)

    # Returns a dictionary with register names as keys and measurement lists as values
    print(results)
    # {"c": [0, 3, 0, 3, ...]}  # List of measurement outcomes

    # Each value is the decimal encoding of the binary string:
    # 0 = 00 (both qubits in |0⟩)
    # 1 = 01
    # 2 = 10
    # 3 = 11 (both qubits in |1⟩)

    # Count the occurrences of each measurement outcome
    from collections import Counter
    counts = Counter(results["c"])
    print(counts)  # {0: 492, 3: 508} for an ideal Bell state

    # Or get results as binary strings
    results = qasm_sim(qasm)
        .with_binary_string_format()
        .run(1000)
    print(results)
    # {"c": ["00", "11", "00", "11", ...]}  # Binary string format

    # Count binary string outcomes
    counts = Counter(results["c"])
    print(counts)  # {"00": 492, "11": 508} for an ideal Bell state
    ```

    The Python API returns results in columnar format, with each register name mapping to a list of values. By default, these are integer values (decimal encoding of the binary strings). With `.with_binary_string_format()`, you get the binary strings directly.

    For large registers (>64 qubits), integer results are automatically converted to Python's arbitrary-precision integers.

## Practical Examples

### Example 1: Studying Noise Effects on Bell States

This example shows how noise affects quantum entanglement:

=== "Rust"

    ```rust
    use pecos_qasm::prelude::*;

    fn bell_state_example() -> Result<(), PecosError> {
        let qasm = r#"
            OPENQASM 2.0;
            include "qelib1.inc";
            qreg q[2];
            creg c[2];
            h q[0];
            cx q[0], q[1];
            measure q -> c;
        "#;

        // Build simulation with depolarizing noise
        let sim = qasm_sim(qasm)
            .seed(42)
            .workers(4)
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

=== "Python"

    ```python
    from pecos.rslib import run_qasm, qasm_sim, DepolarizingNoise
    from collections import Counter

    qasm = '''
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
    '''

    # Build simulation with depolarizing noise
    sim = qasm_sim(qasm) \
        .seed(42) \
        .workers(4) \
        .noise(DepolarizingNoise(p=0.01)) \
        .build()

    # Run multiple times
    for shots in [100, 1000, 10000]:
        results = sim.run(shots)
        print(f"Results for {shots} shots:")
        print(f"Counts: {Counter(results['c'])}")
    ```

### Example 2: Simulating a Noisy Quantum Algorithm

Here's how to simulate a small quantum algorithm with realistic noise:

```rust
use pecos_qasm::prelude::*;
use pecos_engines::noise::GeneralNoiseModel;

fn advanced_noise_example() -> Result<(), PecosError> {
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
        .with_idle_linear_rate(0.00001)    // Small idle noise
        .with_seed(12345);                 // Deterministic noise

    // Run simulation
    let results = run_qasm(qasm, 1000, noise, None, Some(4), Some(42))?;

    let shot_map = results.try_as_shot_map()?;
    println!("GHZ state results with complex noise:");
    println!("{}", shot_map.display());

    Ok(())
}
```

## Optimizing Your Simulations

### When to Parse Once

If you're running the same circuit with different parameters:

=== "Rust"

    ```rust
    // Parse once
    let sim = qasm_sim(qasm).build()?;

    // Run many times
    for noise_level in [0.001, 0.01, 0.1] {
        let noisy_sim = sim.clone().noise(DepolarizingNoise { p: noise_level });
        let results = noisy_sim.run(1000)?;
        analyze_results(results);
    }
    ```

=== "Python"

    ```python
    # Parse once
    sim = qasm_sim(qasm).build()

    # Run many times
    for noise_level in [0.001, 0.01, 0.1]:
        results = sim.noise(DepolarizingNoise(p=noise_level)).run(1000)
        analyze_results(results)
    ```

### Parallel Execution

For many shots, you can use multiple CPU cores to speed up simulation:

=== "Rust"

    ```rust
    // Single threaded (default for run_qasm)
    let results = qasm_sim(qasm).workers(1).run(100000)?;

    // Explicit thread count
    let results = qasm_sim(qasm).workers(4).run(100000)?;

    // Automatically use all available cores
    let results = qasm_sim(qasm).auto_workers().run(100000)?;
    ```

=== "Python"

    ```python
    # Default is single-threaded for run_qasm
    results = run_qasm(qasm, shots=100000)

    # Use 4 worker threads
    results = run_qasm(qasm, shots=100000, workers=4)

    # For auto-detection, use the builder API
    results = qasm_sim(qasm).auto_workers().run(100000)
    ```

### Choosing the Right Engine

- **For Clifford circuits** (H, S, CNOT, measurements): Use `SparseStabilizer` - it's exponentially faster
- **For circuits with T gates or rotations**: Use `StateVector`
- **Not sure?** The engine will be chosen based on the gates in your circuit

## Common Issues and Solutions

### Handling Errors

=== "Rust"

    All methods return `Result<T, PecosError>`:

    - `build()` - Can fail during QASM parsing
    - `run()` - Can fail during simulation execution
    - `try_as_shot_map()` - Can fail during result conversion

=== "Python"

    The API raises `RuntimeError` for invalid operations:
    ```python
    try:
        results = run_qasm("invalid qasm", shots=10)
    except RuntimeError as e:
        print(f"Error: {e}")
    ```

### Additional Python Utilities

Python provides some additional utility functions for working with the QASM simulator:

```python
from pecos.rslib import get_noise_models, get_quantum_engines

# Get list of available noise model names
noise_models = get_noise_models()
print(noise_models)  # ['PassThrough', 'Depolarizing', 'DepolarizingCustom', ...]

# Get list of available quantum engine names
engines = get_quantum_engines()
print(engines)  # ['StateVector', 'SparseStabilizer']
```

These functions are useful for dynamically listing available options in applications or for validating user input.

## Working with Large Circuits

### Circuits with Many Qubits

PECOS automatically handles circuits with more than 64 qubits:

=== "Rust"

    ```rust
    // Results automatically use BigUint for large registers
    let values = shot_map.try_bits_as_biguint("large_reg")?;
    ```

=== "Python"

    ```python
    # Results automatically converted to Python big integers
    results = run_qasm(qasm_large, shots=10)
    # results["c"] will contain Python arbitrary-precision integers
    ```

## Next Steps

- **Learn more about QASM**: [OpenQASM 2.0 Specification](https://arxiv.org/abs/1707.03429)
- **Explore quantum algorithms**: Try implementing Grover's algorithm or QFT
- **Study noise**: Experiment with different noise models to understand their effects
- **Optimize performance**: Profile your simulations and choose appropriate engines

## Further Reading

- [Getting Started with PECOS](../user-guide/getting-started.md)
- [Understanding Quantum Noise](https://quantum-computing.ibm.com/composer/docs/iqx/guide/error-mitigation)
- [PECOS Development Guide](../development/DEVELOPMENT.md)
