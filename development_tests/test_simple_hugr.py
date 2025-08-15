#!/usr/bin/env python3
"""Test HUGR compilation with a simple example that satisfies linearity."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Simple program that creates qubits and measures them
prog = Main(
    q := QReg("q", 2),
    c := CReg("c", 2),
    # Apply some gates
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Measure everything (required for linearity)
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
)

print("=== Simple HUGR Test ===")
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