#!/usr/bin/env python3
"""Test different tuple sizes with static functions."""

import sys
sys.path.append("python/quantum-pecos/src")

from guppylang import guppy
from guppylang.std.quantum import qubit, measure, x
from pecos.frontends import guppy_sim


@guppy
def test_1_tuple() -> bool:
    q = qubit()
    x(q)
    return measure(q)


@guppy
def test_2_tuple() -> tuple[bool, bool]:
    q1 = qubit()
    x(q1)
    r1 = measure(q1)
    
    q2 = qubit()
    r2 = measure(q2)
    
    return r1, r2


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
    
    return r1, r2, r3


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
    
    return r1, r2, r3, r4


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
    
    return r1, r2, r3, r4, r5


def test_function(name: str, func):
    """Test a function and report results."""
    print(f"\nTesting {name}...")
    try:
        results = guppy_sim(func, max_qubits=10).run(5)
        print(f"  Success! Results: {results['_result']}")
        return True
    except Exception as e:
        print(f"  Failed with error: {e}")
        return False


if __name__ == "__main__":
    print("Testing different tuple sizes with static functions...")
    
    tests = [
        ("1-tuple (bool)", test_1_tuple),
        ("2-tuple", test_2_tuple),
        ("3-tuple", test_3_tuple),
        ("4-tuple", test_4_tuple),
        ("5-tuple", test_5_tuple),
    ]
    
    for name, func in tests:
        success = test_function(name, func)
        if not success:
            print(f"\nFailed at {name}")
            break
    else:
        print("\nAll sizes tested successfully!")