#!/usr/bin/env python3
"""Test if 4-tuple of bools causes segfault."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim


@guppy
def test_four_bool_tuple() -> tuple[bool, bool, bool, bool]:
    """Return 4-tuple of bools."""
    q1 = qubit()
    x(q1)
    r1 = measure(q1)
    
    q2 = qubit()
    x(q2)
    r2 = measure(q2)
    
    q3 = qubit()
    r3 = measure(q3)
    
    q4 = qubit()
    x(q4)
    r4 = measure(q4)
    
    return r1, r2, r3, r4


if __name__ == "__main__":
    print("Testing 4-tuple of bools...")
    results = guppy_sim(test_four_bool_tuple, max_qubits=10).run(10)
    print(f"Results: {results}")
    print("Test passed!")