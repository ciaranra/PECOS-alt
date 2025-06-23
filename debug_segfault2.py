#!/usr/bin/env python3
"""Debug script to test with multiple qubits"""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit, cx
from pecos.frontends.run_guppy import run_guppy

# Test 1: Two independent qubits
@guppy
def two_qubits() -> tuple[bool, bool]:
    """Two independent qubits"""
    q1 = qubit()
    q2 = qubit()
    return measure(q1), measure(q2)

print("=== Test 1: Two independent qubits ===")
try:
    result = run_guppy(two_qubits, shots=2, verbose=True, backend="rust")
    print(f"✓ Success! Results: {result['results']}")
except Exception as e:
    print(f"✗ Failed: {e}")

# Test 2: Bell state
@guppy
def bell_state() -> tuple[bool, bool]:
    """Create a Bell state"""
    q1 = qubit()
    q2 = qubit()
    h(q1)
    cx(q1, q2)
    return measure(q1), measure(q2)

print("\n=== Test 2: Bell state ===")
try:
    result = run_guppy(bell_state, shots=2, verbose=True, backend="rust")
    print(f"✓ Success! Results: {result['results']}")
except Exception as e:
    print(f"✗ Failed: {e}")

print("\nAll tests completed.")