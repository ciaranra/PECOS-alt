#!/usr/bin/env python3
"""Test different tuple sizes with static functions."""

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
from guppylang.std.quantum import measure, qubit, x
from pecos.frontends.guppy_api import sim
from pecos_rslib import state_vector


@guppy
def circuit_1_tuple() -> bool:
    q = qubit()
    x(q)
    return measure(q)


@guppy
def circuit_2_tuple() -> tuple[bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)

    q2 = qubit()
    r2 = measure(q2)

    return r1, r2


@guppy
def circuit_3_tuple() -> tuple[bool, bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)

    q2 = qubit()
    r2 = measure(q2)

    q3 = qubit()
    x(q3)
    r3 = measure(q3)

    return r1, r2, r3


@guppy
def circuit_4_tuple() -> tuple[bool, bool, bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)

    q2 = qubit()
    r2 = measure(q2)

    q3 = qubit()
    x(q3)
    r3 = measure(q3)

    q4 = qubit()
    r4 = measure(q4)

    return r1, r2, r3, r4


@guppy
def circuit_5_tuple() -> tuple[bool, bool, bool, bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)

    q2 = qubit()
    r2 = measure(q2)

    q3 = qubit()
    x(q3)
    r3 = measure(q3)

    q4 = qubit()
    r4 = measure(q4)

    q5 = qubit()
    x(q5)
    r5 = measure(q5)

    return r1, r2, r3, r4, r5


def run_function_test(name: str, func) -> bool | None:
    """Helper to test a function and report results."""
    print(f"\nTesting {name}...")
    try:
        results = sim(func).qubits(10).quantum(state_vector()).run(5)
        print(
            f"  Success! Results: {results.get("measurements", results.get("measurement_1", []))}",
        )
        return True
    except Exception as e:
        print(f"  Failed with error: {e}")
        return False


def test_1_tuple_return() -> None:
    """Test that 1-tuple (bool) returns work correctly."""
    assert run_function_test("1-tuple (bool)", circuit_1_tuple)


def test_2_tuple_return() -> None:
    """Test that 2-tuple returns work correctly."""
    assert run_function_test("2-tuple", circuit_2_tuple)


def test_3_tuple_return() -> None:
    """Test that 3-tuple returns work correctly."""
    assert run_function_test("3-tuple", circuit_3_tuple)


def test_4_tuple_return() -> None:
    """Test that 4-tuple returns work correctly."""
    assert run_function_test("4-tuple", circuit_4_tuple)


def test_5_tuple_return() -> None:
    """Test that 5-tuple returns work correctly."""
    assert run_function_test("5-tuple", circuit_5_tuple)


if __name__ == "__main__":
    print("Testing different tuple sizes with static functions...")

    tests = [
        ("1-tuple (bool)", circuit_1_tuple),
        ("2-tuple", circuit_2_tuple),
        ("3-tuple", circuit_3_tuple),
        ("4-tuple", circuit_4_tuple),
        ("5-tuple", circuit_5_tuple),
    ]

    for name, func in tests:
        success = run_function_test(name, func)
        if not success:
            print(f"\nFailed at {name}")
            break
    else:
        print("\nAll sizes tested successfully!")
