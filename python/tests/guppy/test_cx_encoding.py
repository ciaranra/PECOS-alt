#!/usr/bin/env python3
"""Test CX gate result encoding."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x, cx
from pecos.frontends import guppy_sim

@guppy
def test_cx_tuple() -> tuple[bool, bool]:
    """Test CX returning tuple."""
    q1 = qubit()
    q2 = qubit()
    x(q1)  # Set control to |1⟩
    cx(q1, q2)  # CNOT
    return measure(q1), measure(q2)

# Run the test
try:
    results = guppy_sim(test_cx_tuple, max_qubits=10).run(10)
    print(f"Raw results: {results}")
    
    # Decode the integer results
    for val in results['_result']:
        bit0 = bool(val & 1)  # First qubit
        bit1 = bool(val & 2)  # Second qubit
        print(f"Value {val}: q1={bit0}, q2={bit1}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()