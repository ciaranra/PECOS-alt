#!/usr/bin/env python3
"""Try to replicate exact test setup"""

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

# Try to run with different shot counts
for shots in [1, 2, 5, 10, 50]:
    print(f"\n=== Testing with {shots} shots ===")
    try:
        result = run_guppy(hadamard_test, shots=shots, verbose=False, backend="rust")
        print(f"✓ Success with {shots} shots")
    except Exception as e:
        print(f"✗ Failed with {shots} shots: {e}")