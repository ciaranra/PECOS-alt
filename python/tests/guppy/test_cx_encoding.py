#!/usr/bin/env python3
"""Test CX gate result encoding."""

import sys
from typing import List, Tuple


def decode_integer_results(results: List[int], n_bits: int) -> List[Tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded

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
    for val in results['result']:
        bit0 = bool(val & 1)  # First qubit
        bit1 = bool(val & 2)  # Second qubit
        print(f"Value {val}: q1={bit0}, q2={bit1}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()