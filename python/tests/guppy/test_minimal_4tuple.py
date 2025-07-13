#!/usr/bin/env python3
"""Minimal test to reproduce 4-tuple segfault."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure
from pecos.frontends import guppy_sim


@guppy
def minimal_4tuple() -> tuple[bool, bool, bool, bool]:
    """Minimal 4-tuple test."""
    q1 = qubit()
    r1 = measure(q1)
    
    q2 = qubit()
    r2 = measure(q2)
    
    q3 = qubit()
    r3 = measure(q3)
    
    q4 = qubit()
    r4 = measure(q4)
    
    return r1, r2, r3, r4


@guppy
def minimal_3tuple() -> tuple[bool, bool, bool]:
    """Minimal 3-tuple test."""
    q1 = qubit()
    r1 = measure(q1)
    
    q2 = qubit()
    r2 = measure(q2)
    
    q3 = qubit()
    r3 = measure(q3)
    
    return r1, r2, r3


def test_tuple(name, func):
    print(f"\nTesting {name}...")
    try:
        print("  Compiling...")
        sim = guppy_sim(func, max_qubits=10).build()
        print("  Running...")
        results = sim.run(2)
        print(f"  Success! Results: {results}")
        return True
    except Exception as e:
        print(f"  Failed: {e}")
        import traceback
        traceback.print_exc()
        return False


if __name__ == "__main__":
    # Test 3-tuple first
    test_tuple("3-tuple", minimal_3tuple)
    
    # Test 4-tuple  
    test_tuple("4-tuple", minimal_4tuple)