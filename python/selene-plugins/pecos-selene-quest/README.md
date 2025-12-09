# PECOS Quest Selene Plugin

A Quest state vector simulator plugin for the [Selene](https://github.com/CQCL/selene) quantum emulator using the PECOS Quest wrapper.

## Overview

This plugin provides a Quest state vector simulator backend for Selene, using the PECOS Quest wrapper. Quest is a high-performance quantum simulator that supports arbitrary rotation angles and can utilize GPU acceleration when available.

The memory requirement scales exponentially with the number of qubits (16 bytes * 2^n_qubits).

## Installation

```bash
pip install pecos-selene-quest
```

## Usage

```python
from selene_sim.build import build
from pecos_selene_quest import QuestPlugin

# Create a plugin instance
simulator = QuestPlugin()

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

This package requires Rust and the Quest C library to build. The Rust components will be automatically compiled during installation.

```bash
# From the PECOS repository root
cd python/pecos-selene-quest
pip install -e ".[test]"
```

## Running Tests

```bash
pytest tests/
```

## Thread Safety Warning

Quest has a fundamental limitation - it uses a single global environment per process. This means all Quest instances share the same underlying environment, which can lead to race conditions when used concurrently from multiple threads. For safe usage:
- Run tests with `--test-threads=1`
- Use only one Quest instance per process in production

## License

Apache-2.0
