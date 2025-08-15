#!/usr/bin/env python3
"""Debug function measurement analysis."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Simple test
class MeasureQubits(Block):
    def __init__(self, q, c):
        super().__init__()
        self.q = q
        self.c = c
        self.ops = [
            Measure(q[0]) > c[0],
            Measure(q[1]) > c[1],
        ]

prog = Main(
    q := QReg("q", 2),
    c := CReg("c", 2),
    MeasureQubits(q, c),
)

# Check what the converter does
print("Generating Guppy...")
guppy_code = SlrConverter(prog).guppy()
print(guppy_code)

# Check measurement info
print("\nDebug: Checking measurement analysis...")
# Let's manually analyze the MeasureQubits block
from pecos.slr.gen_codes.guppy.measurement_analyzer import MeasurementAnalyzer
analyzer = MeasurementAnalyzer()
block = prog.ops[0]  # MeasureQubits block
print(f"Block type: {type(block).__name__}")
print(f"Block has ops: {hasattr(block, 'ops')}")
if hasattr(block, 'ops'):
    print(f"Number of ops: {len(block.ops)}")
    for i, op in enumerate(block.ops):
        print(f"  Op {i}: {type(op).__name__}")