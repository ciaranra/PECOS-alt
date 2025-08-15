#!/usr/bin/env python3
"""Test the correct syntax for measuring arrays."""

from guppylang import guppy
from guppylang.std import quantum
from guppylang.std.builtins import array, owned, result

# Test which syntax works for measuring arrays
code1 = '''
@guppy
def test_measure() -> None:
    q = array(quantum.qubit() for _ in range(3))
    c = quantum.measure(q)  # Does this work?
    result("c", c)
'''

code2 = '''
@guppy
def test_measure_array() -> None:
    q = array(quantum.qubit() for _ in range(3))
    c = quantum.measure_array(q)  # Or this?
    result("c", c)
'''

print("Testing quantum.measure(array)...")
try:
    exec(code1)
    print("✓ quantum.measure(array) works")
except Exception as e:
    print(f"✗ quantum.measure(array) failed: {type(e).__name__}")

print("\nTesting quantum.measure_array(array)...")
try:
    exec(code2)
    print("✓ quantum.measure_array(array) works")
except Exception as e:
    print(f"✗ quantum.measure_array(array) failed: {type(e).__name__}")