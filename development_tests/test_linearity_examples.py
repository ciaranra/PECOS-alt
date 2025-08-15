#!/usr/bin/env python3
"""Test SLR examples that challenge Guppy's linearity requirements."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing Linearity Challenges ===\n")

# Example 1: Function that modifies qubits but doesn't consume them
print("Example 1: PrepareGHZ block that modifies but doesn't consume qubits")

class PrepareGHZ(Block):
    """Prepare a GHZ state - modifies qubits but doesn't measure them."""
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
    # Problem: PrepareGHZ modifies q but doesn't consume it
    # We need the function to return q
    Measure(q) > c,
)

print("SLR Program:")
print("PrepareGHZ(q) followed by Measure(q)")
print("\nGenerated Guppy (current):")
try:
    print(SlrConverter(prog1).guppy())
except Exception as e:
    print(f"Error: {e}")

print("\n" + "="*50 + "\n")

# Example 2: Main function that doesn't measure all qubits
print("Example 2: Main function with unmeasured qubits")

prog2 = Main(
    q := QReg("q", 5),
    c := CReg("c", 2),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    # Only measure first 2 qubits
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
    # Problem: q[2], q[3], q[4] are never measured
)

print("SLR Program:")
print("5 qubits allocated, only 2 measured")
print("\nGenerated Guppy (current):")
print(SlrConverter(prog2).guppy())

print("\n" + "="*50 + "\n")

# Example 3: Nested function calls with quantum resources
print("Example 3: Nested functions with quantum resources")

class InitializePair(Block):
    """Initialize a qubit pair."""
    def __init__(self, q):
        super().__init__()
        self.q = q
        self.ops = [
            qubit.H(q[0]),
            qubit.H(q[1]),
        ]

class EntanglePair(Block):
    """Entangle a qubit pair."""
    def __init__(self, q):
        super().__init__()
        self.q = q
        self.ops = [
            qubit.CX(q[0], q[1]),
        ]

prog3 = Main(
    q := QReg("q", 2),
    c := CReg("c", 2),
    InitializePair(q),
    EntanglePair(q),
    # These functions modify q but need to pass it through
    Measure(q) > c,
)

print("SLR Program:")
print("InitializePair(q), EntanglePair(q), then Measure(q)")
print("\nGenerated Guppy (current):")
print(SlrConverter(prog3).guppy())

print("\n" + "="*50 + "\n")

# Example 4: Function that consumes some qubits but not others
print("Example 4: Partial consumption of quantum resources")

class MeasureFirst(Block):
    """Measure only the first qubit of a register."""
    def __init__(self, q, c):
        super().__init__()
        self.q = q
        self.c = c
        self.ops = [
            Measure(q[0]) > c[0],
        ]

prog4 = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    qubit.H(q[0]),
    qubit.H(q[1]),
    qubit.H(q[2]),
    MeasureFirst(q, c),
    # Problem: MeasureFirst consumes q[0] but q[1], q[2] still alive
    # Need to handle remaining qubits
    qubit.X(q[1]),  # Can we still use q[1]?
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
)

print("SLR Program:")
print("MeasureFirst(q, c) consumes q[0], but q[1] and q[2] still used")
print("\nGenerated Guppy (current):")
try:
    print(SlrConverter(prog4).guppy())
except Exception as e:
    print(f"Error: {e}")

print("\nAnalysis:")
print("-" * 50)
print("Key issues to address:")
print("1. Functions that modify arrays need @owned parameters")
print("2. Functions must return unconsumed quantum resources")
print("3. Main must consume all allocated qubits (measure or discard)")
print("4. Partial consumption needs careful resource tracking")