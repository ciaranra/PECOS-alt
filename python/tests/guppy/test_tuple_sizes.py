#!/usr/bin/env python3
"""Test different tuple sizes to find segfault threshold."""

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
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim


# Pre-define functions for each tuple size instead of using exec
@guppy
def test_1_tuple() -> tuple[bool]:
    q = qubit()
    x(q)
    return (measure(q),)

@guppy
def test_2_tuple() -> tuple[bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)
    q2 = qubit()
    r2 = measure(q2)
    return (r1, r2)

@guppy
def test_3_tuple() -> tuple[bool, bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)
    q2 = qubit()
    r2 = measure(q2)
    q3 = qubit()
    x(q3)
    r3 = measure(q3)
    return (r1, r2, r3)

@guppy
def test_4_tuple() -> tuple[bool, bool, bool, bool]:
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
    return (r1, r2, r3, r4)

@guppy
def test_5_tuple() -> tuple[bool, bool, bool, bool, bool]:
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
    return (r1, r2, r3, r4, r5)

@guppy
def test_6_tuple() -> tuple[bool, bool, bool, bool, bool, bool]:
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
    q6 = qubit()
    r6 = measure(q6)
    return (r1, r2, r3, r4, r5, r6)

@guppy
def test_7_tuple() -> tuple[bool, bool, bool, bool, bool, bool, bool]:
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
    q6 = qubit()
    r6 = measure(q6)
    q7 = qubit()
    x(q7)
    r7 = measure(q7)
    return (r1, r2, r3, r4, r5, r6, r7)

@guppy
def test_8_tuple() -> tuple[bool, bool, bool, bool, bool, bool, bool, bool]:
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
    q6 = qubit()
    r6 = measure(q6)
    q7 = qubit()
    x(q7)
    r7 = measure(q7)
    q8 = qubit()
    r8 = measure(q8)
    return (r1, r2, r3, r4, r5, r6, r7, r8)


def run_tuple_size_test(n: int, test_func):
    """Helper to test returning n-tuple of bools."""
    print(f"\nTesting {n}-tuple of bools...")
    
    try:
        results = guppy_sim(test_func, max_qubits=10).run(5)
        print(f"  Success! Results: {results['result'][:3]}...")
        return True
    except Exception as e:
        print(f"  Failed with error: {e}")
        import traceback
        traceback.print_exc()
        return False


def test_tuple_size_1():
    """Test 1-tuple returns."""
    assert run_tuple_size_test(1, test_1_tuple)


def test_tuple_size_2():
    """Test 2-tuple returns."""
    assert run_tuple_size_test(2, test_2_tuple)


def test_tuple_size_3():
    """Test 3-tuple returns."""
    assert run_tuple_size_test(3, test_3_tuple)


def test_tuple_size_4():
    """Test 4-tuple returns."""
    assert run_tuple_size_test(4, test_4_tuple)


def test_tuple_size_5():
    """Test 5-tuple returns."""
    assert run_tuple_size_test(5, test_5_tuple)


def test_tuple_size_6():
    """Test 6-tuple returns."""
    assert run_tuple_size_test(6, test_6_tuple)


def test_tuple_size_7():
    """Test 7-tuple returns."""
    assert run_tuple_size_test(7, test_7_tuple)


def test_tuple_size_8():
    """Test 8-tuple returns."""
    assert run_tuple_size_test(8, test_8_tuple)


if __name__ == "__main__":
    print("Testing different tuple sizes...")
    
    # Map of tuple sizes to test functions
    test_functions = {
        1: test_1_tuple,
        2: test_2_tuple,
        3: test_3_tuple,
        4: test_4_tuple,
        5: test_5_tuple,
        6: test_6_tuple,
        7: test_7_tuple,
        8: test_8_tuple,
    }
    
    # Test progressively larger tuples
    for size in [1, 2, 3, 4, 5, 6, 7, 8]:
        success = run_tuple_size_test(size, test_functions[size])
        if not success:
            print(f"\nFailed at tuple size {size}")
            break
    else:
        print("\nAll sizes tested successfully!")