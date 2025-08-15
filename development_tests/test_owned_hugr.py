#!/usr/bin/env python3
"""Test HUGR compilation with @owned parameters."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Even simpler - single qubit register
prog = Main(
    q := QReg("q", 1),
    c := CReg("c", 1),
    qubit.H(q[0]),
    Measure(q[0]) > c[0],
)

print("=== Single Qubit HUGR Test ===")
print("Generated Guppy code:")
guppy_code = SlrConverter(prog).guppy()
print(guppy_code)

print("\n=== Attempting HUGR compilation ===")
try:
    hugr = SlrConverter(prog).hugr()
    print("HUGR compilation successful!")
    print(f"HUGR type: {type(hugr)}")
except Exception as e:
    print(f"HUGR compilation failed: {e}")