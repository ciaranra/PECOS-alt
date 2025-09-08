#!/usr/bin/env python3
"""Minimal test to reproduce 4-tuple segfault."""

import sys


def decode_integer_results(results: list[int], n_bits: int) -> list[tuple[bool, ...]]:
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
from guppylang.std.quantum import measure, qubit
from pecos.frontends.guppy_api import sim
from pecos_rslib import state_vector


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


def run_tuple_test(name, func) -> bool | None:
    """Helper function to test a tuple-returning function."""
    print(f"\nTesting {name}...")
    try:
        print("  Compiling...")
        sim = sim(func).qubits(10).quantum(state_vector()).build()
        print("  Running...")
        results = sim.run(2)
        print(f"  Success! Results: {results}")
        return True
    except Exception as e:
        print(f"  Failed: {e}")
        import traceback

        traceback.print_exc()
        return False


def test_3tuple() -> None:
    """Test that 3-tuple returns work correctly."""
    assert run_tuple_test("3-tuple", minimal_3tuple)


def test_4tuple() -> None:
    """Test that 4-tuple returns work correctly."""
    assert run_tuple_test("4-tuple", minimal_4tuple)


if __name__ == "__main__":
    # Test 3-tuple first
    run_tuple_test("3-tuple", minimal_3tuple)

    # Test 4-tuple
    run_tuple_test("4-tuple", minimal_4tuple)
