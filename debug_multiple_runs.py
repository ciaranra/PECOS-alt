#!/usr/bin/env python3
"""Test running multiple times like the test does"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.frontends.run_guppy import run_guppy

@guppy
def hadamard_test() -> bool:
    q = qubit()
    h(q)
    return measure(q)

print("=== Running with rust backend first ===")
try:
    result1 = run_guppy(hadamard_test, shots=50, backend="rust", verbose=True, seed=42)
    print(f"✓ Rust backend success")
except Exception as e:
    print(f"✗ Rust backend failed: {e}")

print("\n=== Running with external backend second ===")
try:
    result2 = run_guppy(hadamard_test, shots=50, backend="external", verbose=True, seed=42)
    print(f"✓ External backend success")
except Exception as e:
    print(f"✗ External backend failed: {e}")

print("\nBoth runs completed.")