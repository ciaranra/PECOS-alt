# QASM Simulation API Usage Guide

This guide demonstrates how to use the PECOS QASM simulation API.

## Overview

The `pecos_rslib.qasm_sim` module provides a clean Pythonic interface for running QASM simulations with support for various noise models and quantum engines.

## Example Usage

### Basic Setup

```python
from pecos_rslib.qasm_sim import (
    run_qasm,
    DepolarizingNoise,
    QuantumEngine
)

# Example QASM circuit - Bell state
qasm = '''
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[2];
    creg c[2];
    h q[0];
    cx q[0], q[1];
    measure q -> c;
'''
```

### Running Simulations

Using dataclasses for noise models and clean parameter passing:

```python
results = run_qasm(
    qasm,
    shots=1000,
    noise_model=DepolarizingNoise(p=0.01),
    engine=QuantumEngine.SparseStabilizer,
    workers=4,
    seed=42
)
# Returns columnar format: {"c": [0, 3, 0, 3, ...]}
```

## Noise Models

### Simple Noise Models

Available noise model dataclasses:

- `PassThroughNoise()` - No noise (ideal simulation)
- `DepolarizingNoise(p=0.001)` - Standard depolarizing noise
- `DepolarizingCustomNoise(p_prep=0.001, p_meas=0.001, p1=0.001, p2=0.002)` - Custom depolarizing
- `BiasedDepolarizingNoise(p=0.001)` - Biased depolarizing noise
- `BiasedMeasurementNoise(p0=0.01, p1=0.01)` - Biased measurement errors
- `GeneralNoise()` - General noise model

### Advanced Noise Models with Builders

For complex noise configurations, you can use the GeneralNoiseModelBuilder (note: Python bindings for builders are planned for future release):

```python
# Future API (not yet available in Python):
# noise = GeneralNoiseModelBuilder() \
#     .with_prep_probability(0.001) \
#     .with_meas_0_probability(0.005) \
#     .with_meas_1_probability(0.01) \
#     .with_p1_probability(0.0001) \
#     .with_p2_probability(0.01) \
#     .build()
```

Currently, use the dataclasses above or the Rust API for advanced noise configurations.

## Quantum Engines

- `QuantumEngine.StateVector` - Full state vector simulation
- `QuantumEngine.SparseStabilizer` - Efficient stabilizer simulation (default)

## Return Format

The new `run_qasm()` function returns results in columnar format:

```python
# Example return value for a 2-qubit measurement
{
    "c": [0, 3, 0, 3, 0, 3, ...]  # List of measurement outcomes
}
```

Where each value represents the decimal encoding of the measured bit string:
- `0` = `00` (both qubits in |0⟩)
- `1` = `01`
- `2` = `10`
- `3` = `11` (both qubits in |1⟩)

## Builder Pattern

The `qasm_sim()` function provides a builder pattern for creating reusable simulations:

```python
# Build once, run multiple times
sim = qasm_sim(qasm) \
    .seed(42) \
    .noise(DepolarizingNoise(p=0.01)) \
    .engine(QuantumEngine.SparseStabilizer) \
    .workers(4) \
    .build()

# Run with different shot counts
results_100 = sim.run(100)
results_1000 = sim.run(1000)

# Or run directly without building
results = qasm_sim(qasm) \
    .noise(DepolarizingNoise(p=0.01)) \
    .run(1000)
```

## Large Registers

For quantum registers larger than 64 qubits, the results are automatically converted to Python's arbitrary-precision integers:

```python
qasm_large = '''
    OPENQASM 2.0;
    include "qelib1.inc";
    qreg q[100];
    creg c[100];
    measure q -> c;
'''

results = run_qasm(qasm_large, shots=10)
# results["c"] will contain Python big integers
```

## Parallel Execution

Use the `workers` parameter to enable parallel shot execution:

```python
# Run 10,000 shots across 4 worker threads
results = run_qasm(
    qasm,
    shots=10000,
    workers=4,
    seed=42  # Ensures deterministic results even with parallelism
)
```

## Available Functions

### Main Functions

- `run_qasm(qasm, shots, noise_model=None, engine=None, workers=None, seed=None)` - Run a QASM simulation
- `qasm_sim(qasm)` - Create a simulation builder for flexible configuration
- `get_noise_models()` - Get list of available noise model names
- `get_quantum_engines()` - Get list of available quantum engine names

### Enums

- `NoiseModel` - Noise model type enum
- `QuantumEngine` - Quantum engine type enum

## Error Handling

The API will raise a `RuntimeError` for invalid QASM code or unsupported operations:

```python
try:
    results = run_qasm("invalid qasm", shots=10)
except RuntimeError as e:
    print(f"Error: {e}")
```

## See Also

- [QASM Simulation Example](../examples/qasm_sim_example.py) - Comprehensive usage examples
- [PECOS Documentation](https://github.com/CQCL/PECOS) - Main PECOS documentation
