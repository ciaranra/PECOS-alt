#!/usr/bin/env python3
"""Demonstrate code generation strategy for different SLR patterns."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Code Generation Strategy Demo ===\n")

# Case 1: All qubits measured at once
print("CASE 1: Measuring entire QReg at once")
prog1 = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    qubit.CX(q[1], q[2]),
    # All measurements at the end
    Measure(q[0]) > c[0],
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
)

print("SLR detects: All qubits measured together")
print("Strategy: Use measure_array()")
print("Generated code should be:")
print("""
    q = array(quantum.qubit() for _ in range(3))
    quantum.h(q[0])
    quantum.cx(q[0], q[1])
    quantum.cx(q[1], q[2])
    c = quantum.measure_array(q)  # Efficient!
    result("c", c)
""")

# Case 2: Selective/staged measurements
print("\nCASE 2: Selective/staged measurements")
prog2 = Main(
    q := QReg("q", 5),
    c := CReg("c", 5),
    # First stage
    qubit.H(q[0]),
    Measure(q[0]) > c[0],  # Early measurement
    # Continue with others
    qubit.CX(q[1], q[2]),
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
    # Later measurements
    Measure(q[3]) > c[3],
    Measure(q[4]) > c[4],
)

print("SLR detects: Measurements at different stages")
print("Strategy: Unpack array before first measurement")
print("Generated code should be:")
print("""
    q = array(quantum.qubit() for _ in range(5))
    quantum.h(q[0])
    
    # First measurement detected - unpack array
    q0, q1, q2, q3, q4 = q
    c0 = quantum.measure(q0)
    
    # Continue with unpacked qubits
    quantum.cx(q1, q2)
    c1 = quantum.measure(q1)
    c2 = quantum.measure(q2)
    c3 = quantum.measure(q3)
    c4 = quantum.measure(q4)
    
    c = array(c0, c1, c2, c3, c4)
    result("c", c)
""")

# Case 3: QEC pattern with ancilla reuse
print("\nCASE 3: QEC pattern - measure and reallocate")
print("In QEC, we often:")
print("- Measure ancilla qubits")
print("- Allocate fresh ones for next round")
print("- Keep data qubits unmeasured")
print("\nThe generator should:")
print("1. Detect ancilla measurement patterns")
print("2. Not require repacking measured qubits")
print("3. Allow fresh allocation as needed")

# Summary of generation rules
print("\n=== Code Generation Rules ===")
print("""
1. DETECTION PHASE:
   - Scan all operations on a QReg
   - If ALL qubits measured at end → use measure_array()
   - If selective/staged → prepare for unpacking

2. TRANSFORMATION:
   - Gate operations: Keep subscripting (works fine)
   - First measurement: Insert unpacking before it
   - Subsequent ops: Use unpacked variables

3. MEASUREMENT HANDLING:
   - Full array: c = measure_array(q)
   - Selective: c0 = measure(q0), c1 = measure(q1), ...
   - Results: Pack into array at end if needed

4. SPECIAL CASES:
   - Fresh allocation after measurement is fine
   - No need to "repack" consumed qubits
   - Each QReg handled independently
""")

print("\nThis strategy preserves natural SLR code while satisfying Guppy's linearity!")

# Generate actual code to show current vs desired
print("\n=== Example: Current vs Desired Generation ===")
print("Current (broken):")
print(SlrConverter(prog2).guppy())

print("\nDesired (would work with HUGR):")
print("(Would implement the unpacking strategy shown above)")