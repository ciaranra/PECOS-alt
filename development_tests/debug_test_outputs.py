#!/usr/bin/env python3
"""Debug test outputs."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Test 1 Output ===")
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
print(SlrConverter(prog1).guppy())

print("\n=== Test 2 Output ===")
prog2 = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Measure entire register at once
    Measure(q) > c,
)
print(SlrConverter(prog2).guppy())

print("\n=== Test 3 Output ===")
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
print(SlrConverter(prog3).guppy())