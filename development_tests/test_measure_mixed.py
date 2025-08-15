#!/usr/bin/env python3
"""Test mixed measurement destinations."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing Mixed Measurement Destinations ===\n")

# Test 1: Measure to different CRegs
print("Test 1: Consecutive measurements to different CRegs")
prog1 = Main(
    q := QReg("q", 4),
    c1 := CReg("c1", 2),
    c2 := CReg("c2", 2),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Measure all qubits but to different destinations
    Measure(q[0]) > c1[0],
    Measure(q[1]) > c1[1],
    Measure(q[2]) > c2[0],
    Measure(q[3]) > c2[1],
)

print("Generated code:")
print(SlrConverter(prog1).guppy())

print("\n" + "="*50 + "\n")

# Test 2: Measure to non-contiguous indices
print("Test 2: Consecutive measurements to non-contiguous CReg indices")
prog2 = Main(
    q := QReg("q", 3),
    c := CReg("c", 5),
    qubit.H(q[0]),
    # Measure all qubits to non-contiguous positions
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[2],  # Skip c[1]
    Measure(q[2]) > c[4],  # Skip c[3]
)

print("Generated code:")
print(SlrConverter(prog2).guppy())

print("\n" + "="*50 + "\n")

# Test 3: Measure to mismatched size CReg
print("Test 3: Consecutive measurements to larger CReg")
prog3 = Main(
    q := QReg("q", 3),
    c := CReg("c", 5),  # Larger than q
    qubit.H(q[0]),
    # Measure all qubits to first 3 positions of c
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
    # c[3] and c[4] remain unmeasured
)

print("Generated code:")
print(SlrConverter(prog3).guppy())