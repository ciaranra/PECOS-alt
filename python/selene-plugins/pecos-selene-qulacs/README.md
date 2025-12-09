# PECOS Qulacs Selene Plugin

A Qulacs state vector simulator plugin for the [Selene](https://github.com/CQCL/selene) quantum emulator using the PECOS Qulacs wrapper.

## Overview

This plugin provides a Qulacs state vector simulator backend for Selene, using the PECOS Qulacs wrapper. Qulacs is a high-performance quantum simulator that supports arbitrary rotation angles.

The memory requirement scales exponentially with the number of qubits (16 bytes * 2^n_qubits).

## Installation

```bash
pip install pecos-selene-qulacs
```

## Usage

```python
from selene_sim.build import build
from pecos_selene_qulacs import QulacsPlugin

# Create a plugin instance
simulator = QulacsPlugin()

# Use with Selene
runner = build(program)
results = list(runner.run_shots(
    simulator=simulator,
    n_qubits=10,
    n_shots=1000,
))
```

## Parameters

- `random_seed` (int, optional): Seed for the random number generator for deterministic results.

## Building from Source

This package requires Rust and the Qulacs C++ library to build. The Rust components will be automatically compiled during installation.

```bash
# From the PECOS repository root
cd python/pecos-selene-qulacs
pip install -e ".[test]"
```

## Running Tests

```bash
pytest tests/
```

## License

Apache-2.0
