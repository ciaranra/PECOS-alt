"""Minimal pytest test to reproduce segfault"""

import sys
sys.path.append("python/quantum-pecos/src")

import pytest
from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.frontends.run_guppy import run_guppy


def test_minimal_hadamard():
    """Minimal test case"""
    @guppy
    def hadamard_test() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    print("\n[TEST] Starting minimal hadamard test")
    result = run_guppy(hadamard_test, shots=50, backend="rust", verbose=True, seed=42)
    print(f"[TEST] Test completed successfully")
    assert "results" in result


def test_minimal_second():
    """Second minimal test"""
    @guppy
    def simple_test() -> bool:
        q = qubit()
        return measure(q)
    
    print("\n[TEST] Starting second test")
    result = run_guppy(simple_test, shots=50, backend="rust", verbose=True, seed=42)
    print(f"[TEST] Second test completed")
    assert "results" in result