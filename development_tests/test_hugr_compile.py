#!/usr/bin/env python3
"""Test that our generated code compiles to HUGR."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing HUGR Compilation ===\n")

# Test 1: Simple case with @owned and returns
class PrepareGHZ(Block):
    """Prepare a GHZ state."""
    def __init__(self, q):
        super().__init__()
        self.q = q
        self.ops = [
            qubit.H(q[0]),
            qubit.CX(q[0], q[1]),
            qubit.CX(q[1], q[2]),
        ]

prog1 = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    PrepareGHZ(q),
    Measure(q) > c,
)

print("Test 1: PrepareGHZ with proper resource handling")
try:
    hugr = SlrConverter(prog1).hugr()
    print("✓ Successfully compiled to HUGR!")
except Exception as e:
    print(f"✗ Failed: {e}")

print("\n" + "="*50 + "\n")

# Test 2: Unmeasured qubits with cleanup
prog2 = Main(
    q := QReg("q", 5),
    c := CReg("c", 2),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
)

print("Test 2: Partial measurements with automatic cleanup")
try:
    hugr = SlrConverter(prog2).hugr()
    print("✓ Successfully compiled to HUGR!")
except Exception as e:
    print(f"✗ Failed: {e}")

print("\n" + "="*50 + "\n")

# Test 3: Complex measurement pattern
prog3 = Main(
    q := QReg("q", 4),
    c1 := CReg("c1", 2),
    c2 := CReg("c2", 2),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # All measured but to different destinations
    Measure(q[0]) > c1[0],
    Measure(q[1]) > c1[1],
    Measure(q[2]) > c2[0],
    Measure(q[3]) > c2[1],
)

print("Test 3: Complex measurement destinations")
try:
    hugr = SlrConverter(prog3).hugr()
    print("✓ Successfully compiled to HUGR!")
except Exception as e:
    print(f"✗ Failed: {e}")