#!/usr/bin/env python3
"""Simple test to debug single qubit gates."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, h, x, y, z
from pecos.frontends import guppy_sim

@guppy
def test_simple_gates() -> tuple[bool, bool]:
    """Test simple gate combination."""
    q1 = qubit()
    h(q1)
    x(q1)
    r1 = measure(q1)
    
    q2 = qubit()
    x(q2)
    r2 = measure(q2)
    
    return r1, r2

# Run the test
try:
    results = guppy_sim(test_simple_gates, max_qubits=10).run(10)
    print(f"Results: {results}")
    
    # Decode integer results back to tuples
    decoded = []
    for val in results['_result']:
        bit0 = bool(val & 1)
        bit1 = bool(val & 2)
        decoded.append((bit0, bit1))
    
    print(f"Decoded results: {decoded}")
    print(f"All results as expected: {all(r == (False, True) for r in decoded)}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()