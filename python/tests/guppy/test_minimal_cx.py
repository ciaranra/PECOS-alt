#!/usr/bin/env python3
"""Minimal test to isolate CX gate issue."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x, cx
from pecos.frontends import guppy_sim

@guppy
def test_cx_simple() -> bool:
    """Simplest possible CX test."""
    q1 = qubit()
    q2 = qubit()
    x(q1)  # Set control to |1⟩
    cx(q1, q2)  # CNOT
    # Measure only target qubit
    measure(q1)  # Discard control measurement
    return measure(q2)  # Should be |1⟩

# Run the test
try:
    results = guppy_sim(test_cx_simple, max_qubits=10).run(10)
    print(f"Results: {results}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()