#!/usr/bin/env python3
"""Further narrow down the segfault source."""

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
from guppylang.std.quantum import qubit, measure, h, x, y, z
from pecos.frontends import guppy_sim


def test_multiple_measurements():
    """Test returning multiple measurement results."""
    @guppy
    def test() -> tuple[bool, bool]:
        q1 = qubit()
        x(q1)
        r1 = measure(q1)
        
        q2 = qubit()
        y(q2)
        r2 = measure(q2)
        
        return r1, r2
    
    results = guppy_sim(test, max_qubits=10).run(10)
    print("Test passed: multiple measurements")


def test_three_measurements():
    """Test returning three measurement results."""
    @guppy
    def test() -> tuple[bool, bool, bool]:
        q1 = qubit()
        x(q1)
        r1 = measure(q1)
        
        q2 = qubit()
        y(q2)
        r2 = measure(q2)
        
        q3 = qubit()
        z(q3)
        r3 = measure(q3)
        
        return r1, r2, r3
    
    results = guppy_sim(test, max_qubits=10).run(10)
    print("Test passed: three measurements")


def test_four_measurements():
    """Test returning four measurement results."""
    @guppy
    def test() -> tuple[bool, bool, bool, bool]:
        q1 = qubit()
        x(q1)
        r1 = measure(q1)
        
        q2 = qubit()
        y(q2)
        r2 = measure(q2)
        
        q3 = qubit()
        z(q3)
        r3 = measure(q3)
        
        q4 = qubit()
        x(q4)
        r4 = measure(q4)
        
        return r1, r2, r3, r4
    
    results = guppy_sim(test, max_qubits=10).run(10)
    print("Test passed: four measurements")


def test_four_with_extra_gates():
    """Test the exact sequence from comprehensive test."""
    @guppy
    def test() -> tuple[bool, bool, bool, bool]:
        q1 = qubit()
        h(q1)
        x(q1)
        result1 = measure(q1)
        
        q2 = qubit()
        y(q2)
        result2 = measure(q2)
        
        q3 = qubit()
        z(q3)
        result3 = measure(q3)
        
        q4 = qubit()
        x(q4)
        z(q4)
        result4 = measure(q4)
        
        return result1, result2, result3, result4
    
    results = guppy_sim(test, max_qubits=10).run(10)
    print("Test passed: four measurements with extra gates")


if __name__ == "__main__":
    print("Testing narrowed cases...")
    
    test_multiple_measurements()
    test_three_measurements()
    test_four_measurements()
    test_four_with_extra_gates()
    
    print("All tests passed!")