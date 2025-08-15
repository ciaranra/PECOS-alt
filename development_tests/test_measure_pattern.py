#!/usr/bin/env python3
"""Test different measurement patterns to see if they use measure_array."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing Measurement Pattern Detection ===\n")

# Test 1: Individual consecutive measurements of all qubits
print("Test 1: Individual measurements of all qubits (consecutive)")
prog1 = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Measure all qubits individually but consecutively
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
    Measure(q[3]) > c[3],
)

guppy1 = SlrConverter(prog1).guppy()
print("Generated code uses measure_array?", "measure_array" in guppy1)
print()

# Test 2: Single measurement of entire register
print("Test 2: Single measurement of entire register")
prog2 = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Measure entire register at once
    Measure(q) > c,
)

guppy2 = SlrConverter(prog2).guppy()
print("Generated code uses measure_array?", "measure_array" in guppy2)
print()

# Test 3: Mixed operations between measurements
print("Test 3: Operations between measurements")
prog3 = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    qubit.H(q[0]),
    Measure(q[0]) > c[0],
    qubit.CX(q[1], q[2]),  # Operation between measurements
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
    Measure(q[3]) > c[3],
)

guppy3 = SlrConverter(prog3).guppy()
print("Generated code uses measure_array?", "measure_array" in guppy3)
print()

print("Analysis:")
print("-" * 50)
print("Test 1 should use measure_array (all qubits measured consecutively)")
print("Test 2 should use measure_array (single register measurement)")
print("Test 3 should NOT use measure_array (operations between measurements)")