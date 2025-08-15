#!/usr/bin/env python3
"""Test selective measurement with HUGR compilation."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing Selective Measurements with HUGR ===\n")

# Create a test with selective measurements (like Case 2)
prog = Main(
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

print("1. Generated Guppy code:")
print("-" * 50)
guppy_code = SlrConverter(prog).guppy()
print(guppy_code)

print("\n2. Attempting HUGR compilation:")
print("-" * 50)
try:
    hugr = SlrConverter(prog).hugr()
    print("✓ Successfully compiled to HUGR!")
    print(f"HUGR type: {type(hugr)}")
    print(f"HUGR representation: {hugr}")
except Exception as e:
    print(f"✗ HUGR compilation failed: {e}")
    import traceback
    traceback.print_exc()

print("\n3. Analyzing generated code:")
print("-" * 50)

# Check if arrays are unpacked
if "# Unpack" in guppy_code:
    print("✓ Arrays are being unpacked for measurements")
else:
    print("✗ No array unpacking detected")

# Check if measure_array is used
if "measure_array" in guppy_code:
    print("✓ Using measure_array() for efficient full-array measurements")
else:
    print("• No measure_array() usage (may be using selective measurements)")

# Check if results are packed
if "# Pack measurement results" in guppy_code:
    print("✓ Measurement results are being packed into arrays")
else:
    print("• No measurement result packing detected")

# Check function generation
import re
functions = re.findall(r'@guppy\ndef (\w+)', guppy_code)
print(f"\nGenerated functions: {functions}")

print("\nDone!")