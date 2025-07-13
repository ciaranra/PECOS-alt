#!/usr/bin/env python3
"""Test discard operation."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x, h, discard
from pecos.frontends import guppy_sim

@guppy
def test_discard_simple() -> bool:
    """Simplest discard test."""
    q1 = qubit()
    discard(q1)
    q2 = qubit()
    x(q2)
    return measure(q2)

# Run the test
try:
    results = guppy_sim(test_discard_simple, max_qubits=10).run(10)
    print(f"Results: {results}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()