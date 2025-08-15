#!/usr/bin/env python3
"""Test array unpacking in functions."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing Array Unpacking in Functions ===\n")

# Test: Function that needs to unpack for measurements
print("Test: Function with individual measurements")

class MeasureIndividual(Block):
    """Measure individual qubits from an array."""
    def __init__(self, qubits, results):
        super().__init__()
        self.qubits = qubits
        self.results = results
        self.ops = [
            # Individual measurements that need unpacking
            Measure(qubits[0]) > results[0],
            Measure(qubits[1]) > results[1],
            Measure(qubits[2]) > results[2],
        ]

prog = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    
    # Call function that measures individually
    MeasureIndividual(q, c),
    
    # Main can do a full array measure after
    q2 := QReg("q2", 3),
    c2 := CReg("c2", 3),
    Measure(q2) > c2,
)

print("Generated Guppy:")
guppy_code = SlrConverter(prog).guppy()
print(guppy_code)

# Test compilation
print("\n" + "="*50 + "\n")
print("Testing HUGR compilation...")
try:
    hugr = SlrConverter(prog).hugr()
    print("✓ Successfully compiled to HUGR")
except Exception as e:
    print(f"✗ Compilation failed: {e}")